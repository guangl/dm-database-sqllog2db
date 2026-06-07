//! Watch 主循环运行时状态（`WatchLoopState`）及两个节流/防抖常量。

use crate::error::ErrorStats;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// macOS `FSEvents` 与 Linux inotify 同一写操作会先发 `Create(File)` 再发
/// `Modify(Data(Content))` 两个事件。500ms 窗口内同路径第二个事件被丢弃，
/// 确保 `handle_run` 只触发一次，消除统计虚高与 append 模式重复行。
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
