use crate::engine::record::{ProcessArgs, process_log_file};
use crate::engine::run::{Console, FileCounts, RunContext};
use crate::error::{Error, ErrorStats, Result};
use crate::exporter::ExporterManager;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// 顺序导出路径：逐文件处理，维护 `ExporterManager` 生命周期。
/// 返回 `(per_file_counts, run_stats)`。
pub(crate) async fn run_sequential(
    ctx: &RunContext<'_>,
    log_files: &[PathBuf],
    console: &Console<'_>,
    interrupted: &Arc<AtomicBool>,
) -> Result<(FileCounts, ErrorStats)> {
    let mut exporter_manager = ExporterManager::from_config(ctx.cfg)?;
    exporter_manager.initialize()?;
    log::info!("Parsing and exporting SQL logs...");
    let loop_result =
        run_file_loop(ctx, log_files, &mut exporter_manager, console, interrupted).await;
    // 无论 loop_result 成功与否都调用 finalize，确保 BufWriter 数据落盘
    let finalize_result = exporter_manager.finalize();
    (!console.quiet).then(|| exporter_manager.log_stats());
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
async fn run_file_loop(
    ctx: &RunContext<'_>,
    log_files: &[PathBuf],
    exporter_manager: &mut ExporterManager,
    console: &Console<'_>,
    interrupted: &Arc<AtomicBool>,
) -> Result<(FileCounts, ErrorStats)> {
    let mut params_buffer = crate::pipeline::normalizer::ParamBuffer::default();
    let mut ns_scratch: Vec<u8> = Vec::with_capacity(4096);
    let mut per_file_counts: FileCounts = Vec::with_capacity(log_files.len());
    let mut run_stats = ErrorStats::default();
    let show_progress = console.pb.is_some();
    for (idx, log_file) in log_files.iter().enumerate() {
        if interrupted.load(Ordering::Acquire) {
            break;
        }
        console
            .verbose
            .then(|| eprintln!("Processing: {}", log_file.display()));
        let (processed, file_stats) = process_log_file(
            exporter_manager,
            &ProcessArgs {
                ctx,
                file_path: &log_file.to_string_lossy(),
                file_index: idx + 1,
                total_files: log_files.len(),
                show_progress,
                remaining: None,
                reset_pb: true,
                pb: console.pb,
            },
            &mut params_buffer,
            &mut ns_scratch,
            interrupted,
        )
        .await?;
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
