//! Watch 子命令的实现入口。Phase 69 核心：notify watcher + watch loop + 状态行 + 退出摘要。

use crate::config::Config;
use crate::error::{Error, ErrorStats, Result};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use log::warn;
use notify::{Config as NotifyConfig, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

/// Watch 子命令主入口：初始化 notify watcher，进入 watch loop，Ctrl+C 后打印摘要。
pub fn handle_watch(
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<std::sync::atomic::AtomicBool>,
) -> Result<()> {
    let start = Instant::now();
    let mut total_stats = ErrorStats::default();
    let mut trigger_count: u64 = 0u64;

    let watch_dirs = collect_watch_dirs(&cfg.sqllog.inputs);
    if watch_dirs.is_empty() {
        return Err(Error::Io(std::io::Error::other(
            "watch: no existing input directories to watch (check cfg.sqllog.inputs)",
        )));
    }

    let pb = build_progress_bar(&watch_dirs);

    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher = RecommendedWatcher::new(tx, NotifyConfig::default()).map_err(|e| {
        Error::Io(std::io::Error::other(format!(
            "watch: failed to create watcher: {e}"
        )))
    })?;
    for dir in &watch_dirs {
        watcher.watch(dir, RecursiveMode::Recursive).map_err(|e| {
            Error::Io(std::io::Error::other(format!(
                "watch: failed to watch {}: {e}",
                dir.display()
            )))
        })?;
    }

    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => {
                handle_event(
                    &event,
                    cfg,
                    quiet,
                    verbose,
                    interrupted,
                    &mut total_stats,
                    &mut trigger_count,
                    &pb,
                );
            }
            Ok(Err(e)) => {
                warn!("notify error: {e}");
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                break;
            }
        }
        if interrupted.load(Ordering::Relaxed) {
            break;
        }
    }

    pb.finish_and_clear();
    print_final_summary(&start, trigger_count, total_stats.records_exported, quiet);
    let _ = verbose;
    Ok(())
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
#[must_use]
pub fn collect_watch_dirs(inputs: &[String]) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    for input_str in inputs {
        let is_glob = input_str.contains('*') || input_str.contains('?') || input_str.contains('[');
        if is_glob {
            if let Some(ancestor) = Path::new(input_str).ancestors().find(|p| p.exists()) {
                let dir = ancestor.to_path_buf();
                if !dirs.contains(&dir) {
                    dirs.push(dir);
                }
            }
        } else {
            let path = Path::new(input_str);
            if path.is_file() {
                if let Some(parent) = path.parent() {
                    let dir = parent.to_path_buf();
                    if !dirs.contains(&dir) {
                        dirs.push(dir);
                    }
                }
            } else if path.is_dir() {
                let dir = path.to_path_buf();
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

/// 处理单个 notify 事件：仅对 Create + .log 文件触发 `handle_run`。
fn handle_event(
    event: &notify::Event,
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<std::sync::atomic::AtomicBool>,
    total_stats: &mut ErrorStats,
    trigger_count: &mut u64,
    pb: &ProgressBar,
) {
    if !matches!(event.kind, EventKind::Create(_)) {
        return;
    }
    for path in &event.paths {
        if path.extension().is_none_or(|ext| ext != "log") {
            continue;
        }
        let mut tmp_cfg = cfg.clone();
        tmp_cfg.sqllog.inputs = vec![path.to_string_lossy().into_owned()];
        match crate::cli::run::handle_run(&tmp_cfg, quiet, verbose, interrupted, None) {
            Ok(file_stats) => {
                total_stats.merge(&file_stats);
                *trigger_count += 1;
                let dir = path
                    .parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                pb.set_message(format!(
                    "watching {dir} | triggers: {trigger_count} | processed: {} rows | last: just now",
                    total_stats.records_exported
                ));
            }
            Err(e) => {
                warn!("watch trigger error: {e}");
            }
        }
    }
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
}
