//! 记录级处理循环：驱动路径共享的"过滤 → 归一化 → 写出"逻辑。
//!
//! - [`process_log_file`]：顺序路径（`driver::sequential`）的单文件主循环，
//!   带进度条、导出配额与 fatal 错误响应。
//! - [`iterate_records`]：并行路径（`driver::parallel` / `driver::sqlite`）共享的
//!   记录迭代 + 过滤 + 归一化 + 写出回调函数（STRUCT-04）。

use crate::engine::run::RunContext;
use crate::error::{ErrorStats, Result};
use crate::exporter::ExporterManager;
use crate::pipeline::Pipeline;
use crate::pipeline::normalizer::ParamBuffer;
use crate::streaming::open_log_file;
use dm_database_parser_sqllog::{ParseError, Sqllog};
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
    pub(super) show_progress: bool,
    /// 最多再导出多少条记录（跨文件的剩余配额），`None` 表示不限制。
    pub(super) remaining: Option<usize>,
    /// 是否在文件开始时重置进度条计数；并行模式传 `false`，避免多线程互相重置。
    pub(super) reset_pb: bool,
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
    /// 达到导出配额上限，跳出主循环。
    BreakQuota,
    /// 遇到 fatal 导出错误，跳出主循环。
    BreakFatal,
}

/// 被过滤的 PARAMS 记录仅更新 `params_buffer`，不导出。
fn update_params_buffer_only(
    record: &Sqllog,
    state: &mut LoopState<'_>,
    placeholder: Option<bool>,
) {
    let _ = crate::pipeline::compute_normalized(
        record,
        &record.sql,
        state.params_buffer,
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
    record: &Sqllog,
    exporter_manager: &mut ExporterManager,
    state: &mut LoopState<'_>,
    remaining: Option<usize>,
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
            placeholder,
            state.ns_scratch,
        )
    } else {
        None
    };
    if let Some(remaining) = remaining {
        if state.records_in_file >= remaining {
            return ExportAction::BreakQuota;
        }
    }
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
    if args.show_progress {
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
pub(super) async fn process_log_file(
    exporter_manager: &mut ExporterManager,
    args: &ProcessArgs<'_>,
    params_buffer: &mut ParamBuffer,
    ns_scratch: &mut Vec<u8>,
    interrupted: &Arc<AtomicBool>,
) -> Result<(usize, ErrorStats)> {
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
    setup_progress_bar(
        args.pb,
        args.reset_pb,
        args.show_progress,
        args.file_index,
        args.total_files,
        &file_name,
    );
    let records = match open_log_file(std::path::Path::new(args.file_path)) {
        Ok(it) => it,
        Err(e) => {
            log::warn!("parse failed for '{}': {e}", args.file_path);
            let mut file_stats = ErrorStats::default();
            file_stats.add_parse_error();
            return Ok((0, file_stats));
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
        let action = normalize_and_export(
            &env,
            &record,
            exporter_manager,
            &mut state,
            args.remaining,
            passes,
        );
        total_processed = total_processed.wrapping_add(1);
        match action {
            ExportAction::BreakQuota | ExportAction::BreakFatal => break 'outer,
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
    Ok((state.records_in_file, state.file_stats))
}

// ===== 并行路径共享的记录迭代（STRUCT-04）=====

/// 迭代流式解析出的记录，对每条记录执行过滤、归一化，并通过 `on_pass` 回调写出通过过滤的记录。
///
/// - `records`：流式记录迭代器（[`crate::streaming::open_log_file`]），逐条产出
///   `Result<Sqllog, ParseError>`；单条记录解析失败会被跳过并计入 `file_stats`，不影响同文件
///   其余记录的处理（与旧的 `AsyncLogParser::parse()` 整文件 all-or-nothing 语义不同）。
/// - `pipeline`：过滤与处理管道
/// - `do_normalize`：是否启用 SQL 归一化
/// - `placeholder_override`：参数占位符覆盖配置
/// - `interrupted`：中断信号（Ctrl+C）
/// - `file_stats`：文件级错误统计，函数内累加 `filtered_out` 与逐条解析错误
/// - `on_pass`：通过过滤时的写出回调，接收 `(&Sqllog, Option<&str>)`（记录与归一化 SQL）
///
/// 返回成功写出的记录数。
pub(super) fn iterate_records<I, F>(
    records: I,
    pipeline: &Pipeline,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    interrupted: &Arc<AtomicBool>,
    file_stats: &mut ErrorStats,
    mut on_pass: F,
) -> Result<usize>
where
    I: IntoIterator<Item = std::result::Result<Sqllog, ParseError>>,
    F: FnMut(&Sqllog, Option<&str>) -> Result<()>,
{
    let mut params_buf = ParamBuffer::default();
    let mut ns_scratch: Vec<u8> = Vec::with_capacity(4096);
    let mut count = 0usize;

    for result in records {
        if interrupted.load(Ordering::Acquire) {
            break;
        }
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                log::warn!("skipping malformed record: {e}");
                file_stats.add_parse_error();
                continue;
            }
        };

        let passes = pipeline.is_empty() || pipeline.run_with_meta(&record);
        let needs_processing = passes || (do_normalize && record.tag.is_none());
        if !needs_processing {
            // 与 process_log_file 保持一致：!needs_processing 路径不计入 filtered_out。
            // filtered_out 只在 needs_processing=true && !passes 的分支（被过滤的 PARAMS 记录）累加。
            continue;
        }

        if passes {
            let normalized = if do_normalize && (!params_buf.is_empty() || record.tag.is_none()) {
                crate::pipeline::compute_normalized(
                    &record,
                    &record.sql,
                    &mut params_buf,
                    placeholder_override,
                    &mut ns_scratch,
                )
                .map(str::to_owned)
            } else {
                None
            };
            on_pass(&record, normalized.as_deref())?;
            count += 1;
        } else {
            file_stats.filtered_out += 1;
            crate::pipeline::compute_normalized(
                &record,
                &record.sql,
                &mut params_buf,
                placeholder_override,
                &mut ns_scratch,
            );
        }
    }

    Ok(count)
}
