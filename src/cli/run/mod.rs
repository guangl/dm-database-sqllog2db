use crate::color;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::exporter::ExporterManager;
use crate::parser::SqllogParser;
use crate::pipeline::template_reporter::TemplateReporter;
use crate::pipeline::{CompiledMetaFilters, CompiledSqlFilters, TemplateAggregator};
use crate::pipeline::{derive_template_report_paths, templates_report_enabled};
use indicatif::HumanCount;
use log::{info, warn};
use std::path::{Path, PathBuf};
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

/// 将模板统计报告写入独立文件（`[templates]` 配置段路径）
fn write_template_reports(cfg: &Config, stats: &[crate::pipeline::TemplateStats]) -> Result<()> {
    if !templates_report_enabled(cfg) {
        return Ok(());
    }
    let (derived_csv, derived_sqlite) = derive_template_report_paths(cfg);
    let csv_path = cfg
        .templates
        .as_ref()
        .and_then(|t| {
            if t.csv_report_path.trim().is_empty() {
                None
            } else {
                Some(PathBuf::from(&t.csv_report_path))
            }
        })
        .or(derived_csv);
    let sqlite_path = cfg
        .templates
        .as_ref()
        .and_then(|t| {
            if t.sqlite_report_path.trim().is_empty() {
                None
            } else {
                Some(PathBuf::from(&t.sqlite_report_path))
            }
        })
        .or(derived_sqlite);
    if let Some(ref path) = csv_path {
        TemplateReporter::write_csv(path, stats)?;
    }
    if let Some(ref path) = sqlite_path {
        TemplateReporter::write_sqlite(path, stats)?;
    }
    Ok(())
}

/// 主编排函数：解析日志文件并导出到配置的导出器。
/// `compiled_filters` 由调用方预编译（`Config::validate_and_compile`），避免重复编译正则。
/// 并行路径：CSV + 多文件 + 无 limit + jobs > 1；顺序路径：其他情况。
pub fn handle_run(
    cfg: &Config,
    limit: Option<usize>,
    dry_run: bool,
    quiet: bool,
    interrupted: &Arc<AtomicBool>,
    progress_interval: u64,
    resume: bool,
    state_file_override: Option<&str>,
    jobs: usize,
    compiled_filters: Option<(CompiledMetaFilters, CompiledSqlFilters)>,
) -> Result<()> {
    let (compiled_meta, compiled_sql) = match compiled_filters {
        Some((m, s)) => (Some(m), Some(s)),
        None => (None, None),
    };
    let total_start = Instant::now();
    let log_files = SqllogParser::new(&cfg.sqllog.path).log_files()?;
    if log_files.is_empty() {
        warn!("No log files found");
        return Ok(());
    }
    let state_path =
        std::path::PathBuf::from(state_file_override.unwrap_or(&cfg.resume.state_file));
    let mut resume_state = if resume {
        let state = crate::resume::ResumeState::load(&state_path);
        info!(
            "Resume mode: state file {}, {} files previously processed",
            state_path.display(),
            state.processed_count()
        );
        Some(state)
    } else {
        None
    };
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
    let do_template = final_cfg.template.as_ref().is_some_and(|t| t.enable);
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
    let pb = make_progress_bar(quiet, progress_interval);
    let mut total_records = 0usize;
    let mut skipped_files = 0usize;
    let use_parallel = !dry_run
        && jobs > 1
        && log_files.len() > 1
        && limit.is_none()
        && final_cfg.exporter.csv.is_some();

    if use_parallel {
        info!("Parsing and exporting SQL logs (parallel, {jobs} jobs)...");
        let (processed_files, parallel_skipped, parallel_agg) = process_csv_parallel(
            &log_files,
            final_cfg,
            &pipeline,
            jobs,
            &pb,
            interrupted,
            resume_state.as_ref(),
            quiet,
            do_normalize,
            do_template,
            placeholder_override,
            field_mask,
            &ordered_indices,
            sql_record_filter,
        )?;
        total_records = processed_files.iter().map(|(_, c)| *c).sum();
        skipped_files = parallel_skipped;
        if let Some(ref agg) = parallel_agg {
            if let Some(charts_cfg) = final_cfg.charts.as_ref() {
                crate::charts::generate_charts(agg, charts_cfg)?;
            }
        }
        let template_stats = parallel_agg.map(TemplateAggregator::finalize);
        if let Some(ref stats) = template_stats {
            info!("Template analysis: {} unique templates", stats.len());

            if !templates_report_enabled(final_cfg) {
                let csv_out_path = final_cfg
                    .template
                    .as_ref()
                    .filter(|t| !t.output_csv_path.trim().is_empty())
                    .map(|t| t.output_csv_path.as_str());
                let sqlite_table = final_cfg
                    .template
                    .as_ref()
                    .filter(|t| !t.output_sqlite_table.trim().is_empty())
                    .map(|t| t.output_sqlite_table.as_str());
                if let Some(path_str) = csv_out_path {
                    crate::exporter::csv::write_companion_rows(Path::new(path_str), stats)?;
                }
                if let Some(table_name) = sqlite_table {
                    if let Some(sqlite_cfg) = final_cfg.exporter.sqlite.as_ref() {
                        use crate::exporter::{Exporter, SqliteExporter};
                        let mut sqlite = SqliteExporter::from_config(sqlite_cfg);
                        sqlite.open_connection_only()?;
                        sqlite.write_template_stats(stats, None, Some(table_name))?;
                    }
                }
            }
            write_template_reports(final_cfg, stats)?;
        }
        if !interrupted.load(Ordering::Relaxed) {
            if let Some(state) = &mut resume_state {
                for (file, count) in &processed_files {
                    state.mark_processed(file, *count as u64)?;
                }
                state.save(&state_path)?;
            }
        }
    } else {
        let mut exporter_manager = if dry_run {
            ExporterManager::dry_run()
        } else {
            ExporterManager::from_config(final_cfg)?
        };
        exporter_manager.initialize()?;
        info!(
            "{}",
            if dry_run {
                "Dry-run: parsing SQL logs without writing output..."
            } else {
                "Parsing and exporting SQL logs..."
            }
        );
        let mut params_buffer = crate::pipeline::normalizer::ParamBuffer::default();
        let mut ns_scratch: Vec<u8> = Vec::with_capacity(4096);
        let mut template_agg = do_template.then(TemplateAggregator::new);
        for (idx, log_file) in log_files.iter().enumerate() {
            if interrupted.load(Ordering::Relaxed) {
                break;
            }
            let remaining = limit.map(|l| l.saturating_sub(total_records));
            if remaining == Some(0) {
                break;
            }
            if let Some(state) = &resume_state {
                if state.is_processed(log_file) {
                    skipped_files += 1;
                    pb.println(format!(
                        "{} [{}/{}] {} — skipped (already processed)",
                        color::dim("⏭"),
                        idx + 1,
                        log_files.len(),
                        log_file.display()
                    ));
                    continue;
                }
            }
            let processed = process_log_file(
                &log_file.to_string_lossy(),
                idx + 1,
                log_files.len(),
                &mut exporter_manager,
                &pipeline,
                &pb,
                remaining,
                interrupted,
                do_normalize,
                template_agg.as_mut(),
                placeholder_override,
                &mut params_buffer,
                &mut ns_scratch,
                true,
                sql_record_filter,
            )?;
            if !dry_run {
                if let Some(state) = &mut resume_state {
                    state.mark_processed(log_file, processed as u64)?;
                    state.save(&state_path)?;
                }
            }
            total_records += processed;
            if limit.is_some_and(|l| total_records >= l) {
                break;
            }
        }
        if let Some(ref agg) = template_agg {
            if let Some(charts_cfg) = final_cfg.charts.as_ref() {
                crate::charts::generate_charts(agg, charts_cfg)?;
            }
        }
        exporter_manager.finalize()?;
        if !quiet {
            exporter_manager.log_stats();
        }
        let template_stats = template_agg.map(TemplateAggregator::finalize);
        if let Some(ref stats) = template_stats {
            info!("Template analysis: {} unique templates", stats.len());

            if !templates_report_enabled(final_cfg) {
                let csv_out_path = final_cfg
                    .template
                    .as_ref()
                    .filter(|t| !t.output_csv_path.trim().is_empty())
                    .map(|t| t.output_csv_path.as_str());
                let sqlite_table = final_cfg
                    .template
                    .as_ref()
                    .filter(|t| !t.output_sqlite_table.trim().is_empty())
                    .map(|t| t.output_sqlite_table.as_str());
                exporter_manager.write_template_stats(stats, csv_out_path, sqlite_table)?;
            }
            write_template_reports(final_cfg, stats)?;
        }
    }
    pb.finish_and_clear();
    if !quiet {
        let elapsed = total_start.elapsed().as_secs_f64();
        let mode_label = if dry_run {
            " [dry-run]"
        } else if use_parallel {
            " [parallel]"
        } else {
            ""
        };
        let skip_label = if skipped_files > 0 {
            format!(", {} skipped", color::dim(HumanCount(skipped_files as u64)))
        } else {
            String::new()
        };
        eprintln!(
            "\n{} SQL Log Export Task Completed{mode_label} in {elapsed:.2}s — {} records total{skip_label}",
            color::green("✓"),
            color::green(HumanCount(total_records as u64)),
        );
    }
    if interrupted.load(Ordering::Relaxed) {
        return Err(Error::Interrupted);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
