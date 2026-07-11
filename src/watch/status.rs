//! Watch 状态栏构建、刷新与最终摘要输出。

use super::dirs::format_paths_display;
use indicatif::{HumanDuration, ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// 构建并初始化 watch 状态行的 spinner。
pub(super) fn build_progress_bar(watch_dirs: &[PathBuf]) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_draw_target(ProgressDrawTarget::stderr());
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {wide_msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner())
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    let paths_display = format_paths_display(watch_dirs);
    pb.set_message(format!(
        "watching {paths_display} | waiting for new .log files..."
    ));
    pb
}

/// 生成状态行字符串，使用 `HumanDuration` 动态格式化 last 字段。
pub(super) fn render_active_status(
    dir: &str,
    trigger_count: u64,
    rows: usize,
    elapsed: Duration,
) -> String {
    format!(
        "watching {dir} | triggers: {trigger_count} | processed: {rows} rows | last: {}",
        HumanDuration(elapsed)
    )
}

/// 刷新状态行（节流调用点），读取 `last_trigger_at` 计算动态 elapsed。
pub(super) fn refresh_active_status(
    pb: &ProgressBar,
    watch_dirs: &[PathBuf],
    trigger_count: u64,
    rows: usize,
    last_trigger_at: Option<Instant>,
) {
    let Some(triggered_at) = last_trigger_at else {
        return; // safe-guard，调用方已检查
    };
    let dir_str = format_paths_display(watch_dirs);
    pb.set_message(render_active_status(
        &dir_str,
        trigger_count,
        rows,
        triggered_at.elapsed(),
    ));
}

/// 将 Duration 格式化为 hh:mm:ss 字符串。
#[must_use]
pub fn format_elapsed_hms(elapsed: Duration) -> String {
    let secs = elapsed.as_secs();
    format!(
        "{:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

/// 打印 watch 结束后的最终摘要（到 stderr）。quiet 模式下跳过。
pub(super) fn print_final_summary(start: &Instant, trigger_count: u64, rows: usize, quiet: bool) {
    if quiet {
        return;
    }
    eprintln!(
        "Watch stopped. Triggers: {}, total processed: {} rows, elapsed: {}",
        trigger_count,
        rows,
        format_elapsed_hms(start.elapsed())
    );
}
