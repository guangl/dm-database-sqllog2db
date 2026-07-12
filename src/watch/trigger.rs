//! Watch 触发路径：新建 `.log` 文件的全量处理与内容追加的增量处理，
//! 以及两条路径共享的 Config 追加模式注入与状态行更新。

use super::offsets;
use super::state::{WatchLoopState, render_active_status};
use crate::config::Config;
use indicatif::ProgressBar;
use log::warn;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

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

// ===== 全量触发 =====

/// 对新创建的 .log 文件执行全量处理，并持久化文件大小为初始 offset（per D-03/D-10）。
pub async fn trigger_full_file(
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
    match crate::engine::run(&tmp_cfg, quiet, verbose, interrupted, None).await {
        Ok(file_stats) => {
            state.total_stats.merge(&file_stats);
            let last_elapsed = state.last_trigger_at.map(|t| t.elapsed());
            state.trigger_count += 1;
            state.last_trigger_at = Some(Instant::now());
            // engine::run 返回后 SqliteExporter 已 drop（per Pitfall 4），安全写 offset
            record_offset_after_trigger(path, state);
            update_status_bar(path, state, pb, last_elapsed);
        }
        Err(crate::error::Error::Interrupted) => interrupted.store(true, Ordering::Release),
        Err(e) => warn!("watch trigger error (full): {e}"),
    }
}

/// `engine::run` 成功后，持久化文件当前大小为新 offset（per D-08）。
/// 调用 `ensure_offset_table` 确保辅助表存在（幂等，per D-05），防止直接调用
/// `trigger_full_file` / `trigger_incremental`（绕过 `watch::run` 启动逻辑）时表缺失。
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

// ===== 增量触发 =====

/// 对内容追加的 `.log` 文件执行增量处理，仅读取 `[start_offset, new_size)` 字节（per D-01/D-02）。
pub async fn trigger_incremental(
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
    run_incremental(
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
    )
    .await;
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

/// 使用临时文件路径调用 `engine::run`，更新 state，并持久化新 offset（per D-07/D-09）。
// tmp_file 按值传入确保函数返回时 NamedTempFile 自动删除临时文件
async fn run_incremental(
    original_path: &Path,
    canonical_path: &Path,
    tmp_file: tempfile::NamedTempFile,
    new_size: u64,
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<AtomicBool>,
    state: &mut WatchLoopState,
    pb: &ProgressBar,
) {
    let tmp_cfg = build_incremental_cfg(cfg, &tmp_file);
    match crate::engine::run(&tmp_cfg, quiet, verbose, interrupted, None).await {
        Ok(file_stats) => {
            state.total_stats.merge(&file_stats);
            let last_elapsed = state.last_trigger_at.map(|t| t.elapsed());
            state.trigger_count += 1;
            state.last_trigger_at = Some(Instant::now());
            // engine::run 返回后 SqliteExporter 已 drop（per Pitfall 4），安全写 offset
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

/// 构造增量处理用的临时 `Config`：指向 `tmp_file` 路径，强制 append=true（per D-09）。
fn build_incremental_cfg(cfg: &Config, tmp_file: &tempfile::NamedTempFile) -> Config {
    let mut tmp_cfg = cfg.clone();
    tmp_cfg.sqllog.inputs = vec![tmp_file.path().to_string_lossy().into_owned()];
    // WATCH-07 (D-01): 增量路径强制 CSV + SQLite 追加；WATCH-08 (D-05): error log 追加模式
    force_append_for_watch_trigger(&mut tmp_cfg);
    tmp_cfg
}
