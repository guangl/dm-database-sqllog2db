use crate::config::Config;
use crate::error::{Error, ErrorStats, Result};
use crate::exporter::ExporterManager;
use crate::parser::SqllogParser;
use crate::pipeline::{CompiledMetaFilters, CompiledSqlFilters};
use log::{info, warn};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

mod filter_processor;
mod parallel;
mod prescan;
mod processor;

use filter_processor::{build_pipeline, make_progress_bar};
use parallel::process_csv_parallel;
use prescan::{recompile_meta_if_needed, scan_for_trxids_by_transaction_filters};
use processor::process_log_file;

/// 主编排函数：解析日志文件并导出到配置的导出器。
/// `compiled_filters` 由调用方预编译（`Config::validate_and_compile`），避免重复编译正则。
/// 并行路径：CSV + 多文件 + jobs > 1；顺序路径：其他情况。
pub fn handle_run(
    cfg: &Config,
    quiet: bool,
    interrupted: &Arc<AtomicBool>,
    compiled_filters: Option<(CompiledMetaFilters, CompiledSqlFilters)>,
) -> Result<ErrorStats> {
    let (compiled_meta, compiled_sql) = match compiled_filters {
        Some((m, s)) => (Some(m), Some(s)),
        None => (None, None),
    };
    let total_start = Instant::now();
    let log_files = SqllogParser::new(&cfg.sqllog.path).log_files()?;
    let mut run_stats = ErrorStats::default();
    if log_files.is_empty() {
        warn!("No log files found");
        return Ok(ErrorStats::default());
    }
    let jobs = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);

    // 仅当有事务级过滤器时才克隆配置（避免常规路径的额外分配）
    let owned_cfg;
    let final_cfg: &Config = if cfg
        .filter
        .as_ref()
        .is_some_and(crate::pipeline::FiltersFeature::has_transaction_filters)
    {
        let extra_trxids = scan_for_trxids_by_transaction_filters(&log_files, cfg, jobs)?;
        let mut tmp = cfg.clone();
        if let Some(f) = &mut tmp.filter {
            f.merge_found_trxids(extra_trxids);
        }
        owned_cfg = tmp;
        &owned_cfg
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
    let show_progress = make_progress_bar(quiet, 80);
    let mut total_records = 0usize;
    let mut skipped_files = 0usize;
    let use_parallel = jobs > 1 && log_files.len() > 1 && final_cfg.exporter.csv.is_some();

    if use_parallel {
        info!("Parsing and exporting SQL logs (parallel, {jobs} jobs)...");
        let (processed_files, parallel_skipped) = process_csv_parallel(
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
        total_records = processed_files.iter().map(|(_, c)| *c).sum();
        skipped_files = parallel_skipped;
    } else {
        let mut exporter_manager = ExporterManager::from_config(final_cfg)?;
        exporter_manager.initialize()?;
        info!("Parsing and exporting SQL logs...");
        let mut params_buffer = crate::pipeline::normalizer::ParamBuffer::default();
        let mut ns_scratch: Vec<u8> = Vec::with_capacity(4096);
        for (idx, log_file) in log_files.iter().enumerate() {
            if interrupted.load(Ordering::Relaxed) {
                break;
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
            )?;
            total_records += processed;
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
    }
    if !quiet {
        let elapsed = total_start.elapsed().as_secs_f64();
        let mode_label = if use_parallel { " [parallel]" } else { "" };
        let skip_label = if skipped_files > 0 {
            format!(", {skipped_files} skipped")
        } else {
            String::new()
        };
        eprintln!(
            "\n✓ SQL Log Export Task Completed{mode_label} in {elapsed:.2}s — {total_records} records total{skip_label}",
        );
    }
    if interrupted.load(Ordering::Relaxed) {
        return Err(Error::Interrupted);
    }
    Ok(run_stats)
}

#[cfg(test)]
mod tests;
