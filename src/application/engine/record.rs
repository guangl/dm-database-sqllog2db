//! 记录级处理循环：驱动路径共享的"过滤 → 归一化 → 写出"逻辑。
//!
//! - [`process_log_file`]：顺序路径（`sequential`）的单文件主循环，
//!   带进度条与 fatal 错误响应。

use crate::engine::context::RunContext;
use crate::error::ErrorStats;
use crate::exporter::ExporterManager;
use crate::input::export_records;
use crate::model::LogRecord;
use crate::pipeline::normalizer::ParamBuffer;
use indicatif::ProgressBar;
use log::info;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// [`process_log_file`] 的入参打包：运行上下文 + 单文件的定位/进度信息。
pub(super) struct ProcessArgs<'a> {
    pub(super) ctx: &'a RunContext<'a>,
    pub(super) file_path: &'a str,
    pub(super) file_index: usize,
    pub(super) total_files: usize,
    pub(super) pb: Option<&'a ProgressBar>,
}

/// 单条记录导出的只读环境：运行上下文、性能指标开关、文件路径（用于日志）。
pub(super) struct ExportEnv<'a> {
    pub(super) ctx: &'a RunContext<'a>,
    pub(super) include_pm: bool,
    pub(super) file_path: &'a str,
}

/// 记录循环中被反复读写的可变状态（scratch 缓冲 + 计数 + 统计）。
pub(super) struct LoopState<'a> {
    pub(super) params_buffer: &'a mut ParamBuffer,
    pub(super) ns_scratch: &'a mut Vec<u8>,
    pub(super) records_in_file: usize,
    pub(super) file_stats: ErrorStats,
}

/// 控制主循环对单条记录的导出结果响应。
pub(super) enum ExportAction {
    /// 正常导出（或被过滤后 `params_buffer` 已更新），继续处理下一条。
    Continue,
    /// 遇到 fatal 导出错误，跳出主循环。
    BreakFatal,
}

/// 被过滤的 PARAMS 记录仅更新 `params_buffer`，不导出。
fn update_params_buffer_only(
    record: &LogRecord,
    state: &mut LoopState<'_>,
    placeholder: Option<bool>,
) {
    let _ = crate::pipeline::compute_normalized(
        record,
        &record.sql,
        state.params_buffer,
        &[],
        placeholder,
        state.ns_scratch,
    );
}

/// 对单条已过滤的记录执行归一化 + 导出 + 错误处理。
///
/// `passes`：调用方已判断该记录是否通过过滤器。
/// 仅在 `passes==false && do_normalize && record.tag.is_none()` 时更新 `params_buffer`（不导出）。
pub(super) fn normalize_and_export(
    env: &ExportEnv<'_>,
    record: &LogRecord,
    exporter_manager: &mut ExporterManager,
    state: &mut LoopState<'_>,
    passes: bool,
) -> ExportAction {
    let do_normalize = env.ctx.do_normalize;
    let placeholder = env.ctx.placeholder_override;
    if !passes {
        if do_normalize && record.tag.is_none() {
            update_params_buffer_only(record, state, placeholder);
        }
        state.file_stats.filtered_out += 1;
        return ExportAction::Continue;
    }
    let ns = if do_normalize && (!state.params_buffer.is_empty() || record.tag.is_none()) {
        crate::pipeline::compute_normalized(
            record,
            &record.sql,
            state.params_buffer,
            &env.ctx.normalize_tags,
            placeholder,
            state.ns_scratch,
        )
    } else {
        None
    };
    let export_result = exporter_manager.export_one_preparsed(record, env.include_pm, ns);
    let file_path = env.file_path;
    match export_result {
        Ok(()) => {
            state.records_in_file += 1;
            ExportAction::Continue
        }
        Err(ref e) if e.is_fatal() => {
            state.file_stats.set_fatal(e.to_string());
            eprintln!("[{}] {file_path}: {e}", e.severity());
            log::warn!("{file_path} | fatal export error: {export_result:?}");
            ExportAction::BreakFatal
        }
        Err(ref e) => {
            state.file_stats.add_export_error();
            eprintln!("[{}] {file_path}: {e}", e.severity());
            log::warn!("{file_path} | export error: {export_result:?}");
            ExportAction::Continue
        }
    }
}

/// 文件处理结束时输出统计日志与进度条完成消息。
fn log_file_result(
    args: &ProcessArgs<'_>,
    records_in_file: usize,
    errors_in_file: usize,
    elapsed: f64,
) {
    let file_path = args.file_path;
    if errors_in_file > 0 {
        log::warn!("{file_path}: {errors_in_file} parse errors");
    }
    info!(
        "File {file_path}: {records_in_file} records, {errors_in_file} errors, total {elapsed:.2}s",
    );
    if let Some(pb) = args.pb {
        let errors_label = if errors_in_file > 0 {
            format!(", {errors_in_file} errors")
        } else {
            String::new()
        };
        pb.set_message(format!(
            "✓ [{}/{}] {file_path} — {records_in_file}{errors_label}, {elapsed:.2}s",
            args.file_index, args.total_files,
        ));
        pb.inc(1);
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
            // u32::MAX（约 42 亿条）以上速率显示饱和即可，f64::from 保证无损转换
            let records = f64::from(u32::try_from(records_in_file).unwrap_or(u32::MAX));
            let rec_per_s = records / elapsed.max(1e-9);
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

/// 处理单个日志文件，返回 `(实际导出记录数, 文件级错误统计)`。
pub(super) fn process_log_file(
    exporter_manager: &mut ExporterManager,
    args: &ProcessArgs<'_>,
    params_buffer: &mut ParamBuffer,
    ns_scratch: &mut Vec<u8>,
    interrupted: &Arc<AtomicBool>,
) -> (usize, ErrorStats) {
    params_buffer.clear();
    let env = ExportEnv {
        ctx: args.ctx,
        include_pm: exporter_manager.csv_include_performance_metrics(),
        file_path: args.file_path,
    };
    let file_start = Instant::now();
    let file_name = std::path::Path::new(args.file_path)
        .file_name()
        .map_or_else(
            || args.file_path.to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
    if let Some(pb) = args.pb {
        pb.set_message(format!(
            "[{}/{}] {file_name}",
            args.file_index, args.total_files
        ));
    }
    let records = match export_records(std::path::Path::new(args.file_path)) {
        Ok(it) => it,
        Err(e) => {
            log::warn!("parse failed for '{}': {e}", args.file_path);
            let mut file_stats = ErrorStats::default();
            file_stats.add_parse_error();
            return (0, file_stats);
        }
    };
    let mut state = LoopState {
        params_buffer,
        ns_scratch,
        records_in_file: 0,
        file_stats: ErrorStats::default(),
    };
    let pipeline = &args.ctx.pipeline;
    let do_normalize = args.ctx.do_normalize;
    let mut total_processed = 0usize;
    'outer: for result in records {
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                log::warn!("skipping malformed record in '{}': {e}", args.file_path);
                state.file_stats.add_parse_error();
                continue;
            }
        };
        let passes = pipeline.is_empty() || pipeline.run_with_meta(&record);
        let needs_processing = passes || (do_normalize && record.tag.is_none());
        if !needs_processing {
            continue;
        }
        let action = normalize_and_export(&env, &record, exporter_manager, &mut state, passes);
        total_processed = total_processed.wrapping_add(1);
        match action {
            ExportAction::BreakFatal => break 'outer,
            ExportAction::Continue
                if passes
                    && tick_progress(
                        args.pb,
                        state.records_in_file,
                        file_start,
                        &file_name,
                        interrupted,
                    ) =>
            {
                break 'outer;
            }
            ExportAction::Continue
                if !passes
                    && total_processed.trailing_zeros() >= 10
                    && interrupted.load(Ordering::Acquire) =>
            {
                break 'outer;
            }
            ExportAction::Continue => {}
        }
    }
    let elapsed = file_start.elapsed().as_secs_f64();
    log_file_result(
        args,
        state.records_in_file,
        state.file_stats.total_errors,
        elapsed,
    );
    (state.records_in_file, state.file_stats)
}
