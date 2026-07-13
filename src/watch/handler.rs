//! Watch 主入口：收集监听目录，初始化 watcher，进入 watch loop，Ctrl+C 后打印摘要。

use super::events::{create_watcher, handle_event};
use super::offsets;
use super::state::{
    STATUS_REFRESH_INTERVAL, WatchLoopState, WatchRun, build_progress_bar, print_final_summary,
    refresh_active_status,
};
use crate::config::Config;
use crate::error::{Error, Result};
use indicatif::ProgressBar;
use log::warn;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

/// 从 cfg.sqllog.inputs 收集实际存在的监听目录，去重后返回。
/// 路径经 canonicalize 处理以解决 macOS /var → /private/var 等符号链接问题。
#[must_use]
pub(super) fn collect_watch_dirs(inputs: &[String]) -> Vec<PathBuf> {
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

/// Watch 主入口：初始化 notify watcher，进入 watch loop，Ctrl+C 后打印摘要。
///
/// # Errors
///
/// 无可监听目录、notify watcher 创建/订阅失败，或收到中断信号
/// （返回 [`Error::Interrupted`]）时返回错误。
pub async fn run(
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<AtomicBool>,
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
    let env = WatchRun {
        cfg,
        quiet,
        verbose,
        interrupted,
        pb: &pb,
    };
    run_watch_loop(&env, &rx, &watch_dirs, &mut state).await;
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

/// Watch 主循环：接收 notify 事件并分发，在 Timeout 分支节流刷新状态行。
async fn run_watch_loop(
    env: &WatchRun<'_>,
    rx: &Receiver<notify::Result<notify::Event>>,
    watch_dirs: &[PathBuf],
    state: &mut WatchLoopState,
) {
    loop {
        // block_in_place is acceptable here: the 100ms timeout bounds each individual
        // blocking call to at most 100ms, keeping the block duration short per the
        // tokio guidance for block_in_place. The watch loop is the only long-running
        // consumer on this tokio worker thread and does not compete with other async tasks.
        let recv_result =
            tokio::task::block_in_place(|| rx.recv_timeout(Duration::from_millis(100)));
        match recv_result {
            Ok(Ok(event)) => {
                handle_event(env, &event, state).await;
            }
            Ok(Err(e)) => warn!("notify error: {e}"),
            Err(RecvTimeoutError::Timeout) => maybe_refresh_status(
                env.pb,
                watch_dirs,
                state.trigger_count,
                state.total_stats.records_exported,
                state.last_trigger_at,
                &mut state.last_status_refresh,
            ),
            Err(RecvTimeoutError::Disconnected) => break,
        }
        if env.interrupted.load(Ordering::Acquire) {
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
