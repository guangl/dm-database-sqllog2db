//! Watch 防抖逻辑：同路径事件在窗口期内只触发一次。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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
