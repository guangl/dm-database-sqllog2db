//! 创建 notify watcher 并订阅所有监听目录。

use crate::error::{Error, Result};
use notify::{Config as NotifyConfig, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

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
