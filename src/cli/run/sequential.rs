use super::processor::process_log_file;
use crate::config::Config;
use crate::error::{Error, ErrorStats, Result};
use crate::exporter::ExporterManager;
use indicatif::ProgressBar;
use log::info;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// 顺序导出路径：逐文件处理，维护 `ExporterManager` 生命周期。
/// 返回 `(per_file_counts, run_stats)`。
#[allow(clippy::too_many_arguments)]
#[allow(clippy::fn_params_excessive_bools)]
pub(super) fn run_sequential(
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
