use super::driver::chunk::{should_split, split_file_into_chunks};
use super::driver::parallel::process_csv_parallel;
use super::driver::sequential::run_sequential;
use super::driver::sqlite::process_sqlite_parallel;
use super::prepare::{
    DEFAULT_MEMORY_BUDGET_BYTES, effective_jobs_for_memory_budget, make_progress_bar,
    merge_trxid_prescan, resolve_input_files,
};
use super::report::{RunSummary, print_run_summary, write_error_log};
use crate::config::Config;
use crate::error::{Error, ErrorStats, Result};
use crate::pipeline::filters::build_pipeline;
use crate::pipeline::{FIELD_NAMES, FieldMask, NormalizeConfig, OutputConfig, Pipeline};
use indicatif::ProgressBar;
use log::info;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// 运行上下文：配置、pipeline 与归一化/投影选项，贯穿所有执行路径。
pub(super) struct RunContext<'a> {
    pub(super) cfg: &'a Config,
    pub(super) pipeline: Pipeline,
    pub(super) field_mask: FieldMask,
    pub(super) ordered_indices: Vec<usize>,
    pub(super) do_normalize: bool,
    pub(super) placeholder_override: Option<bool>,
}

/// 终端展示环境：安静/详细模式与可选进度条（`show_progress ≡ pb.is_some()`）。
pub(super) struct Console<'a> {
    pub(super) quiet: bool,
    pub(super) verbose: bool,
    pub(super) pb: Option<&'a ProgressBar>,
}

/// 已处理文件及各自的记录数（顺序与输入文件一致）。
pub(super) type FileCounts = Vec<(PathBuf, usize)>;

/// 执行路径的统一产出：`(per-file counts, 跳过文件数, 错误统计)`。
pub(super) type ProcessOutcome = (FileCounts, usize, ErrorStats);

type ProcessResult = Result<ProcessOutcome>;

/// CSV 拆分（`max_rows_per_file`）是否启用。
///
/// 拆分依赖 `CsvExporter` 的单实例状态按行数轮转文件，而并行路径会把每个文件
/// 解析成独立临时 part 再 `concat` 成单一输出，无法保持全局行数边界。因此启用拆分时
/// 必须走顺序流式路径（本就是常数内存），否则 `max_rows_per_file` 会被静默忽略。
fn csv_splitting_enabled(cfg: &Config) -> bool {
    cfg.exporter
        .csv
        .as_ref()
        .is_some_and(|c| c.max_rows_per_file.is_some())
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
    process_csv_parallel(ctx, log_files, jobs, verbose, interrupted)
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
    process_sqlite_parallel(ctx, log_files, interrupted)
}

/// 单文件切块路径的前置条件（不含文件大小判断，供 `run` 提前算出 `use_parallel`/进度条策略）。
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
    // 拆分依赖单实例的行数轮转，chunked 并行会 concat 成单文件，两者不兼容。
    if csv_splitting_enabled(ctx.cfg) {
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
    let result = process_csv_parallel(ctx, &chunks.paths, chunk_jobs, verbose, interrupted);
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
    console: &Console<'_>,
    interrupted: &Arc<AtomicBool>,
) -> ProcessResult {
    if let Some(result) = try_chunked_single_file_csv(
        ctx,
        log_files,
        jobs,
        is_stdin_pipe,
        console.verbose,
        interrupted,
    ) {
        return result;
    }
    let multi_file = jobs > 1 && log_files.len() > 1 && !is_stdin_pipe;
    if multi_file && ctx.cfg.exporter.csv.is_some() && csv_splitting_enabled(ctx.cfg) {
        // 并行 CSV 会 concat 成单文件，无法保持 max_rows_per_file 边界；回退顺序流式路径。
        log::info!("max_rows_per_file is set; exporting sequentially to honor CSV file splitting");
    } else if multi_file && ctx.cfg.exporter.csv.is_some() {
        // NOTE: run_csv_parallel is a synchronous blocking function (it uses block_in_place
        // internally) called directly from this async fn. Ideally this would be wrapped in
        // spawn_blocking, but RunContext holds borrowed references (&Config, &Pipeline) that
        // are not 'static and cannot be moved into a spawn_blocking closure without
        // restructuring the call chain. The current call blocks the async task for the
        // duration of the parallel CSV export. This is acceptable because engine::run is the
        // top-level orchestrator and no other async tasks depend on its progress.
        return run_csv_parallel(ctx, log_files, jobs, console.verbose, interrupted);
    }
    if multi_file && ctx.cfg.exporter.sqlite.is_some() {
        return run_sqlite_parallel(ctx, log_files, jobs, console.verbose, interrupted);
    }
    let (files, stats) = run_sequential(ctx, log_files, console, interrupted).await?;
    Ok((files, 0, stats))
}

/// 主编排函数：解析日志文件并导出到配置的导出器。
/// 并行路径：CSV + 多文件 + jobs > 1；顺序路径：其他情况。
/// `jobs_override` 为测试钩子，生产代码传 None 保持 `available_parallelism` 原行为。
///
/// # Errors
///
/// 未找到任何输入文件、导出器初始化/写出发生致命错误，或运行期间收到中断信号
/// （返回 [`Error::Interrupted`]）时返回错误。
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
            && ((final_cfg.exporter.csv.is_some() && !csv_splitting_enabled(final_cfg))
                || final_cfg.exporter.sqlite.is_some()));
    let show_progress = !quiet && !verbose && !use_parallel;
    let pb = make_progress_bar(show_progress, log_files.len());
    let console = Console {
        quiet,
        verbose,
        pb: pb.as_ref(),
    };
    let (processed_files, skipped_files, stats) =
        route_processing(&ctx, &log_files, jobs, is_stdin_pipe, &console, interrupted).await?;
    run_stats.merge(&stats);
    let total_records: usize = processed_files.iter().map(|(_, c)| *c).sum();
    run_stats.records_exported = total_records;
    if let Some(pb) = &pb {
        pb.finish_and_clear();
    }
    print_run_summary(
        quiet,
        verbose,
        &RunSummary {
            use_parallel,
            elapsed: total_start.elapsed().as_secs_f64(),
            processed_files: &processed_files,
            total_records,
            skipped_files,
        },
        &run_stats,
    );
    write_error_log(final_cfg, &run_stats);
    if interrupted.load(Ordering::Acquire) {
        return Err(Error::Interrupted);
    }
    Ok(run_stats)
}
