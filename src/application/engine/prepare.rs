//! Run 前置准备：输入文件解析与 stdin pipe 检测、事务级过滤器预扫描（trxid 收集）、
//! 以及进度条构建。

use crate::config::Config;
use crate::error::Result;
use crate::input::InputResolver;
use crate::input::open_log_file;
use crate::pipeline::filters::transaction::TransactionFilters;
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn};
use std::io::IsTerminal;
use std::path::PathBuf;

// ===== 输入解析与进度条 =====

/// 解析输入文件列表并检测 stdin pipe 模式。
/// 返回 `(log_files, is_stdin_pipe)`。当无文件且非 Unix stdin pipe 时返回错误。
pub(super) fn resolve_input_files(cfg: &Config) -> Result<(Vec<PathBuf>, bool)> {
    let log_files = InputResolver::new(cfg.sqllog.inputs.clone()).log_files()?;
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
    is_stdin_pipe: bool,
    quiet: bool,
) -> Option<Config> {
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
            return None;
        }
        let matches = scan_for_trxids_by_transaction_filters(log_files, cfg);
        let mut tmp = cfg.clone();
        if let Some(f) = &mut tmp.filter {
            f.merge_found_trxids(matches.included.into_iter().collect());
            if let Some(ids) = &mut f.include.trxids {
                ids.retain(|id| !matches.excluded.contains(id));
            }
            // 第二遍只使用已计算的事务 ID；stdin 则保留原条件逐条匹配。
            f.include.clear_transaction_filters();
            f.exclude.clear_transaction_filters();
        }
        Some(tmp)
    } else {
        None
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

// ===== Pre-scan: 单文件扫描（文件内去重）=====

/// 预扫描选中的事务和被否决的事务；跨文件合并后统一应用否决。
#[derive(Debug, Default)]
pub(super) struct TransactionMatches {
    pub included: std::collections::HashSet<String>,
    pub excluded: std::collections::HashSet<String>,
}

/// 流式扫描单个文件，分别收集包含和排除条件命中的事务 ID。
/// 内存随唯一事务数量增长；解析失败的记录会跳过并记录警告。
pub(super) fn scan_log_file_for_matches(file_path: &str, cfg: &Config) -> TransactionMatches {
    let filters = match &cfg.filter {
        Some(f) if f.has_transaction_filters() => f,
        _ => return TransactionMatches::default(),
    };

    let records = match open_log_file(std::path::Path::new(file_path)) {
        Ok(it) => it,
        Err(e) => {
            log::warn!("Pre-scan: failed to parse '{file_path}': {e}");
            return TransactionMatches::default();
        }
    };

    let filters = TransactionFilters::new(filters);
    let mut matches = TransactionMatches::default();
    for result in records {
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                log::warn!("Pre-scan: skipping malformed record in '{file_path}': {e}");
                continue;
            }
        };
        if filters.includes(&record) {
            matches.included.insert(record.trxid.clone());
        }
        if filters.excludes(&record) {
            matches.excluded.insert(record.trxid);
        }
    }
    matches
}

// ===== Pre-scan: 跨文件编排（顺序扫描）=====

pub(super) fn scan_for_trxids_by_transaction_filters(
    log_files: &[std::path::PathBuf],
    cfg: &Config,
) -> TransactionMatches {
    log::info!(
        "Pre-scanning {} files for transaction-level filters...",
        log_files.len()
    );

    let mut matched = TransactionMatches::default();
    for file in log_files {
        let next = if let Some(path) = file.to_str() {
            scan_log_file_for_matches(path, cfg)
        } else {
            log::warn!(
                "Pre-scan: skipping file with non-UTF8 path: {}",
                file.display()
            );
            TransactionMatches::default()
        };
        matched.included.extend(next.included);
        matched.excluded.extend(next.excluded);
    }
    matched
}
