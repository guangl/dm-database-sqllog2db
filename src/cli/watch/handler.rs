//! Watch 子命令主入口：初始化 watcher，进入 watch loop，Ctrl+C 后打印摘要。

use super::dirs::collect_watch_dirs;
use super::event::handle_event;
use super::offsets;
use super::state::{STATUS_REFRESH_INTERVAL, WatchLoopState};
use super::status::{build_progress_bar, print_final_summary, refresh_active_status};
use super::watcher::create_watcher;
use crate::config::Config;
use crate::error::{Error, Result};
use indicatif::ProgressBar;
use log::warn;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

/// Watch 子命令主入口：初始化 notify watcher，进入 watch loop，Ctrl+C 后打印摘要。
pub fn handle_watch(
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

/// Watch 主循环：接收 notify 事件并分发，在 Timeout 分支节流刷新状态行。
fn run_watch_loop(
    rx: &Receiver<notify::Result<notify::Event>>,
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<AtomicBool>,
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
