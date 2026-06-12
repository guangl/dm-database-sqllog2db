//! 处理单个 notify 事件：按 `EventKind` 路由到 `trigger_full_file` 或 `trigger_incremental`。

use super::debounce::should_trigger;
use super::state::{DEBOUNCE_WINDOW, WatchLoopState};
use super::trigger_full::trigger_full_file;
use super::trigger_incremental::trigger_incremental;
use crate::config::Config;
use indicatif::ProgressBar;
use notify::{
    EventKind,
    event::{DataChange, ModifyKind},
};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

/// 处理单个 notify 事件：按 `EventKind` 路由到 `trigger_full_file` 或 `trigger_incremental`。
pub(super) async fn handle_event(
    event: &notify::Event,
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<AtomicBool>,
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
            trigger_full_file(path, cfg, quiet, verbose, interrupted, state, pb).await;
        } else {
            trigger_incremental(path, cfg, quiet, verbose, interrupted, state, pb).await;
        }
    }
}
