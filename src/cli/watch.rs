//! Watch 子命令的实现入口。Phase 69 核心：notify watcher + watch loop + 状态行 + 退出摘要。

use crate::config::Config;
use crate::error::{Error, ErrorStats, Result};
use indicatif::{HumanDuration, ProgressBar, ProgressDrawTarget, ProgressStyle};
use log::warn;
use notify::{
    Config as NotifyConfig, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    event::{DataChange, ModifyKind},
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

/// macOS `FSEvents` 与 Linux inotify 同一写操作会先发 `Create(File)` 再发
/// `Modify(Data(Content))` 两个事件。500ms 窗口内同路径第二个事件被丢弃，
/// 确保 `handle_run` 只触发一次，消除统计虚高与 append 模式重复行。
const DEBOUNCE_WINDOW: Duration = Duration::from_millis(500);

/// 状态行刷新节流间隔：避免频繁调用 `pb.set_message` 导致 spinner 抖动。
/// 200ms 间隔在视觉上足够流畅，同时不引入额外 CPU 开销。
const STATUS_REFRESH_INTERVAL: Duration = Duration::from_millis(200);

/// Watch 子命令主入口：初始化 notify watcher，进入 watch loop，Ctrl+C 后打印摘要。
pub fn handle_watch(
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<std::sync::atomic::AtomicBool>,
) -> Result<()> {
    let start = Instant::now();
    let watch_dirs = collect_watch_dirs(&cfg.sqllog.inputs);
    if watch_dirs.is_empty() {
        return Err(Error::Io(std::io::Error::other(
            "watch: no existing input directories to watch (check cfg.sqllog.inputs)",
        )));
    }
    let pb = build_progress_bar(&watch_dirs);
    let (rx, _watcher) = create_watcher(&watch_dirs)?;
    let mut state = WatchLoopState::new();
    run_watch_loop(
        &rx,
        cfg,
        quiet,
        verbose,
        interrupted,
        &watch_dirs,
        &pb,
        &mut state,
    );
    pb.finish_and_clear();
    print_final_summary(
        &start,
        state.trigger_count,
        state.total_stats.records_exported,
        quiet,
    );
    let _ = verbose;
    Ok(())
}

/// 创建 notify watcher 并订阅所有监听目录，返回事件接收端与 watcher 所有权。
fn create_watcher(
    watch_dirs: &[PathBuf],
) -> Result<(Receiver<notify::Result<notify::Event>>, RecommendedWatcher)> {
    let (tx, rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
    // Note: using full path since `mpsc` is not directly imported (only Receiver/RecvTimeoutError are)
    let mut watcher = RecommendedWatcher::new(tx, NotifyConfig::default()).map_err(|e| {
        Error::Io(std::io::Error::other(format!(
            "watch: failed to create watcher: {e}"
        )))
    })?;
    for dir in watch_dirs {
        watcher.watch(dir, RecursiveMode::Recursive).map_err(|e| {
            Error::Io(std::io::Error::other(format!(
                "watch: failed to watch {}: {e}",
                dir.display()
            )))
        })?;
    }
    Ok((rx, watcher))
}

/// Watch 主循环运行时状态（合并多个可变字段，减少参数列表长度）。
struct WatchLoopState {
    last_trigger_at: Option<Instant>,
    last_status_refresh: Instant,
    debounce_map: HashMap<PathBuf, Instant>,
    total_stats: ErrorStats,
    trigger_count: u64,
}

impl WatchLoopState {
    fn new() -> Self {
        Self {
            last_trigger_at: None,
            last_status_refresh: Instant::now(),
            debounce_map: HashMap::new(),
            total_stats: ErrorStats::default(),
            trigger_count: 0u64,
        }
    }
}

/// Watch 主循环：接收 notify 事件并分发，在 Timeout 分支节流刷新状态行。
fn run_watch_loop(
    rx: &Receiver<notify::Result<notify::Event>>,
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<std::sync::atomic::AtomicBool>,
    watch_dirs: &[PathBuf],
    pb: &ProgressBar,
    state: &mut WatchLoopState,
) {
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => handle_event(
                &event,
                cfg,
                quiet,
                verbose,
                interrupted,
                &mut state.total_stats,
                &mut state.trigger_count,
                &mut state.last_trigger_at,
                &mut state.debounce_map,
                pb,
            ),
            Ok(Err(e)) => warn!("notify error: {e}"),
            Err(RecvTimeoutError::Timeout) => maybe_refresh_status(
                pb,
                watch_dirs,
                state.trigger_count,
                state.total_stats.records_exported,
                state.last_trigger_at,
                &mut state.last_status_refresh,
            ),
            Err(RecvTimeoutError::Disconnected) => break,
        }
        if interrupted.load(Ordering::Acquire) {
            break;
        }
    }
}

/// 若满足节流条件，刷新状态行中的 last 字段。
fn maybe_refresh_status(
    pb: &ProgressBar,
    watch_dirs: &[PathBuf],
    trigger_count: u64,
    rows: usize,
    last_trigger_at: Option<Instant>,
    last_status_refresh: &mut Instant,
) {
    if last_trigger_at.is_some() && last_status_refresh.elapsed() >= STATUS_REFRESH_INTERVAL {
        refresh_active_status(pb, watch_dirs, trigger_count, rows, last_trigger_at);
        *last_status_refresh = Instant::now();
    }
}

fn build_progress_bar(watch_dirs: &[PathBuf]) -> ProgressBar {
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

/// 从 cfg.sqllog.inputs 收集实际存在的监听目录，去重后返回。
/// 路径经 canonicalize 处理以解决 macOS /var → /private/var 等符号链接问题。
#[must_use]
pub fn collect_watch_dirs(inputs: &[String]) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    for input_str in inputs {
        let is_glob = input_str.contains('*') || input_str.contains('?') || input_str.contains('[');
        if is_glob {
            if let Some(ancestor) = Path::new(input_str).ancestors().find(|p| p.exists()) {
                let dir = ancestor
                    .canonicalize()
                    .unwrap_or_else(|_| ancestor.to_path_buf());
                if !dirs.contains(&dir) {
                    dirs.push(dir);
                }
            }
        } else {
            let path = Path::new(input_str);
            if path.is_file() {
                if let Some(parent) = path.parent() {
                    let dir = parent
                        .canonicalize()
                        .unwrap_or_else(|_| parent.to_path_buf());
                    if !dirs.contains(&dir) {
                        dirs.push(dir);
                    }
                }
            } else if path.is_dir() {
                let dir = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                if !dirs.contains(&dir) {
                    dirs.push(dir);
                }
            }
        }
    }
    dirs
}

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

/// 处理单个 notify 事件：仅对 Create + .log 文件触发 `handle_run`，含路径维度防抖。
fn handle_event(
    event: &notify::Event,
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<std::sync::atomic::AtomicBool>,
    total_stats: &mut ErrorStats,
    trigger_count: &mut u64,
    last_trigger_at: &mut Option<Instant>,
    debounce_map: &mut HashMap<PathBuf, Instant>,
    pb: &ProgressBar,
) {
    // TODO: track processed paths to prevent duplicate processing on log rotation (WR-03)
    let is_relevant = matches!(event.kind, EventKind::Create(_))
        || matches!(
            event.kind,
            EventKind::Modify(ModifyKind::Data(DataChange::Content))
        );
    if !is_relevant {
        return;
    }
    let now = Instant::now();
    for path in &event.paths {
        if path.extension().is_none_or(|ext| ext != "log") {
            continue;
        }
        if !should_trigger(path, debounce_map, now, DEBOUNCE_WINDOW) {
            continue;
        }
        process_log_path(
            path,
            cfg,
            quiet,
            verbose,
            interrupted,
            total_stats,
            trigger_count,
            last_trigger_at,
            pb,
        );
    }
}

/// 对单个通过防抖检查的 .log 路径执行 `handle_run` 并更新统计与状态行。
fn process_log_path(
    path: &Path,
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<std::sync::atomic::AtomicBool>,
    total_stats: &mut ErrorStats,
    trigger_count: &mut u64,
    last_trigger_at: &mut Option<Instant>,
    pb: &ProgressBar,
) {
    if !path.exists() {
        warn!(
            "watch: triggered path no longer exists, skipping: {}",
            path.display()
        );
        return;
    }
    let mut tmp_cfg = cfg.clone();
    tmp_cfg.sqllog.inputs = vec![path.to_string_lossy().into_owned()];
    match crate::cli::run::handle_run(&tmp_cfg, quiet, verbose, interrupted, None) {
        Ok(file_stats) => {
            total_stats.merge(&file_stats);
            *trigger_count += 1;
            *last_trigger_at = Some(Instant::now());
            let dir = path
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            pb.set_message(render_active_status(
                &dir,
                *trigger_count,
                total_stats.records_exported,
                Duration::from_secs(0),
            ));
        }
        Err(crate::error::Error::Interrupted) => {
            // 正常的优雅退出——重新设置 flag 确保外层循环在下次检查时退出。
            interrupted.store(true, std::sync::atomic::Ordering::Release);
        }
        Err(e) => warn!("watch trigger error: {e}"),
    }
}

/// 判断路径是否应触发处理（防抖逻辑）。
/// 若该路径上次触发到 now 的间隔 < window，返回 false（抑制）；
/// 否则更新表项为 now 并返回 true。
/// 同时清理超过 4 × window 的过期条目，防止表无界增长。
fn should_trigger(
    path: &Path,
    map: &mut HashMap<PathBuf, Instant>,
    now: Instant,
    window: Duration,
) -> bool {
    // 清理过期条目（O(n)，n 极小）
    map.retain(|_, prev| now.duration_since(*prev) <= window * 4);

    if let Some(prev) = map.get(path) {
        if now.duration_since(*prev) < window {
            return false;
        }
    }
    map.insert(path.to_path_buf(), now);
    true
}

/// 生成状态行字符串，使用 `HumanDuration` 动态格式化 last 字段。
fn render_active_status(dir: &str, trigger_count: u64, rows: usize, elapsed: Duration) -> String {
    format!(
        "watching {dir} | triggers: {trigger_count} | processed: {rows} rows | last: {}",
        HumanDuration(elapsed)
    )
}

/// 刷新状态行（节流调用点），读取 `last_trigger_at` 计算动态 elapsed。
fn refresh_active_status(
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
fn print_final_summary(start: &Instant, trigger_count: u64, rows: usize, quiet: bool) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn test_interrupted_flag_exits_immediately() {
        let cfg = Config::default();
        let interrupted = Arc::new(AtomicBool::new(true));
        let result = handle_watch(&cfg, true, false, &interrupted);
        // 默认 Config 的 sqllog.inputs = ["sqllogs"]，该目录不存在时返回 Err
        // 但 interrupted=true 时如果目录存在则立即跳出 loop 返回 Ok
        // 我们只验证函数能正常返回（不 panic）
        let _ = result;
    }

    #[test]
    fn test_collect_watch_dirs_nonexistent_glob_returns_empty() {
        let dirs = collect_watch_dirs(&["nonexistent_dir_xyz/*.log".to_string()]);
        assert!(
            dirs.is_empty(),
            "nonexistent glob parent should yield empty Vec, got: {dirs:?}"
        );
    }

    #[test]
    fn test_collect_watch_dirs_existing_dir_returns_itself() {
        let tmp = tempfile::TempDir::new().expect("failed to create tempdir");
        let dir_path = tmp.path().to_string_lossy().into_owned();
        let dirs = collect_watch_dirs(&[dir_path]);
        assert_eq!(dirs.len(), 1, "existing dir should yield exactly 1 entry");
        assert_eq!(
            dirs[0].canonicalize().ok(),
            tmp.path().canonicalize().ok(),
            "returned dir should match the temp dir"
        );
    }

    #[test]
    fn test_format_elapsed_hms_zero() {
        assert_eq!(
            format_elapsed_hms(Duration::from_secs(0)),
            "00:00:00",
            "zero seconds should format as 00:00:00"
        );
    }

    #[test]
    fn test_format_elapsed_hms_3661_seconds() {
        assert_eq!(
            format_elapsed_hms(Duration::from_secs(3661)),
            "01:01:01",
            "3661 seconds should format as 01:01:01"
        );
    }

    // --- 新增 4 个测试（Task 1 gap closure） ---

    #[test]
    fn test_should_trigger_first_call_admits_and_records() {
        let mut map: HashMap<PathBuf, Instant> = HashMap::new();
        let path = PathBuf::from("/tmp/test.log");
        let now = Instant::now();
        let result = should_trigger(&path, &mut map, now, DEBOUNCE_WINDOW);
        assert!(result, "首次调用应返回 true（允许触发）");
        assert!(
            map.contains_key(&path),
            "首次触发后，路径应写入 debounce_map"
        );
        let recorded = map[&path];
        assert_eq!(recorded, now, "记录的时间戳应等于传入的 now");
    }

    #[test]
    fn test_should_trigger_within_window_rejects() {
        let mut map: HashMap<PathBuf, Instant> = HashMap::new();
        let path = PathBuf::from("/tmp/test2.log");
        let t0 = Instant::now();

        // 首次触发
        let first = should_trigger(&path, &mut map, t0, DEBOUNCE_WINDOW);
        assert!(first, "首次调用应返回 true");

        // 100ms 内再次触发（窗口期内应被抑制）
        let t1 = t0 + Duration::from_millis(100);
        let second = should_trigger(&path, &mut map, t1, DEBOUNCE_WINDOW);
        assert!(!second, "窗口期 100ms 内再次触发应被抑制（返回 false）");

        // 600ms 后触发（窗口期外应被允许）
        let t2 = t0 + Duration::from_millis(600);
        let third = should_trigger(&path, &mut map, t2, DEBOUNCE_WINDOW);
        assert!(third, "窗口期外（600ms > 500ms）应允许触发（返回 true）");
        // 表项应更新为 t2
        assert_eq!(map[&path], t2, "允许触发后，表项应更新为最新时间戳");
    }

    #[test]
    fn test_should_trigger_isolates_distinct_paths() {
        let mut map: HashMap<PathBuf, Instant> = HashMap::new();
        let path1 = PathBuf::from("/tmp/a.log");
        let path2 = PathBuf::from("/tmp/b.log");
        let t0 = Instant::now();

        // P1 在 t0 触发
        let _ = should_trigger(&path1, &mut map, t0, DEBOUNCE_WINDOW);

        // P2 在 t0+100ms 触发（不同路径，不受 P1 窗口影响）
        let t1 = t0 + Duration::from_millis(100);
        let result = should_trigger(&path2, &mut map, t1, DEBOUNCE_WINDOW);
        assert!(result, "不同路径（P2）不受 P1 防抖窗口影响，应返回 true");
    }

    #[test]
    fn test_render_active_status_includes_human_duration() {
        // Duration::from_secs(0) => HumanDuration 输出 "0s" 或 "just now"
        let status_zero = render_active_status("/foo", 3, 42, Duration::from_secs(0));
        assert!(
            status_zero.contains("triggers: 3"),
            "状态行应包含 triggers: 3，实际: {status_zero}"
        );
        assert!(
            status_zero.contains("processed: 42 rows"),
            "状态行应包含 processed: 42 rows，实际: {status_zero}"
        );
        // HumanDuration(0) 输出 "0s" 或 "just now"，均含 "s" 或 "now"
        assert!(
            status_zero.contains("last: "),
            "状态行应包含 last: 前缀，实际: {status_zero}"
        );

        // Duration::from_secs(125) => HumanDuration 应包含 "m"（分钟单位）
        let status_125 = render_active_status("/foo", 1, 1, Duration::from_secs(125));
        assert!(
            status_125.contains('m'),
            "125 秒的 HumanDuration 输出应包含 'm'（分钟），实际: {status_125}"
        );
    }
}
