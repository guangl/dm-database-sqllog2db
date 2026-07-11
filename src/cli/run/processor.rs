use crate::error::{ErrorStats, Result};
use crate::exporter::ExporterManager;
use crate::pipeline::Pipeline;
use crate::pipeline::normalizer::ParamBuffer;
use crate::streaming::open_log_file;
use dm_database_parser_sqllog::Sqllog;
use indicatif::ProgressBar;
use log::info;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// 控制主循环对单条记录的导出结果响应。
pub(super) enum ExportAction {
    /// 正常导出（或被过滤后 `params_buffer` 已更新），继续处理下一条。
    Continue,
    /// 达到导出配额上限，跳出主循环。
    BreakQuota,
    /// 遇到 fatal 导出错误，跳出主循环。
    BreakFatal,
}

/// 被过滤的 PARAMS 记录仅更新 `params_buffer`，不导出。
///
/// 在 `normalize_and_export` 的 `!passes` 路径调用，封装 `compute_normalized` 调用。
fn update_params_buffer_only(
    record: &Sqllog,
    params_buffer: &mut ParamBuffer,
    placeholder_override: Option<bool>,
    ns_scratch: &mut Vec<u8>,
) {
    let _ = crate::pipeline::compute_normalized(
        record,
        &record.sql,
        params_buffer,
        placeholder_override,
        ns_scratch,
    );
}

/// 对单条已过滤的记录执行归一化 + 导出 + 错误处理。
///
/// `passes`：调用方已判断该记录是否通过过滤器。
/// 仅在 `passes==false && do_normalize && record.tag.is_none()` 时更新 `params_buffer`（不导出）。
#[allow(clippy::too_many_arguments)]
pub(super) fn normalize_and_export(
    record: &Sqllog,
    exporter_manager: &mut ExporterManager,
    include_pm: bool,
    do_normalize: bool,
    params_buffer: &mut ParamBuffer,
    placeholder_override: Option<bool>,
    ns_scratch: &mut Vec<u8>,
    remaining: Option<usize>,
    records_in_file: &mut usize,
    file_stats: &mut ErrorStats,
    file_path: &str,
    passes: bool,
) -> ExportAction {
    if !passes {
        if do_normalize && record.tag.is_none() {
            update_params_buffer_only(record, params_buffer, placeholder_override, ns_scratch);
        }
        file_stats.filtered_out += 1;
        return ExportAction::Continue;
    }
    let ns = if do_normalize && (!params_buffer.is_empty() || record.tag.is_none()) {
        crate::pipeline::compute_normalized(
            record,
            &record.sql,
            params_buffer,
            placeholder_override,
            ns_scratch,
        )
    } else {
        None
    };
    if let Some(remaining) = remaining {
        if *records_in_file >= remaining {
            return ExportAction::BreakQuota;
        }
    }
    let export_result = exporter_manager.export_one_preparsed(record, include_pm, ns);
    match export_result {
        Ok(()) => {
            *records_in_file += 1;
            ExportAction::Continue
        }
        Err(ref e) if e.is_fatal() => {
            file_stats.set_fatal(e.to_string());
            eprintln!("[{}] {file_path}: {e}", e.severity());
            log::warn!("{file_path} | fatal export error: {export_result:?}");
            ExportAction::BreakFatal
        }
        Err(ref e) => {
            file_stats.add_export_error();
            eprintln!("[{}] {file_path}: {e}", e.severity());
            log::warn!("{file_path} | export error: {export_result:?}");
            ExportAction::Continue
        }
    }
}

/// 在文件处理开始时设置进度条消息与位置。
///
/// 仅在 `reset_pb && show_progress` 时生效，否则为空操作。
fn setup_progress_bar(
    pb: Option<&ProgressBar>,
    reset_pb: bool,
    show_progress: bool,
    file_index: usize,
    total_files: usize,
    file_name: &str,
) {
    if reset_pb && show_progress {
        if let Some(pb) = pb {
            pb.set_message(format!("[{file_index}/{total_files}] {file_name}"));
        }
    }
}

/// 文件处理结束时输出统计日志与进度条完成消息。
#[allow(clippy::too_many_arguments)]
fn log_file_result(
    pb: Option<&ProgressBar>,
    show_progress: bool,
    file_path: &str,
    file_index: usize,
    total_files: usize,
    records_in_file: usize,
    errors_in_file: usize,
    elapsed: f64,
) {
    if errors_in_file > 0 {
        log::warn!("{file_path}: {errors_in_file} parse errors");
    }
    info!(
        "File {file_path}: {records_in_file} records, {errors_in_file} errors, total {elapsed:.2}s",
    );
    if show_progress {
        if let Some(pb) = pb {
            let errors_label = if errors_in_file > 0 {
                format!(", {errors_in_file} errors")
            } else {
                String::new()
            };
            pb.set_message(format!(
                "✓ [{file_index}/{total_files}] {file_path} — {records_in_file}{errors_label}, {elapsed:.2}s",
            ));
            pb.inc(1);
        }
    }
}

/// 每 1024 条记录更新进度条消息（嵌入 records/sec）并检查中断信号。
/// 返回 true 表示收到中断信号，调用方应跳出主循环。
fn tick_progress(
    pb: Option<&ProgressBar>,
    records_in_file: usize,
    file_start: std::time::Instant,
    file_name: &str,
    interrupted: &Arc<AtomicBool>,
) -> bool {
    if records_in_file == 0 {
        return false;
    }
    if records_in_file.trailing_zeros() >= 10 {
        if let Some(pb) = pb {
            let elapsed = file_start.elapsed().as_secs_f64();
            #[allow(clippy::cast_precision_loss)]
            let rec_per_s = records_in_file as f64 / elapsed.max(1e-9);
            let speed_label = if rec_per_s >= 10_000.0 {
                format!("{:.0}k rec/s", rec_per_s / 1000.0)
            } else {
                format!("{rec_per_s:.0} rec/s")
            };
            pb.set_message(format!("{file_name} | {speed_label}"));
        }
        if interrupted.load(Ordering::Acquire) {
            return true;
        }
    }
    false
}

/// 处理单个日志文件，返回本文件实际导出的记录数。
///
/// `remaining`: 最多再导出多少条记录（跨文件的剩余配额），`None` 表示不限制。
/// `reset_pb`: 是否在文件开始时重置进度条计数；并行模式传 `false`，避免多线程互相重置。
// 保持 `async fn` 签名以维持 run 子命令的异步调用链（`run_sequential` → `run_file_loop`）不变；
// 切换到流式解析后函数体内已无 `.await`。当前 clippy 之所以未报 `unused_async`，仅因
// `sequential.rs` 通过 `use` 导入本函数——该抑制并不可靠（换 clippy 版本、或改为全限定路径
// 调用即会触发 `-D warnings` 失败）。故显式加 allow，与 `scanner::scan_files` 的处理保持一致。
#[allow(clippy::unused_async)]
#[rustfmt::skip]
pub(super) async fn process_log_file(
    file_path: &str, file_index: usize, total_files: usize,
    exporter_manager: &mut ExporterManager, pipeline: &Pipeline,
    show_progress: bool, remaining: Option<usize>, interrupted: &Arc<AtomicBool>,
    do_normalize: bool, placeholder_override: Option<bool>,
    params_buffer: &mut ParamBuffer, ns_scratch: &mut Vec<u8>,
    reset_pb: bool, pb: Option<&ProgressBar>,
) -> Result<(usize, ErrorStats)> {
    params_buffer.clear();
    let include_pm = exporter_manager.csv_include_performance_metrics();
    let file_start = Instant::now();
    let file_name = std::path::Path::new(file_path).file_name()
        .map_or_else(|| file_path.to_string(), |n| n.to_string_lossy().into_owned());
    setup_progress_bar(pb, reset_pb, show_progress, file_index, total_files, &file_name);
    let file_path_buf = std::path::Path::new(file_path);
    let records = match open_log_file(file_path_buf) {
        Ok(it) => it,
        Err(e) => {
            log::warn!("parse failed for '{}': {e}", file_path_buf.display());
            let mut file_stats = ErrorStats::default();
            file_stats.add_parse_error();
            return Ok((0, file_stats));
        }
    };
    let (mut records_in_file, mut file_stats) = (0usize, ErrorStats::default());
    let mut total_processed = 0usize;
    'outer: for result in records {
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                log::warn!("skipping malformed record in '{file_path}': {e}");
                file_stats.add_parse_error();
                continue;
            }
        };
        let passes = pipeline.is_empty() || pipeline.run_with_meta(&record);
        let needs_processing = passes || (do_normalize && record.tag.is_none());
        if !needs_processing { continue; }
        let action = normalize_and_export(
            &record, exporter_manager, include_pm, do_normalize,
            params_buffer, placeholder_override, ns_scratch,
            remaining, &mut records_in_file, &mut file_stats, file_path, passes,
        );
        total_processed = total_processed.wrapping_add(1);
        match action {
            ExportAction::BreakQuota | ExportAction::BreakFatal => break 'outer,
            ExportAction::Continue if passes && tick_progress(pb, records_in_file, file_start, &file_name, interrupted) => break 'outer,
            ExportAction::Continue if !passes
                && total_processed.trailing_zeros() >= 10
                && interrupted.load(Ordering::Acquire) => break 'outer,
            ExportAction::Continue => {}
        }
    }
    let elapsed = file_start.elapsed().as_secs_f64();
    log_file_result(pb, show_progress, file_path, file_index, total_files, records_in_file, file_stats.total_errors, elapsed);
    Ok((records_in_file, file_stats))
}
