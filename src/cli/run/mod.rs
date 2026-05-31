use crate::config::Config;
use crate::error::{Error, ErrorStats, Result};
use crate::exporter::ExporterManager;
use crate::parser::SqllogParser;
use crate::pipeline::{CompiledMetaFilters, CompiledSqlFilters};
use log::{info, warn};
use std::io::IsTerminal;
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
use prescan::{recompile_meta_if_needed, scan_for_trxids_by_transaction_filters};
use processor::process_log_file;
use sqlite_parallel::process_sqlite_parallel;

/// 主编排函数：解析日志文件并导出到配置的导出器。
/// `compiled_filters` 由调用方预编译（`Config::validate_and_compile`），避免重复编译正则。
/// 并行路径：CSV + 多文件 + jobs > 1；顺序路径：其他情况。
pub fn handle_run(
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<AtomicBool>,
    compiled_filters: Option<(CompiledMetaFilters, CompiledSqlFilters)>,
) -> Result<ErrorStats> {
    let (compiled_meta, compiled_sql) = match compiled_filters {
        Some((m, s)) => (Some(m), Some(s)),
        None => (None, None),
    };
    let total_start = Instant::now();

    let log_files = SqllogParser::new(cfg.sqllog.inputs.clone()).log_files()?;
    let mut run_stats = ErrorStats::default();

    // Stdin pipe mode: fall back when no log files found AND stdin is not a terminal.
    // /dev/stdin is Unix-only; skip pipe mode on Windows.
    let is_stdin_pipe =
        log_files.is_empty() && !std::io::stdin().is_terminal() && !cfg!(target_os = "windows");
    let log_files = if is_stdin_pipe {
        info!("No log files found, reading from stdin (pipe mode)");
        vec![std::path::PathBuf::from("/dev/stdin")]
    } else if log_files.is_empty() {
        return Err(crate::error::Error::Parser(
            crate::error::ParserError::NoFilesFound {
                inputs: cfg.sqllog.inputs.clone(),
            },
        ));
    } else {
        log_files
    };
    let jobs = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);

    // 仅当有事务级过滤器时才克隆配置（避免常规路径的额外分配）
    let owned_cfg;
    let final_cfg: &Config = if cfg
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
            cfg
        } else {
            let extra_trxids = scan_for_trxids_by_transaction_filters(&log_files, cfg, jobs)?;
            let mut tmp = cfg.clone();
            if let Some(f) = &mut tmp.filter {
                f.merge_found_trxids(extra_trxids);
            }
            owned_cfg = tmp;
            &owned_cfg
        }
    } else {
        cfg
    };
    let compiled_meta_for_pipeline = recompile_meta_if_needed(final_cfg, compiled_meta)?;
    let pipeline = build_pipeline(final_cfg, compiled_meta_for_pipeline);
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
    let compiled_record_sql: Option<CompiledSqlFilters> = compiled_sql.filter(|_| {
        final_cfg
            .filter
            .as_ref()
            .is_some_and(|f| f.enable && f.record_sql.has_filters())
    });
    let sql_record_filter = compiled_record_sql.as_ref();
    let show_progress = !quiet && !verbose;
    let pb = if show_progress {
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
    };
    let mut total_records = 0usize;
    let mut skipped_files = 0usize;
    let use_csv_parallel =
        jobs > 1 && log_files.len() > 1 && !is_stdin_pipe && final_cfg.exporter.csv.is_some();
    let use_sqlite_parallel =
        jobs > 1 && log_files.len() > 1 && !is_stdin_pipe && final_cfg.exporter.sqlite.is_some();
    let use_parallel = use_csv_parallel || use_sqlite_parallel;

    let processed_files: Vec<(std::path::PathBuf, usize)> = if use_csv_parallel {
        if verbose {
            eprintln!(
                "Processing {} files in parallel ({} jobs)",
                log_files.len(),
                jobs
            );
        }
        info!("Parsing and exporting SQL logs (parallel, {jobs} jobs)...");
        let (csv_processed_files, parallel_skipped, csv_parallel_stats) = process_csv_parallel(
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
            sql_record_filter,
        )?;
        run_stats.merge(&csv_parallel_stats);
        total_records = csv_processed_files.iter().map(|(_, c)| *c).sum();
        skipped_files = parallel_skipped;
        csv_processed_files
    } else if use_sqlite_parallel {
        if verbose {
            eprintln!(
                "Processing {} files in parallel ({} jobs)",
                log_files.len(),
                jobs
            );
        }
        info!("Parsing and exporting SQL logs (SQLite parallel, {jobs} jobs)...");
        let (sqlite_processed_files, parallel_skipped, sqlite_parallel_stats) =
            process_sqlite_parallel(
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
                sql_record_filter,
            )?;
        run_stats.merge(&sqlite_parallel_stats);
        total_records = sqlite_processed_files.iter().map(|(_, c)| *c).sum();
        skipped_files = parallel_skipped;
        sqlite_processed_files
    } else {
        let mut exporter_manager = ExporterManager::from_config(final_cfg)?;
        exporter_manager.initialize()?;
        info!("Parsing and exporting SQL logs...");
        let mut params_buffer = crate::pipeline::normalizer::ParamBuffer::default();
        let mut ns_scratch: Vec<u8> = Vec::with_capacity(4096);
        let mut per_file_counts: Vec<(std::path::PathBuf, usize)> =
            Vec::with_capacity(log_files.len());
        for (idx, log_file) in log_files.iter().enumerate() {
            if interrupted.load(Ordering::Relaxed) {
                break;
            }
            if verbose {
                eprintln!("Processing: {}", log_file.display());
            }
            let (processed, file_stats) = process_log_file(
                &log_file.to_string_lossy(),
                idx + 1,
                log_files.len(),
                &mut exporter_manager,
                &pipeline,
                show_progress,
                None,
                interrupted,
                do_normalize,
                placeholder_override,
                &mut params_buffer,
                &mut ns_scratch,
                true,
                sql_record_filter,
                pb.as_ref(),
            )?;
            total_records += processed;
            per_file_counts.push((log_file.clone(), processed));
            run_stats.merge(&file_stats);
            if file_stats.has_fatal() {
                return Err(Error::Export(crate::error::ExportError::WriteFailed {
                    path: log_file.into(),
                    reason: file_stats.fatal_error.unwrap_or_default(),
                }));
            }
        }
        exporter_manager.finalize()?;
        if !quiet {
            exporter_manager.log_stats();
        }
        per_file_counts
    };
    if !quiet {
        let elapsed = total_start.elapsed().as_secs_f64();
        let mode_label = if use_parallel { " [parallel]" } else { "" };
        let skip_label = if skipped_files > 0 {
            format!(", {skipped_files} skipped")
        } else {
            String::new()
        };
        if verbose && !processed_files.is_empty() {
            for (path, count) in &processed_files {
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
    if let Some(pb) = &pb {
        pb.finish_and_clear();
    }
    if interrupted.load(Ordering::Relaxed) {
        return Err(Error::Interrupted);
    }
    Ok(run_stats)
}

#[cfg(test)]
mod tests;
