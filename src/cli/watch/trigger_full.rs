//! 全量触发：对新创建的 .log 文件执行完整的 handle_run，并持久化文件大小为初始 offset。

use super::append::force_append_for_watch_trigger;
use super::offsets;
use super::state::WatchLoopState;
use super::status::render_active_status;
use crate::config::Config;
use indicatif::ProgressBar;
use log::warn;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// 对新创建的 .log 文件执行全量处理，并持久化文件大小为初始 offset（per D-03/D-10）。
pub fn trigger_full_file(
    path: &Path,
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<AtomicBool>,
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
pub(super) fn update_status_bar(
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
