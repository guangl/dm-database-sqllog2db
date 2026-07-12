//! Watch 主循环运行时状态（`WatchLoopState`）、节流/防抖常量，以及状态行构建与刷新。

use crate::error::ErrorStats;
use indicatif::{HumanDuration, ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// macOS `FSEvents` 与 Linux inotify 同一写操作会先发 `Create(File)` 再发
/// `Modify(Data(Content))` 两个事件。500ms 窗口内同路径第二个事件被丢弃，
/// 确保 `engine::run` 只触发一次，消除统计虚高与 append 模式重复行。
pub(super) const DEBOUNCE_WINDOW: Duration = Duration::from_millis(500);

/// 状态行刷新节流间隔：避免频繁调用 `pb.set_message` 导致 spinner 抖动。
/// 200ms 间隔在视觉上足够流畅，同时不引入额外 CPU 开销。
pub(super) const STATUS_REFRESH_INTERVAL: Duration = Duration::from_millis(200);

/// Watch 主循环运行时状态（合并多个可变字段，减少参数列表长度）。
#[derive(Debug)]
pub struct WatchLoopState {
    pub(super) last_trigger_at: Option<Instant>,
    pub(super) last_status_refresh: Instant,
    pub(super) debounce_map: HashMap<PathBuf, Instant>,
    pub(super) total_stats: ErrorStats,
    pub(super) trigger_count: u64,
    /// Phase 70 新增（per D-12）：路径→字节偏移映射，用于增量处理。
    pub(super) file_offsets: HashMap<PathBuf, u64>,
    /// Phase 70 新增（per D-12）：SQLite 数据库 URL，`None` 表示未使用 `SqliteExporter`。
    pub(super) sqlite_db_url: Option<String>,
}

impl WatchLoopState {
    /// 构造 `WatchLoopState`，接受初始偏移映射与可选 `SQLite` 数据库 URL。
    #[must_use]
    pub fn new(init_offsets: HashMap<PathBuf, u64>, sqlite_db_url: Option<String>) -> Self {
        Self {
            last_trigger_at: None,
            last_status_refresh: Instant::now(),
            debounce_map: HashMap::new(),
            total_stats: ErrorStats::default(),
            trigger_count: 0u64,
            file_offsets: init_offsets,
            sqlite_db_url,
        }
    }

    /// 返回当前 `trigger_count`（全量 + 增量触发次数之和）。
    #[must_use]
    pub fn trigger_count(&self) -> u64 {
        self.trigger_count
    }

    /// 返回总错误统计。
    #[must_use]
    pub fn total_stats(&self) -> &ErrorStats {
        &self.total_stats
    }

    /// 返回路径→字节偏移映射（用于集成测试验证 offset 持久化）。
    #[must_use]
    #[allow(dead_code)]
    pub fn file_offsets(&self) -> &HashMap<PathBuf, u64> {
        &self.file_offsets
    }
}

// ===== 状态行构建与刷新 =====

/// 格式化监听目录列表用于状态行显示。
fn format_paths_display(dirs: &[PathBuf]) -> String {
    if dirs.len() > 3 {
        format!("{} directories", dirs.len())
    } else {
        dirs.iter()
            .map(|d| d.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

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
pub(super) fn format_elapsed_hms(elapsed: Duration) -> String {
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
