//! Export lifecycle: resolve inputs, prepare filters, execute and report.
mod context;
mod prepare;
mod record;
mod report;
mod sequential;

#[cfg(test)]
#[path = "../../tests/unit/engine/mod.rs"]
mod tests;

use self::context::{Console, build_run_context};
use self::prepare::{make_progress_bar, merge_trxid_prescan, resolve_input_files};
use self::report::{RunSummary, print_run_summary, write_error_log};
use self::sequential::run_sequential;
use crate::config::Config;
use crate::error::{Error, ErrorStats, Result};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// 主编排函数：解析日志文件并导出到配置的导出器。
/// 所有导出器共用顺序流式处理和进度展示。
///
/// # Errors
///
/// 未找到任何输入文件、导出器初始化/写出发生致命错误，或运行期间收到中断信号
/// （返回 [`Error::Interrupted`]）时返回错误。
pub fn run(
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<AtomicBool>,
) -> Result<ErrorStats> {
    let total_start = Instant::now();
    let mut run_stats = ErrorStats::default();
    let (log_files, is_stdin_pipe) = resolve_input_files(cfg)?;
    let merged = merge_trxid_prescan(cfg, &log_files, is_stdin_pipe, quiet);
    let final_cfg: &Config = merged.as_ref().unwrap_or(cfg);
    let ctx = build_run_context(final_cfg);
    let show_progress = !quiet;
    let pb = make_progress_bar(show_progress, log_files.len());
    let console = Console {
        quiet,
        verbose,
        pb: pb.as_ref(),
    };
    let (processed_files, stats) = run_sequential(&ctx, &log_files, &console, interrupted)?;
    run_stats.merge(&stats);
    let total_records: usize = processed_files.iter().map(|(_, c)| *c).sum();
    run_stats.records_exported = total_records;
    if let Some(pb) = &pb {
        pb.finish_and_clear();
    }
    print_run_summary(
        quiet,
        verbose,
        &RunSummary {
            elapsed: total_start.elapsed().as_secs_f64(),
            processed_files: &processed_files,
            total_records,
        },
        &run_stats,
    );
    write_error_log(final_cfg, &run_stats);
    if interrupted.load(Ordering::Acquire) {
        return Err(Error::Interrupted);
    }
    Ok(run_stats)
}
