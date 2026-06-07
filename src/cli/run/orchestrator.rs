use super::error_log::write_error_log;
use super::filter_processor::build_pipeline;
use super::input::{make_progress_bar, merge_trxid_prescan, resolve_input_files};
use super::parallel::process_csv_parallel;
use super::sequential::run_sequential;
use super::sqlite_parallel::process_sqlite_parallel;
use super::summary::print_run_summary;
use crate::config::Config;
use crate::error::{Error, ErrorStats, Result};
use log::info;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

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
