use crate::config::Config;
use crate::error::{Error, ErrorStats, Result};
use crate::exporter::ExporterManager;
use crate::parser::SqllogParser;
use log::{info, warn};
use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

mod filter_processor;
mod parallel;
mod prescan;
mod processor;
mod sqlite_parallel;

use filter_processor::build_pipeline;
use indicatif::{ProgressBar, ProgressStyle};
use parallel::process_csv_parallel;
use prescan::scan_for_trxids_by_transaction_filters;
use processor::process_log_file;
use sqlite_parallel::process_sqlite_parallel;

/// 主编排函数：解析日志文件并导出到配置的导出器。
/// 并行路径：CSV + 多文件 + jobs > 1；顺序路径：其他情况。
pub fn handle_run(
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<AtomicBool>,
) -> Result<ErrorStats> {
    let total_start = Instant::now();
    let mut run_stats = ErrorStats::default();
    let (log_files, is_stdin_pipe) = resolve_input_files(cfg)?;
    let jobs = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let merged = merge_trxid_prescan(cfg, &log_files, jobs, is_stdin_pipe)?;
    let final_cfg: &Config = merged.as_ref().unwrap_or(cfg);
    let pipeline = build_pipeline(final_cfg);
    let field_mask = final_cfg.output.as_ref().map_or(
        crate::pipeline::FieldMask::ALL,
        crate::pipeline::OutputConfig::field_mask,
    );
    let ordered_indices = final_cfg.output.as_ref().map_or_else(
        || (0..crate::pipeline::FIELD_NAMES.len()).collect(),
        crate::pipeline::OutputConfig::ordered_field_indices,
    );
    let do_normalize = field_mask.includes_normalized_sql()
        && final_cfg
            .replace_parameters
            .as_ref()
            .is_none_or(|r| r.enable);
    let placeholder_override = final_cfg
        .replace_parameters
        .as_ref()
        .and_then(crate::pipeline::NormalizeConfig::placeholder_override);
    let show_progress = !quiet && !verbose;
    let pb = make_progress_bar(show_progress);
    let mut skipped_files = 0usize;
    let use_csv_parallel =
        jobs > 1 && log_files.len() > 1 && !is_stdin_pipe && final_cfg.exporter.csv.is_some();
    let use_sqlite_parallel =
        jobs > 1 && log_files.len() > 1 && !is_stdin_pipe && final_cfg.exporter.sqlite.is_some();
    let use_parallel = use_csv_parallel || use_sqlite_parallel;
    let processed_files: Vec<(PathBuf, usize)> = if use_csv_parallel {
        let (files, skipped, stats) = run_csv_parallel(
            &log_files,
            final_cfg,
            &pipeline,
            jobs,
            show_progress,
            interrupted,
            do_normalize,
            placeholder_override,
            field_mask,
            &ordered_indices,
            verbose,
        )?;
        run_stats.merge(&stats);
        skipped_files = skipped;
        files
    } else if use_sqlite_parallel {
        let (files, skipped, stats) = run_sqlite_parallel(
            &log_files,
            final_cfg,
            &pipeline,
            jobs,
            show_progress,
            interrupted,
            do_normalize,
            placeholder_override,
            field_mask,
            &ordered_indices,
            verbose,
        )?;
        run_stats.merge(&stats);
        skipped_files = skipped;
        files
    } else {
        let (files, seq_stats) = run_sequential(
            &log_files,
            final_cfg,
            &pipeline,
            do_normalize,
            placeholder_override,
            verbose,
            quiet,
            show_progress,
            pb.as_ref(),
            interrupted,
        )?;
        run_stats.merge(&seq_stats);
        files
    };
    let total_records: usize = processed_files.iter().map(|(_, c)| *c).sum();
    print_run_summary(
        quiet,
        verbose,
        use_parallel,
        total_start.elapsed().as_secs_f64(),
        &processed_files,
        total_records,
        skipped_files,
        &run_stats,
    );
    if let Some(pb) = &pb {
        pb.finish_and_clear();
    }
    if interrupted.load(Ordering::Relaxed) {
        return Err(Error::Interrupted);
    }
    Ok(run_stats)
}

/// 解析输入文件列表并检测 stdin pipe 模式。
/// 返回 `(log_files, is_stdin_pipe)`。当无文件且非 Unix stdin pipe 时返回错误。
fn resolve_input_files(cfg: &Config) -> Result<(Vec<PathBuf>, bool)> {
    let log_files = SqllogParser::new(cfg.sqllog.inputs.clone()).log_files()?;
    // Stdin pipe mode: fall back when no log files found AND stdin is not a terminal.
    // /dev/stdin is Unix-only; skip pipe mode on Windows.
    #[cfg(target_os = "windows")]
    let is_stdin_pipe = false;
    #[cfg(not(target_os = "windows"))]
    let is_stdin_pipe = log_files.is_empty() && !std::io::stdin().is_terminal();
    let log_files = if is_stdin_pipe {
        info!("No log files found, reading from stdin (pipe mode)");
        vec![PathBuf::from("/dev/stdin")]
    } else if log_files.is_empty() {
        // On Windows, if stdin is piped but no files found, warn the user that stdin
        // pipe mode is not supported on this platform.
        #[cfg(target_os = "windows")]
        if !std::io::stdin().is_terminal() {
            warn!("Stdin pipe mode is not supported on Windows. No log files found.");
        }
        return Err(crate::error::Error::Parser(
            crate::error::ParserError::NoFilesFound {
                inputs: cfg.sqllog.inputs.clone(),
            },
        ));
    } else {
        log_files
    };
    Ok((log_files, is_stdin_pipe))
}

/// 在有事务级过滤器时执行预扫描并合并 trxid，返回合并后的 Config。
/// `None` = 无需预扫描（无事务过滤器，或 stdin pipe 降级）；`Some` = 已合并 trxid 的新 Config。
fn merge_trxid_prescan(
    cfg: &Config,
    log_files: &[PathBuf],
    jobs: usize,
    is_stdin_pipe: bool,
) -> Result<Option<Config>> {
    if cfg
        .filter
        .as_ref()
        .is_some_and(crate::pipeline::FiltersFeature::has_transaction_filters)
    {
        if is_stdin_pipe {
            warn!(
                "Transaction-level filters are configured but stdin pipe mode \
                 cannot pre-scan for transaction IDs. Degrading to per-record matching \
                 (transaction integrity not guaranteed)."
            );
            eprintln!(
                "[WARN] Transaction-level filters with stdin: pre-scan disabled, \
                 degrading to per-record matching."
            );
            return Ok(None);
        }
        let extra_trxids = scan_for_trxids_by_transaction_filters(log_files, cfg, jobs)?;
        let mut tmp = cfg.clone();
        if let Some(f) = &mut tmp.filter {
            f.merge_found_trxids(extra_trxids);
        }
        Ok(Some(tmp))
    } else {
        Ok(None)
    }
}

/// 创建进度条（spinner 样式），`show_progress` 为 false 时返回 `None`。
fn make_progress_bar(show_progress: bool) -> Option<ProgressBar> {
    if show_progress {
        let bar = ProgressBar::new_spinner();
        bar.set_style(
            ProgressStyle::with_template("{msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner())
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
        );
        bar.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(bar)
    } else {
        None
    }
}

/// CSV 并行导出路径：以 `jobs` 个线程并行处理所有日志文件。
/// 返回 `(processed_files, skipped_files, stats)`。
#[allow(clippy::too_many_arguments)]
fn run_csv_parallel(
    log_files: &[PathBuf],
    final_cfg: &Config,
    pipeline: &crate::pipeline::Pipeline,
    jobs: usize,
    show_progress: bool,
    interrupted: &Arc<AtomicBool>,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    field_mask: crate::pipeline::FieldMask,
    ordered_indices: &[usize],
    verbose: bool,
) -> Result<(Vec<(PathBuf, usize)>, usize, ErrorStats)> {
    if verbose {
        eprintln!(
            "Processing {} files in parallel ({} jobs)",
            log_files.len(),
            jobs
        );
    }
    info!("Parsing and exporting SQL logs (parallel, {jobs} jobs)...");
    let (processed_files, skipped, stats) = process_csv_parallel(
        log_files,
        final_cfg,
        pipeline,
        jobs,
        show_progress,
        interrupted,
        do_normalize,
        placeholder_override,
        field_mask,
        ordered_indices,
    )?;
    Ok((processed_files, skipped, stats))
}

/// `SQLite` 并行导出路径：以 `jobs` 个线程并行处理所有日志文件。
/// 返回 `(processed_files, skipped_files, stats)`。
#[allow(clippy::too_many_arguments)]
fn run_sqlite_parallel(
    log_files: &[PathBuf],
    final_cfg: &Config,
    pipeline: &crate::pipeline::Pipeline,
    jobs: usize,
    show_progress: bool,
    interrupted: &Arc<AtomicBool>,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    field_mask: crate::pipeline::FieldMask,
    ordered_indices: &[usize],
    verbose: bool,
) -> Result<(Vec<(PathBuf, usize)>, usize, ErrorStats)> {
    if verbose {
        eprintln!(
            "Processing {} files in parallel ({} jobs)",
            log_files.len(),
            jobs
        );
    }
    info!("Parsing and exporting SQL logs (SQLite parallel, {jobs} jobs)...");
    let (processed_files, skipped, stats) = process_sqlite_parallel(
        log_files,
        final_cfg,
        pipeline,
        jobs,
        show_progress,
        interrupted,
        do_normalize,
        placeholder_override,
        field_mask,
        ordered_indices,
    )?;
    Ok((processed_files, skipped, stats))
}

/// 顺序导出路径：逐文件处理，维护 `ExporterManager` 生命周期。
/// 返回 `(per_file_counts, run_stats)`。
#[allow(clippy::too_many_arguments)]
#[allow(clippy::fn_params_excessive_bools)]
fn run_sequential(
    log_files: &[PathBuf],
    final_cfg: &Config,
    pipeline: &crate::pipeline::Pipeline,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    verbose: bool,
    quiet: bool,
    show_progress: bool,
    pb: Option<&ProgressBar>,
    interrupted: &Arc<AtomicBool>,
) -> Result<(Vec<(PathBuf, usize)>, ErrorStats)> {
    let mut exporter_manager = ExporterManager::from_config(final_cfg)?;
    exporter_manager.initialize()?;
    info!("Parsing and exporting SQL logs...");
    let (per_file_counts, run_stats) = run_file_loop(
        log_files,
        &mut exporter_manager,
        pipeline,
        do_normalize,
        placeholder_override,
        verbose,
        show_progress,
        pb,
        interrupted,
    )?;
    exporter_manager.finalize()?;
    (!quiet).then(|| exporter_manager.log_stats());
    Ok((per_file_counts, run_stats))
}

/// 逐文件循环：为每个日志文件调用 `process_log_file`，fatal 时提前返回错误。
/// 返回 `(per_file_counts, run_stats)`。
#[allow(clippy::too_many_arguments)]
#[allow(clippy::fn_params_excessive_bools)]
fn run_file_loop(
    log_files: &[PathBuf],
    exporter_manager: &mut ExporterManager,
    pipeline: &crate::pipeline::Pipeline,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    verbose: bool,
    show_progress: bool,
    pb: Option<&ProgressBar>,
    interrupted: &Arc<AtomicBool>,
) -> Result<(Vec<(PathBuf, usize)>, ErrorStats)> {
    let mut params_buffer = crate::pipeline::normalizer::ParamBuffer::default();
    let mut ns_scratch: Vec<u8> = Vec::with_capacity(4096);
    let mut per_file_counts: Vec<(PathBuf, usize)> = Vec::with_capacity(log_files.len());
    let mut run_stats = ErrorStats::default();
    for (idx, log_file) in log_files.iter().enumerate() {
        if interrupted.load(Ordering::Relaxed) {
            break;
        }
        verbose.then(|| eprintln!("Processing: {}", log_file.display()));
        let (processed, file_stats) = process_log_file(
            &log_file.to_string_lossy(),
            idx + 1,
            log_files.len(),
            exporter_manager,
            pipeline,
            show_progress,
            None,
            interrupted,
            do_normalize,
            placeholder_override,
            &mut params_buffer,
            &mut ns_scratch,
            true,
            pb,
        )?;
        per_file_counts.push((log_file.clone(), processed));
        run_stats.merge(&file_stats);
        if file_stats.has_fatal() {
            return Err(Error::Export(crate::error::ExportError::WriteFailed {
                path: log_file.into(),
                reason: file_stats.fatal_error.unwrap_or_default(),
            }));
        }
    }
    Ok((per_file_counts, run_stats))
}

/// 输出运行摘要（文件数、记录数、耗时、错误统计）。`quiet` 为 true 时不输出任何内容。
fn print_run_summary(
    quiet: bool,
    verbose: bool,
    use_parallel: bool,
    elapsed: f64,
    processed_files: &[(PathBuf, usize)],
    total_records: usize,
    skipped_files: usize,
    run_stats: &ErrorStats,
) {
    if !quiet {
        let mode_label = if use_parallel { " [parallel]" } else { "" };
        let skip_label = if skipped_files > 0 {
            format!(", {skipped_files} skipped")
        } else {
            String::new()
        };
        if verbose && !processed_files.is_empty() {
            for (path, count) in processed_files {
                eprintln!("Processed: {} — {} records", path.display(), count);
            }
        }
        eprintln!(
            "\n✓ SQL Log Export Task Completed{mode_label} in {elapsed:.2}s — {total_records} records total{skip_label}",
        );
        if run_stats.has_errors() {
            eprintln!(
                "  Errors: {} total ({} parse, {} export)",
                run_stats.total_errors, run_stats.parse_errors, run_stats.export_errors
            );
        }
    }
}

#[cfg(test)]
mod tests;
