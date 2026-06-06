//! Watch 子命令的实现入口。Phase 69 核心：notify watcher + watch loop + 状态行 + 退出摘要。

pub(super) mod offsets;

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
    let sqlite_db_url: Option<String> =
        cfg.exporter.sqlite.as_ref().map(|s| s.database_url.clone());
    let init_offsets = if let Some(ref database_url) = sqlite_db_url {
        if let Err(e) = offsets::ensure_offset_table(database_url) {
            warn!("watch: ensure_offset_table failed: {e}");
        }
        offsets::load_offsets(database_url)
    } else {
        HashMap::new()
    };
    let mut state = WatchLoopState::new(init_offsets, sqlite_db_url);
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
        state.trigger_count(),
        state.total_stats().records_exported,
        quiet,
    );
    // WATCH-09 (D-07/D-08): 摘要打印后再检查中断标志，main.rs Err(Interrupted) 分支处理 exit(130)
    if interrupted.load(Ordering::Acquire) {
        return Err(Error::Interrupted);
    }
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
#[derive(Debug)]
pub struct WatchLoopState {
    last_trigger_at: Option<Instant>,
    last_status_refresh: Instant,
    debounce_map: HashMap<PathBuf, Instant>,
    total_stats: ErrorStats,
    trigger_count: u64,
    /// Phase 70 新增（per D-12）：路径→字节偏移映射，用于增量处理。
    file_offsets: HashMap<PathBuf, u64>,
    /// Phase 70 新增（per D-12）：SQLite 数据库 URL，`None` 表示未使用 `SqliteExporter`。
    sqlite_db_url: Option<String>,
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
            Ok(Ok(event)) => handle_event(&event, cfg, quiet, verbose, interrupted, state, pb),
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
    let mut seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    for input_str in inputs {
        let is_glob = input_str.contains('*') || input_str.contains('?') || input_str.contains('[');
        if is_glob {
            if let Some(ancestor) = Path::new(input_str).ancestors().find(|p| p.exists()) {
                let dir = ancestor
                    .canonicalize()
                    .unwrap_or_else(|_| ancestor.to_path_buf());
                if seen.insert(dir.clone()) {
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
                    if seen.insert(dir.clone()) {
                        dirs.push(dir);
                    }
                }
            } else if path.is_dir() {
                let dir = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                if seen.insert(dir.clone()) {
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

/// 处理单个 notify 事件：按 `EventKind` 路由到 `trigger_full_file` 或 `trigger_incremental`。
fn handle_event(
    event: &notify::Event,
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<std::sync::atomic::AtomicBool>,
    state: &mut WatchLoopState,
    pb: &ProgressBar,
) {
    let is_create = matches!(event.kind, EventKind::Create(_));
    let is_content_modify = matches!(
        event.kind,
        EventKind::Modify(ModifyKind::Data(DataChange::Content))
    );
    if !is_create && !is_content_modify {
        return;
    }
    let now = Instant::now();
    for path in &event.paths {
        if path.extension().is_none_or(|ext| ext != "log") {
            continue;
        }
        if !should_trigger(path, &mut state.debounce_map, now, DEBOUNCE_WINDOW) {
            continue;
        }
        if is_create {
            trigger_full_file(path, cfg, quiet, verbose, interrupted, state, pb);
        } else {
            trigger_incremental(path, cfg, quiet, verbose, interrupted, state, pb);
        }
    }
}

/// 对新创建的 .log 文件执行全量处理，并持久化文件大小为初始 offset（per D-03/D-10）。
pub fn trigger_full_file(
    path: &Path,
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<std::sync::atomic::AtomicBool>,
    state: &mut WatchLoopState,
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
    // WATCH-07 (D-01): 全量触发也强制 CSV 追加，避免每次触发覆盖既有数据
    // WATCH-08 (D-05): error log 追加模式，保留 watch 进程历史错误
    force_append_for_watch_trigger(&mut tmp_cfg);
    match crate::cli::run::handle_run(&tmp_cfg, quiet, verbose, interrupted, None) {
        Ok(file_stats) => {
            state.total_stats.merge(&file_stats);
            let last_elapsed = state.last_trigger_at.map(|t| t.elapsed());
            state.trigger_count += 1;
            state.last_trigger_at = Some(Instant::now());
            // handle_run 返回后 SqliteExporter 已 drop（per Pitfall 4），安全写 offset
            record_offset_after_trigger(path, state);
            update_status_bar(path, state, pb, last_elapsed);
        }
        Err(crate::error::Error::Interrupted) => interrupted.store(true, Ordering::Release),
        Err(e) => warn!("watch trigger error (full): {e}"),
    }
}

/// `handle_run` 成功后，持久化文件当前大小为新 offset（per D-08）。
/// 调用 `ensure_offset_table` 确保辅助表存在（幂等，per D-05），防止直接调用
/// `trigger_full_file` / `trigger_incremental`（绕过 `handle_watch` 启动逻辑）时表缺失。
fn record_offset_after_trigger(path: &Path, state: &mut WatchLoopState) {
    match std::fs::metadata(path) {
        Ok(meta) => {
            let new_size = meta.len();
            let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            state.file_offsets.insert(canonical_path.clone(), new_size);
            if let Some(ref database_url) = state.sqlite_db_url {
                if let Err(e) = offsets::ensure_offset_table(database_url) {
                    warn!("watch: ensure_offset_table in record_offset_after_trigger: {e}");
                }
                offsets::save_offset(database_url, &canonical_path, new_size);
            }
        }
        Err(e) => {
            // metadata 失败时保留旧 offset，避免重置为 0 导致下次全量重复导入
            warn!(
                "watch: metadata failed after trigger, offset not updated for {}: {e}",
                path.display()
            );
        }
    }
}

/// 用当前 `trigger_count` 和 `records_exported` 更新 progress bar 消息。
/// `last_elapsed`：上次触发到本次触发之间的间隔（`None` 表示首次触发，显示 0s）。
fn update_status_bar(
    path: &Path,
    state: &WatchLoopState,
    pb: &ProgressBar,
    last_elapsed: Option<Duration>,
) {
    let dir = path
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    pb.set_message(render_active_status(
        &dir,
        state.trigger_count,
        state.total_stats.records_exported,
        last_elapsed.unwrap_or_default(),
    ));
}

/// 对内容追加的 `.log` 文件执行增量处理，仅读取 `[start_offset, new_size)` 字节（per D-01/D-02）。
pub fn trigger_incremental(
    path: &Path,
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<std::sync::atomic::AtomicBool>,
    state: &mut WatchLoopState,
    pb: &ProgressBar,
) {
    if !path.exists() {
        warn!(
            "watch: triggered path no longer exists, skipping: {}",
            path.display()
        );
        return;
    }
    let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let Ok(metadata) = std::fs::metadata(path) else {
        warn!("watch: metadata error for {}", path.display());
        return;
    };
    let new_size = metadata.len();
    let Some(start_offset) = resolve_incremental_offset(&canonical_path, new_size, state) else {
        return; // 首次见到或无新字节——已处理基线，直接跳过
    };
    let Ok(tmp_file) = read_bytes_to_tempfile(path, start_offset) else {
        warn!("watch: read_bytes_to_tempfile error for {}", path.display());
        return;
    };
    run_incremental_handle_run(
        path,
        &canonical_path,
        tmp_file,
        new_size,
        cfg,
        quiet,
        verbose,
        interrupted,
        state,
        pb,
    );
}

/// 返回增量处理的 `start_offset`；`None` 表示应跳过（首次见到或无新字节）。
/// 首次见到时会将当前文件大小写入 `state.file_offsets` 作为基线（per D-04）。
fn resolve_incremental_offset(
    canonical_path: &Path,
    new_size: u64,
    state: &mut WatchLoopState,
) -> Option<u64> {
    let Some(&start_offset) = state.file_offsets.get(canonical_path) else {
        // D-04: 首次 Modify 无记录 → 记录基线 = 当前大小，跳过处理
        state
            .file_offsets
            .insert(canonical_path.to_path_buf(), new_size);
        if let Some(ref database_url) = state.sqlite_db_url {
            offsets::save_offset(database_url, canonical_path, new_size);
        }
        return None;
    };
    if new_size < start_offset {
        // 文件缩小——日志轮转（copytruncate）。重置 offset 从头处理，避免读取错误偏移内容。
        warn!(
            "watch: file shrank ({} → {} bytes), resetting offset for {}",
            start_offset,
            new_size,
            canonical_path.display()
        );
        state.file_offsets.insert(canonical_path.to_path_buf(), 0);
        if let Some(ref database_url) = state.sqlite_db_url {
            offsets::save_offset(database_url, canonical_path, 0);
        }
        return Some(0);
    }

    // D-02: 无新字节快速跳过
    if new_size == start_offset {
        return None;
    }

    Some(start_offset)
}

/// 从文件 `start_offset` 处读取剩余字节，写入 `NamedTempFile` 并返回（per D-01）。
pub(super) fn read_bytes_to_tempfile(
    path: &Path,
    start_offset: u64,
) -> std::io::Result<tempfile::NamedTempFile> {
    use std::io::{Read, Seek, SeekFrom, Write};
    let mut source_file = std::fs::File::open(path)?;
    source_file.seek(SeekFrom::Start(start_offset))?;
    let mut buffer = Vec::new();
    source_file.read_to_end(&mut buffer)?;
    let mut tmp_file = tempfile::Builder::new()
        .prefix("sqllog2db-watch-")
        .suffix(".log")
        .tempfile()?;
    tmp_file.write_all(&buffer)?;
    tmp_file.flush()?;
    Ok(tmp_file)
}

/// 使用临时文件路径调用 `handle_run`，更新 state，并持久化新 offset（per D-07/D-09）。
// tmp_file 按值传入确保函数返回时 NamedTempFile 自动删除临时文件
#[allow(clippy::needless_pass_by_value)]
fn run_incremental_handle_run(
    original_path: &Path,
    canonical_path: &Path,
    tmp_file: tempfile::NamedTempFile,
    new_size: u64,
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<std::sync::atomic::AtomicBool>,
    state: &mut WatchLoopState,
    pb: &ProgressBar,
) {
    let tmp_cfg = build_incremental_cfg(cfg, &tmp_file);
    match crate::cli::run::handle_run(&tmp_cfg, quiet, verbose, interrupted, None) {
        Ok(file_stats) => {
            state.total_stats.merge(&file_stats);
            let last_elapsed = state.last_trigger_at.map(|t| t.elapsed());
            state.trigger_count += 1;
            state.last_trigger_at = Some(Instant::now());
            // handle_run 返回后 SqliteExporter 已 drop（per Pitfall 4），安全写 offset
            state
                .file_offsets
                .insert(canonical_path.to_path_buf(), new_size);
            if let Some(ref database_url) = state.sqlite_db_url {
                offsets::save_offset(database_url, canonical_path, new_size);
            }
            update_status_bar(original_path, state, pb, last_elapsed);
        }
        Err(crate::error::Error::Interrupted) => interrupted.store(true, Ordering::Release),
        Err(e) => warn!("watch trigger error (incremental): {e}"),
    }
}

/// watch 触发前强制 `tmp_cfg` 进入追加模式（per D-01/D-05），适用全量与增量路径。
fn force_append_for_watch_trigger(cfg: &mut Config) {
    // WATCH-07 (D-01): CSV exporter 强制 append=true、overwrite=false
    if let Some(ref mut csv_cfg) = cfg.exporter.csv {
        csv_cfg.append = true;
        csv_cfg.overwrite = false;
    }
    // WATCH-07 (D-01): SQLite exporter 同样强制 append=true、overwrite=false，避免每次触发清空表
    if let Some(ref mut sqlite_cfg) = cfg.exporter.sqlite {
        sqlite_cfg.append = true;
        sqlite_cfg.overwrite = false;
    }
    // WATCH-08 (D-05): error log 追加模式，保留历史错误
    cfg.append_error_log = true;
}

/// 构造增量处理用的临时 `Config`：指向 `tmp_file` 路径，强制 append=true（per D-09）。
fn build_incremental_cfg(cfg: &Config, tmp_file: &tempfile::NamedTempFile) -> Config {
    let mut tmp_cfg = cfg.clone();
    tmp_cfg.sqllog.inputs = vec![tmp_file.path().to_string_lossy().into_owned()];
    // WATCH-07 (D-01): 增量路径强制 CSV + SQLite 追加；WATCH-08 (D-05): error log 追加模式
    force_append_for_watch_trigger(&mut tmp_cfg);
    tmp_cfg
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
    fn test_watch_loop_state_new_with_offsets() {
        let mut initial_offsets = HashMap::new();
        initial_offsets.insert(PathBuf::from("/tmp/test.log"), 1024u64);
        let state = WatchLoopState::new(initial_offsets.clone(), Some("test.db".to_string()));
        assert_eq!(
            state.file_offsets.get(&PathBuf::from("/tmp/test.log")),
            Some(&1024u64)
        );
        assert_eq!(state.sqlite_db_url, Some("test.db".to_string()));
        assert_eq!(state.trigger_count, 0);
    }

    #[test]
    fn test_watch_loop_state_new_without_sqlite() {
        let state = WatchLoopState::new(HashMap::new(), None);
        assert!(state.file_offsets.is_empty());
        assert!(state.sqlite_db_url.is_none());
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

    // --- Task 2: trigger_incremental + read_bytes_to_tempfile 测试 ---

    /// 测试 `read_bytes_to_tempfile`：从 `start_offset` 处读取，临时文件仅含新增字节。
    #[test]
    fn test_read_bytes_to_tempfile_reads_from_offset() {
        use std::io::Read;
        let mut source_file = tempfile::NamedTempFile::new().expect("failed to create tmp");
        let content = b"Hello World! This is a test file with 50 bytes content!";
        std::io::Write::write_all(&mut source_file, content).unwrap();
        let source_path = source_file.path().to_path_buf();
        let start_offset = 13u64; // skip "Hello World! "
        let tmp_result = read_bytes_to_tempfile(&source_path, start_offset)
            .expect("read_bytes_to_tempfile should succeed");
        let mut actual_content = Vec::new();
        std::fs::File::open(tmp_result.path())
            .unwrap()
            .read_to_end(&mut actual_content)
            .unwrap();
        let start_idx = usize::try_from(start_offset).expect("offset fits in usize");
        assert_eq!(
            actual_content,
            &content[start_idx..],
            "临时文件应仅包含 start_offset 之后的字节"
        );
    }

    /// 测试 `read_bytes_to_tempfile`：`start_offset` == 文件大小时，临时文件为空。
    #[test]
    fn test_read_bytes_to_tempfile_at_eof_produces_empty() {
        use std::io::Read;
        let mut source_file = tempfile::NamedTempFile::new().expect("failed to create tmp");
        let content = b"some content here";
        std::io::Write::write_all(&mut source_file, content).unwrap();
        let source_path = source_file.path().to_path_buf();
        let start_offset = content.len() as u64; // at EOF
        let tmp_result = read_bytes_to_tempfile(&source_path, start_offset)
            .expect("read_bytes_to_tempfile at EOF should succeed");
        let mut actual_content = Vec::new();
        std::fs::File::open(tmp_result.path())
            .unwrap()
            .read_to_end(&mut actual_content)
            .unwrap();
        assert!(
            actual_content.is_empty(),
            "offset == 文件大小时，临时文件应为空，实际: {actual_content:?}"
        );
    }

    /// 测试 `trigger_incremental`：`new_size` <= `start_offset` 时跳过，`trigger_count` 不增。
    #[test]
    fn test_trigger_incremental_skips_if_no_new_bytes() {
        let source_file = tempfile::NamedTempFile::new().expect("failed to create tmp");
        let content = vec![0u8; 100];
        std::fs::write(source_file.path(), &content).unwrap();
        let canonical_path = source_file.path().canonicalize().unwrap();
        let mut initial_offsets = HashMap::new();
        // file_offsets 记录 offset = 100（等于文件大小）
        initial_offsets.insert(canonical_path, 100u64);
        let mut state = WatchLoopState::new(initial_offsets, None);
        let cfg = Config::default();
        let interrupted = Arc::new(AtomicBool::new(false));
        let pb = indicatif::ProgressBar::hidden();
        trigger_incremental(
            source_file.path(),
            &cfg,
            true,
            false,
            &interrupted,
            &mut state,
            &pb,
        );
        assert_eq!(
            state.trigger_count, 0,
            "new_size == start_offset 时 trigger_count 不应增加"
        );
    }

    /// 测试 `trigger_incremental`：首次 Modify 无 `file_offsets` 记录时，记录基线并跳过处理。
    #[test]
    fn test_trigger_incremental_first_seen_records_baseline() {
        let source_file = tempfile::NamedTempFile::new().expect("failed to create tmp");
        let content = vec![0u8; 100];
        std::fs::write(source_file.path(), &content).unwrap();
        let canonical_path = source_file.path().canonicalize().unwrap();
        // file_offsets 中无该路径记录
        let mut state = WatchLoopState::new(HashMap::new(), None);
        let cfg = Config::default();
        let interrupted = Arc::new(AtomicBool::new(false));
        let pb = indicatif::ProgressBar::hidden();
        trigger_incremental(
            source_file.path(),
            &cfg,
            true,
            false,
            &interrupted,
            &mut state,
            &pb,
        );
        // 应记录基线 offset = 文件大小（100），但不处理
        assert_eq!(
            state.trigger_count, 0,
            "首次 Modify 应跳过处理，trigger_count 不增"
        );
        assert_eq!(
            state.file_offsets.get(&canonical_path),
            Some(&100u64),
            "首次 Modify 应将当前文件大小作为基线写入 file_offsets"
        );
    }

    // ── WATCH-07/08/09 Wave 0 Tests ─────────────────────────────────────────

    /// 一条合法的达梦 SQL 日志行（含 header + footer），用于 WATCH-07/08 测试。
    const DM_LOG_LINE_SEL: &str = "2024-01-15 10:00:00.123 (EP[0] sess:0x0001 thrd:456 user:SYSDBA trxid:1001 stmt:select_001 appname:test ip:127.0.0.1) [SEL] select * from t1. EXECTIME: 5(ms) ROWCOUNT: 10(rows) EXEC_ID: 1.\n";
    const DM_LOG_LINE_SEL2: &str = "2024-01-15 10:00:01.456 (EP[0] sess:0x0002 thrd:457 user:SYSDBA trxid:1002 stmt:select_002 appname:test ip:127.0.0.1) [SEL] select * from t2. EXECTIME: 3(ms) ROWCOUNT: 5(rows) EXEC_ID: 2.\n";
    const DM_LOG_LINE_GARBAGE: &str = "this is not a valid dm sql log line at all\n";

    // WATCH-07: CSV watch 追加，多次 trigger_full_file 后 CSV 行数累计，header 仅一行
    #[test]
    fn test_watch_csv_append() {
        let dir = tempfile::TempDir::new().unwrap();
        let log_a = dir.path().join("a.log");
        let log_b = dir.path().join("b.log");
        let csv_path = dir.path().join("out.csv");

        std::fs::write(&log_a, DM_LOG_LINE_SEL).unwrap();
        std::fs::write(&log_b, DM_LOG_LINE_SEL2).unwrap();

        // Config: CSV exporter，初始 append=false、overwrite=true（trigger_full_file 内部会注入 append=true）
        let toml = format!(
            "[sqllog]\ninputs = [\"{logdir}\"]\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\n",
            logdir = dir.path().to_string_lossy().replace('\\', "/"),
            csv = csv_path.to_string_lossy().replace('\\', "/"),
        );
        let cfg: crate::config::Config = toml::from_str(&toml).unwrap();
        let interrupted = Arc::new(AtomicBool::new(false));
        let pb = indicatif::ProgressBar::hidden();
        let mut state = WatchLoopState::new(HashMap::new(), None);

        trigger_full_file(&log_a, &cfg, true, false, &interrupted, &mut state, &pb);
        trigger_full_file(&log_b, &cfg, true, false, &interrupted, &mut state, &pb);

        assert!(csv_path.exists(), "CSV 文件应在触发后存在");
        let content = std::fs::read_to_string(&csv_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        // 应有 1 header + 2 data rows（来自 A 和 B 各 1 条）
        assert!(
            lines.len() >= 3,
            "CSV 应有 header + 2 data rows，实际: {} 行，内容:\n{content}",
            lines.len()
        );
        // header 只出现一次（append 模式不重复写 header）
        let header = lines[0];
        let header_count = lines.iter().filter(|&&l| l == header).count();
        assert_eq!(
            header_count, 1,
            "header 行应只出现一次，实际出现 {header_count} 次"
        );
    }

    // WATCH-08: error log 追加，两次 trigger_full_file 后 error log 含所有历史错误
    #[test]
    fn test_watch_error_log_append() {
        let dir = tempfile::TempDir::new().unwrap();
        let log_a = dir.path().join("a.log");
        let log_b = dir.path().join("b.log");
        let csv_path = dir.path().join("out.csv");
        let error_log_path = dir.path().join("errors.log");

        // 每个日志文件都有一条解析失败行（触发 write_error_log）+ 一条合法行（保证 handle_run 不提前失败）
        let content_a = format!("{DM_LOG_LINE_GARBAGE}{DM_LOG_LINE_SEL}");
        let content_b = format!("{DM_LOG_LINE_GARBAGE}{DM_LOG_LINE_SEL2}");
        std::fs::write(&log_a, &content_a).unwrap();
        std::fs::write(&log_b, &content_b).unwrap();

        let toml = format!(
            "[sqllog]\ninputs = [\"{logdir}\"]\n[error]\nfile = \"{errlog}\"\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\n",
            logdir = dir.path().to_string_lossy().replace('\\', "/"),
            errlog = error_log_path.to_string_lossy().replace('\\', "/"),
            csv = csv_path.to_string_lossy().replace('\\', "/"),
        );
        let cfg: crate::config::Config = toml::from_str(&toml).unwrap();
        let interrupted = Arc::new(AtomicBool::new(false));
        let pb = indicatif::ProgressBar::hidden();
        let mut state = WatchLoopState::new(HashMap::new(), None);

        trigger_full_file(&log_a, &cfg, true, false, &interrupted, &mut state, &pb);
        trigger_full_file(&log_b, &cfg, true, false, &interrupted, &mut state, &pb);

        assert!(error_log_path.exists(), "error log 应在有解析错误时创建");
        let error_content = std::fs::read_to_string(&error_log_path).unwrap();
        // 两次触发各有一条解析失败行，error log 应包含两条 [ERROR] 记录
        let error_line_count = error_content
            .lines()
            .filter(|l| l.starts_with("[ERROR]"))
            .count();
        assert!(
            error_line_count >= 2,
            "error log 应包含 2 条 [ERROR] 行（来自 A 和 B 各 1 次触发），实际: {error_line_count} 条\n内容:\n{error_content}"
        );
    }

    // WATCH-09: interrupted=true 时 handle_watch 返回 Err(Error::Interrupted)
    #[test]
    fn test_handle_watch_returns_interrupted() {
        let dir = tempfile::TempDir::new().unwrap();
        let csv_path = dir.path().join("out.csv");
        let toml = format!(
            "[sqllog]\ninputs = [\"{logdir}\"]\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\n",
            logdir = dir.path().to_string_lossy().replace('\\', "/"),
            csv = csv_path.to_string_lossy().replace('\\', "/"),
        );
        let cfg: crate::config::Config = toml::from_str(&toml).unwrap();
        let interrupted = Arc::new(AtomicBool::new(true));
        let result = handle_watch(&cfg, true, false, &interrupted);
        assert!(
            matches!(result, Err(Error::Interrupted)),
            "interrupted=true 时 handle_watch 应返回 Err(Error::Interrupted)，实际: {result:?}"
        );
    }
}
