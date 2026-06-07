use super::prescan::scan_for_trxids_by_transaction_filters;
use crate::config::Config;
use crate::error::Result;
use crate::parser::SqllogParser;
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn};
use std::io::IsTerminal;
use std::path::PathBuf;

/// 解析输入文件列表并检测 stdin pipe 模式。
/// 返回 `(log_files, is_stdin_pipe)`。当无文件且非 Unix stdin pipe 时返回错误。
pub(super) fn resolve_input_files(cfg: &Config) -> Result<(Vec<PathBuf>, bool)> {
    let log_files = SqllogParser::new(cfg.sqllog.inputs.clone()).log_files()?;
    // Stdin pipe mode: fall back when no log files found AND stdin is not a terminal.
    // /dev/stdin is Unix-only; skip pipe mode on Windows.
    #[cfg(target_os = "windows")]
    let is_stdin_pipe = false;
    #[cfg(not(target_os = "windows"))]
    let is_stdin_pipe = log_files.is_empty() && !std::io::stdin().is_terminal();
    let log_files = if is_stdin_pipe {
        info!("No log files found, reading from stdin (pipe mode)");
        vec![PathBuf::from("/dev/stdin")]
    } else if log_files.is_empty() {
        // On Windows, if stdin is piped but no files found, warn the user that stdin
        // pipe mode is not supported on this platform.
        #[cfg(target_os = "windows")]
        if !std::io::stdin().is_terminal() {
            warn!("Stdin pipe mode is not supported on Windows. No log files found.");
        }
        return Err(crate::error::Error::Parser(
            crate::error::ParserError::NoFilesFound {
                inputs: cfg.sqllog.inputs.clone(),
            },
        ));
    } else {
        log_files
    };
    Ok((log_files, is_stdin_pipe))
}

/// 在有事务级过滤器时执行预扫描并合并 trxid，返回合并后的 Config。
/// `None` = 无需预扫描（无事务过滤器，或 stdin pipe 降级）；`Some` = 已合并 trxid 的新 Config。
pub(super) fn merge_trxid_prescan(
    cfg: &Config,
    log_files: &[PathBuf],
    jobs: usize,
    is_stdin_pipe: bool,
    quiet: bool,
) -> Result<Option<Config>> {
    if cfg
        .filter
        .as_ref()
        .is_some_and(crate::pipeline::FiltersFeature::has_transaction_filters)
    {
        if is_stdin_pipe {
            warn!(
                "Transaction-level filters are configured but stdin pipe mode \
                 cannot pre-scan for transaction IDs. Degrading to per-record matching \
                 (transaction integrity not guaranteed)."
            );
            if !quiet {
                eprintln!(
                    "[WARN] Transaction-level filters with stdin: pre-scan disabled, \
                     degrading to per-record matching."
                );
            }
            return Ok(None);
        }
        let extra_trxids = scan_for_trxids_by_transaction_filters(log_files, cfg, jobs)?;
        let mut tmp = cfg.clone();
        if let Some(f) = &mut tmp.filter {
            f.merge_found_trxids(extra_trxids);
        }
        Ok(Some(tmp))
    } else {
        Ok(None)
    }
}

/// 创建文件计数进度条，`show_progress` 为 false 时返回 `None`。
/// `total_files` 决定 `{pos}/{len}` 计数器的上限，ETA 由 indicatif 自动计算。
pub(super) fn make_progress_bar(show_progress: bool, total_files: usize) -> Option<ProgressBar> {
    if show_progress {
        let bar = ProgressBar::new(total_files as u64);
        bar.set_style(
            ProgressStyle::with_template("{spinner:.cyan} [{pos}/{len}] {wide_msg} | eta {eta}")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
        );
        bar.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(bar)
    } else {
        None
    }
}
