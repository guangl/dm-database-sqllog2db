//! Watch 事件源与路由：notify watcher 创建、事件分发（Create → 全量 / Modify → 增量）、
//! 以及同路径事件的防抖抑制。

use super::state::{DEBOUNCE_WINDOW, WatchLoopState, WatchRun};
use super::trigger::{trigger_full_file, trigger_incremental};
use crate::error::{Error, Result};
use notify::{
    Config as NotifyConfig, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    event::{DataChange, ModifyKind},
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

/// 创建 notify watcher 并订阅所有监听目录，返回事件接收端与 watcher 所有权。
pub(super) fn create_watcher(
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

/// 处理单个 notify 事件：按 `EventKind` 路由到 `trigger_full_file` 或 `trigger_incremental`。
pub(super) async fn handle_event(
    env: &WatchRun<'_>,
    event: &notify::Event,
    state: &mut WatchLoopState,
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
            trigger_full_file(path, env.cfg, env.quiet, env.verbose, env.interrupted, state, env.pb)
                .await;
        } else {
            trigger_incremental(
                path,
                env.cfg,
                env.quiet,
                env.verbose,
                env.interrupted,
                state,
                env.pb,
            )
            .await;
        }
    }
}

/// 判断路径是否应触发处理（防抖逻辑）。
/// 若该路径上次触发到 now 的间隔 < window，返回 false（抑制）；
/// 否则更新表项为 now 并返回 true。
/// 同时清理超过 4 × window 的过期条目，防止表无界增长。
pub(super) fn should_trigger(
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
