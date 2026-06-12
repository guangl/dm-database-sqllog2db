use super::error_log::write_error_log;
use super::filter_processor::build_pipeline;
use super::input::{make_progress_bar, merge_trxid_prescan, resolve_input_files};
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
    if verbose {
        eprintln!(
            "Processing {} files in parallel ({} jobs)",
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

async fn run_sqlite_parallel(
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
    )
    .await?;
    Ok((files, skipped, stats))
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
        return run_sqlite_parallel(ctx, log_files, jobs, verbose, interrupted).await;
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
pub async fn handle_run(
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
    let merged = merge_trxid_prescan(cfg, &log_files, jobs, is_stdin_pipe, quiet)?;
    let final_cfg: &Config = merged.as_ref().unwrap_or(cfg);
    let ctx = build_run_context(final_cfg);
    let use_parallel = (jobs > 1 && log_files.len() > 1 && !is_stdin_pipe)
        && (final_cfg.exporter.csv.is_some() || final_cfg.exporter.sqlite.is_some());
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
