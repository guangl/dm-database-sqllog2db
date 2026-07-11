use super::chunk::{should_split, split_file_into_chunks};
use super::error_log::write_error_log;
use crate::pipeline::filters::build_pipeline;
use super::input::{make_progress_bar, merge_trxid_prescan, resolve_input_files};
use super::memory_budget::{DEFAULT_MEMORY_BUDGET_BYTES, effective_jobs_for_memory_budget};
use super::parallel::process_csv_parallel;
use super::sequential::run_sequential;
use super::sqlite_parallel::process_sqlite_parallel;
use super::summary::print_run_summary;
use crate::config::Config;
use crate::error::{Error, ErrorStats, Result};
use crate::pipeline::{FIELD_NAMES, FieldMask, NormalizeConfig, OutputConfig, Pipeline};
use indicatif::ProgressBar;
use log::info;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

struct RunContext<'a> {
    cfg: &'a Config,
    pipeline: Pipeline,
    field_mask: FieldMask,
    ordered_indices: Vec<usize>,
    do_normalize: bool,
    placeholder_override: Option<bool>,
}

fn build_run_context(cfg: &Config) -> RunContext<'_> {
    let pipeline = build_pipeline(cfg);
    let field_mask = cfg
        .output
        .as_ref()
        .map_or(FieldMask::ALL, OutputConfig::field_mask);
    let ordered_indices = cfg.output.as_ref().map_or_else(
        || (0..FIELD_NAMES.len()).collect(),
        OutputConfig::ordered_field_indices,
    );
    let do_normalize = field_mask.includes_normalized_sql()
        && cfg.replace_parameters.as_ref().is_none_or(|r| r.enable);
    let placeholder_override = cfg
        .replace_parameters
        .as_ref()
        .and_then(NormalizeConfig::placeholder_override);
    RunContext {
        cfg,
        pipeline,
        field_mask,
        ordered_indices,
        do_normalize,
        placeholder_override,
    }
}

type ProcessResult = Result<(Vec<(PathBuf, usize)>, usize, ErrorStats)>;

fn run_csv_parallel(
    ctx: &RunContext<'_>,
    log_files: &[PathBuf],
    jobs: usize,
    verbose: bool,
    interrupted: &Arc<AtomicBool>,
) -> ProcessResult {
    let jobs = effective_jobs_for_memory_budget(log_files, jobs, DEFAULT_MEMORY_BUDGET_BYTES);
    if verbose {
        eprintln!(
            "Processing {} files in parallel ({} jobs, memory-budget capped)",
            log_files.len(),
            jobs
        );
    }
    info!("Parsing and exporting SQL logs (parallel, {jobs} jobs)...");
    let (files, skipped, stats) = process_csv_parallel(
        log_files,
        ctx.cfg,
        &ctx.pipeline,
        jobs,
        interrupted,
        ctx.do_normalize,
        ctx.placeholder_override,
        ctx.field_mask,
        &ctx.ordered_indices,
        verbose,
    )?;
    Ok((files, skipped, stats))
}

fn run_sqlite_parallel(
    ctx: &RunContext<'_>,
    log_files: &[PathBuf],
    jobs: usize,
    verbose: bool,
    interrupted: &Arc<AtomicBool>,
) -> ProcessResult {
    if verbose {
        eprintln!(
            "Processing {} files in parallel ({} jobs)",
            log_files.len(),
            jobs
        );
    }
    info!("Parsing and exporting SQL logs (SQLite parallel, {jobs} jobs)...");
    let (files, skipped, stats) = process_sqlite_parallel(
        log_files,
        ctx.cfg,
        &ctx.pipeline,
        jobs,
        interrupted,
        ctx.do_normalize,
        ctx.placeholder_override,
        ctx.field_mask,
        &ctx.ordered_indices,
    )?;
    Ok((files, skipped, stats))
}

/// 单文件切块路径的前置条件（不含文件大小判断，供 `handle_run` 提前算出 `use_parallel`/进度条策略）。
///
/// 安全前提：未启用参数替换（PARAMS 缓存跨记录依赖）且未配置事务级过滤器
/// （`indicators`/`sql`，事务边界判定依赖完整文件预扫描）。条件不满足时返回 `false`，
/// 调用方退回常规（单文件顺序）路径。
fn chunked_single_file_eligible(
    ctx: &RunContext<'_>,
    log_files: &[PathBuf],
    jobs: usize,
    is_stdin_pipe: bool,
) -> bool {
    if jobs <= 1 || log_files.len() != 1 || is_stdin_pipe || ctx.do_normalize {
        return false;
    }
    if ctx.cfg.exporter.csv.is_none() {
        return false;
    }
    let has_transaction_filters = ctx
        .cfg
        .filter
        .as_ref()
        .is_some_and(crate::pipeline::FiltersFeature::has_transaction_filters);
    !has_transaction_filters
}

fn try_chunked_single_file_csv(
    ctx: &RunContext<'_>,
    log_files: &[PathBuf],
    jobs: usize,
    is_stdin_pipe: bool,
    verbose: bool,
    interrupted: &Arc<AtomicBool>,
) -> Option<ProcessResult> {
    if !chunked_single_file_eligible(ctx, log_files, jobs, is_stdin_pipe) {
        return None;
    }
    let file = &log_files[0];
    let size = should_split(file, jobs)?;
    let chunks = match split_file_into_chunks(file, jobs, size) {
        Ok(c) => c,
        Err(e) => {
            log::warn!(
                "Failed to split '{}' into chunks for parallel parsing, \
                 falling back to sequential processing: {e}",
                file.display()
            );
            return None;
        }
    };
    if chunks.paths.len() <= 1 {
        return None;
    }
    if verbose {
        eprintln!(
            "Splitting '{}' into {} chunks for parallel parsing",
            file.display(),
            chunks.paths.len()
        );
    }
    info!(
        "Parsing and exporting SQL logs (single-file chunked parallel, {} chunks)...",
        chunks.paths.len()
    );
    let chunk_jobs = jobs.min(chunks.paths.len());
    let result = process_csv_parallel(
        &chunks.paths,
        ctx.cfg,
        &ctx.pipeline,
        chunk_jobs,
        interrupted,
        ctx.do_normalize,
        ctx.placeholder_override,
        ctx.field_mask,
        &ctx.ordered_indices,
        verbose,
    );
    Some(result.map(|(files, skipped, stats)| {
        let total: usize = files.iter().map(|(_, c)| *c).sum();
        (vec![(file.clone(), total)], skipped, stats)
    }))
}

async fn route_processing(
    ctx: &RunContext<'_>,
    log_files: &[PathBuf],
    jobs: usize,
    is_stdin_pipe: bool,
    verbose: bool,
    quiet: bool,
    pb: Option<&ProgressBar>,
    interrupted: &Arc<AtomicBool>,
) -> ProcessResult {
    if let Some(result) =
        try_chunked_single_file_csv(ctx, log_files, jobs, is_stdin_pipe, verbose, interrupted)
    {
        return result;
    }
    let multi_file = jobs > 1 && log_files.len() > 1 && !is_stdin_pipe;
    if multi_file && ctx.cfg.exporter.csv.is_some() {
        // NOTE: run_csv_parallel is a synchronous blocking function (it uses block_in_place
        // internally) called directly from this async fn. Ideally this would be wrapped in
        // spawn_blocking, but RunContext holds borrowed references (&Config, &Pipeline) that
        // are not 'static and cannot be moved into a spawn_blocking closure without
        // restructuring the call chain. The current call blocks the async task for the
        // duration of the parallel CSV export. This is acceptable because handle_run is the
        // top-level orchestrator and no other async tasks depend on its progress.
        return run_csv_parallel(ctx, log_files, jobs, verbose, interrupted);
    }
    if multi_file && ctx.cfg.exporter.sqlite.is_some() {
        return run_sqlite_parallel(ctx, log_files, jobs, verbose, interrupted);
    }
    let (files, stats) = run_sequential(
        log_files,
        ctx.cfg,
        &ctx.pipeline,
        ctx.do_normalize,
        ctx.placeholder_override,
        verbose,
        quiet,
        pb.is_some(),
        pb,
        interrupted,
    )
    .await?;
    Ok((files, 0, stats))
}

fn finalize_run(
    cfg: &Config,
    run_stats: &mut ErrorStats,
    processed_files: &[(PathBuf, usize)],
    skipped_files: usize,
    pb: Option<&ProgressBar>,
    use_parallel: bool,
    quiet: bool,
    verbose: bool,
    elapsed: f64,
) {
    let total_records: usize = processed_files.iter().map(|(_, c)| *c).sum();
    run_stats.records_exported = total_records;
    if let Some(pb) = pb {
        pb.finish_and_clear();
    }
    print_run_summary(
        quiet,
        verbose,
        use_parallel,
        elapsed,
        processed_files,
        total_records,
        skipped_files,
        run_stats,
    );
    write_error_log(cfg, run_stats);
}

/// 主编排函数：解析日志文件并导出到配置的导出器。
/// 并行路径：CSV + 多文件 + jobs > 1；顺序路径：其他情况。
/// `jobs_override` 为测试钩子，生产代码传 None 保持 `available_parallelism` 原行为。
pub async fn run(
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<AtomicBool>,
    jobs_override: Option<usize>,
) -> Result<ErrorStats> {
    let total_start = Instant::now();
    let mut run_stats = ErrorStats::default();
    let (log_files, is_stdin_pipe) = resolve_input_files(cfg)?;
    let jobs = jobs_override
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, std::num::NonZero::get));
    let prescan_jobs =
        effective_jobs_for_memory_budget(&log_files, jobs, DEFAULT_MEMORY_BUDGET_BYTES);
    let merged = merge_trxid_prescan(cfg, &log_files, prescan_jobs, is_stdin_pipe, quiet)?;
    let final_cfg: &Config = merged.as_ref().unwrap_or(cfg);
    let ctx = build_run_context(final_cfg);
    let will_chunk_single_file =
        chunked_single_file_eligible(&ctx, &log_files, jobs, is_stdin_pipe)
            && log_files
                .first()
                .is_some_and(|f| should_split(f, jobs).is_some());
    let use_parallel = will_chunk_single_file
        || ((jobs > 1 && log_files.len() > 1 && !is_stdin_pipe)
            && (final_cfg.exporter.csv.is_some() || final_cfg.exporter.sqlite.is_some()));
    let show_progress = !quiet && !verbose && !use_parallel;
    let pb = make_progress_bar(show_progress, log_files.len());
    let (processed_files, skipped_files, stats) = route_processing(
        &ctx,
        &log_files,
        jobs,
        is_stdin_pipe,
        verbose,
        quiet,
        pb.as_ref(),
        interrupted,
    )
    .await?;
    run_stats.merge(&stats);
    finalize_run(
        final_cfg,
        &mut run_stats,
        &processed_files,
        skipped_files,
        pb.as_ref(),
        use_parallel,
        quiet,
        verbose,
        total_start.elapsed().as_secs_f64(),
    );
    if interrupted.load(Ordering::Acquire) {
        return Err(Error::Interrupted);
    }
    Ok(run_stats)
}
