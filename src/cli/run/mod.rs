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

mod collector;
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
/// `jobs_override` 为测试钩子，生产代码传 None 保持 `available_parallelism` 原行为。
pub fn handle_run(
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
    let use_csv_parallel =
        jobs > 1 && log_files.len() > 1 && !is_stdin_pipe && final_cfg.exporter.csv.is_some();
    let use_sqlite_parallel =
        jobs > 1 && log_files.len() > 1 && !is_stdin_pipe && final_cfg.exporter.sqlite.is_some();
    let use_parallel = use_csv_parallel || use_sqlite_parallel;
    let show_progress = !quiet && !verbose && !use_parallel;
    let pb = make_progress_bar(show_progress, log_files.len());
    let mut skipped_files = 0usize;
    let processed_files: Vec<(PathBuf, usize)> = if use_csv_parallel {
        if verbose {
            eprintln!(
                "Processing {} files in parallel ({} jobs)",
                log_files.len(),
                jobs
            );
        }
        info!("Parsing and exporting SQL logs (parallel, {jobs} jobs)...");
        let (files, skipped, stats) = process_csv_parallel(
            &log_files,
            final_cfg,
            &pipeline,
            jobs,
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
        if verbose {
            eprintln!(
                "Processing {} files in parallel ({} jobs)",
                log_files.len(),
                jobs
            );
        }
        info!("Parsing and exporting SQL logs (SQLite parallel, {jobs} jobs)...");
        let (files, skipped, stats) = process_sqlite_parallel(
            &log_files,
            final_cfg,
            &pipeline,
            jobs,
            interrupted,
            do_normalize,
            placeholder_override,
            field_mask,
            &ordered_indices,
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
    run_stats.records_exported = total_records;
    if let Some(pb) = &pb {
        pb.finish_and_clear();
    }
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
    write_error_log(final_cfg, &run_stats);
    if interrupted.load(Ordering::Acquire) {
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
    quiet: bool,
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
            if !quiet {
                eprintln!(
                    "[WARN] Transaction-level filters with stdin: pre-scan disabled, \
                     degrading to per-record matching."
                );
            }
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

/// 创建文件计数进度条，`show_progress` 为 false 时返回 `None`。
/// `total_files` 决定 `{pos}/{len}` 计数器的上限，ETA 由 indicatif 自动计算。
fn make_progress_bar(show_progress: bool, total_files: usize) -> Option<ProgressBar> {
    if show_progress {
        let bar = ProgressBar::new(total_files as u64);
        bar.set_style(
            ProgressStyle::with_template("{spinner:.cyan} [{pos}/{len}] {wide_msg} | eta {eta}")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
        );
        bar.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(bar)
    } else {
        None
    }
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
    let loop_result = run_file_loop(
        log_files,
        &mut exporter_manager,
        pipeline,
        do_normalize,
        placeholder_override,
        verbose,
        show_progress,
        pb,
        interrupted,
    );
    // 无论 loop_result 成功与否都调用 finalize，确保 BufWriter 数据落盘
    let finalize_result = exporter_manager.finalize();
    (!quiet).then(|| exporter_manager.log_stats());
    let (per_file_counts, run_stats) = match loop_result {
        Ok(v) => v,
        Err(loop_err) => {
            if let Err(fin_err) = finalize_result {
                log::warn!("finalize failed during loop error cleanup: {fin_err}");
            }
            return Err(loop_err);
        }
    };
    finalize_result?;
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
        if interrupted.load(Ordering::Acquire) {
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
        // 先检查 fatal，再合并统计：fatal 路径直接返回，合并无意义
        if file_stats.has_fatal() {
            return Err(Error::Export(crate::error::ExportError::DatabaseFailed {
                reason: file_stats.fatal_error.unwrap_or_default(),
            }));
        }
        per_file_counts.push((log_file.clone(), processed));
        run_stats.merge(&file_stats);
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
            eprintln!(
                "  errors by type: encoding={}, field_missing={}, parse_failed={}",
                run_stats
                    .by_type
                    .get(&crate::error::ErrorKind::EncodingError)
                    .copied()
                    .unwrap_or(0),
                run_stats
                    .by_type
                    .get(&crate::error::ErrorKind::FieldMissing)
                    .copied()
                    .unwrap_or(0),
                run_stats
                    .by_type
                    .get(&crate::error::ErrorKind::ParseFailed)
                    .copied()
                    .unwrap_or(0),
            );
        }
        if run_stats.filtered_out > 0 {
            let total_read = total_records as u64 + run_stats.filtered_out;
            #[allow(clippy::cast_precision_loss)]
            let pct = if total_read > 0 {
                run_stats.filtered_out as f64 / total_read as f64 * 100.0
            } else {
                0.0
            };
            eprintln!(
                "  filtered: {} records ({pct:.1}% of {} total)",
                run_stats.filtered_out, total_read
            );
        }
        if run_stats
            .by_type
            .get(&crate::error::ErrorKind::EncodingError)
            .copied()
            .unwrap_or(0)
            > 0
        {
            eprintln!("  hint: 多行 encoding_error — 建议检查文件编码是否为 GBK/GB18030");
        }
        if run_stats
            .by_type
            .get(&crate::error::ErrorKind::FieldMissing)
            .copied()
            .unwrap_or(0)
            > 0
        {
            eprintln!("  hint: 多行 field_missing — 建议确认日志格式与 DM SQL log 格式一致");
        }
    }
}

/// 将解析错误记录批量写出到配置的 error log 文件（覆盖模式）。
/// 无配置或无错误时为空操作；写出失败仅 warn 不终止。
fn write_error_log(cfg: &crate::config::Config, stats: &ErrorStats) {
    let Some(error_cfg) = cfg.error.as_ref() else {
        return;
    };
    if stats.parse_error_records.is_empty() {
        return;
    }
    use std::io::Write;
    let file = match std::fs::File::create(&error_cfg.file) {
        Ok(f) => f,
        Err(e) => {
            log::warn!("Failed to create error log {}: {e}", error_cfg.file);
            return;
        }
    };
    let mut writer = std::io::BufWriter::new(file);
    let truncated = stats.parse_errors > stats.parse_error_records.len();
    for rec in &stats.parse_error_records {
        let _ = writeln!(
            writer,
            "[ERROR] line {}: {}  reason: {}",
            rec.line_number,
            rec.raw_truncated,
            rec.kind.kind_display()
        );
    }
    if truncated {
        let _ = writeln!(
            writer,
            "[truncated; showing first 10000 of {} total parse errors]",
            stats.parse_errors
        );
    }
    if let Err(e) = writer.flush() {
        log::warn!("Failed to flush error log: {e}");
    }
}

#[cfg(test)]
mod tests;
