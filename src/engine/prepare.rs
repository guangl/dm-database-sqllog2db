//! Run 前置准备：输入文件解析与 stdin pipe 检测、事务级过滤器预扫描（trxid 收集）、
//! 多文件并行的内存预算并发控制，以及进度条构建。

use crate::config::Config;
use crate::error::{Error, Result};
use crate::parser::SqllogParser;
use crate::pipeline::filters::{IndicatorFilters, SqlFilters};
use crate::streaming::open_log_file;
use dm_database_parser_sqllog::{Filter, FilterBuilder};
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

// ===== 输入解析与进度条 =====

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

// ===== Pre-scan: 指标/SQL 过滤器构建 =====

pub(super) fn build_indicator_filters(indicators: &IndicatorFilters) -> Vec<Filter> {
    let mut filters = Vec::new();
    if let Some(min_ms) = indicators.min_runtime_ms {
        filters.push(FilterBuilder::new().exec_time_gte(min_ms).build());
    }
    if let Some(min_r) = indicators.min_row_count {
        // rowcount >= min_r: for u32, rowcount_gt(min_r - 1) works when min_r > 0
        let filter = if min_r == 0 {
            FilterBuilder::new().build()
        } else {
            FilterBuilder::new().rowcount_gt(min_r - 1).build()
        };
        filters.push(filter);
    }
    if let Some(ids) = &indicators.exec_ids {
        for &id in ids {
            filters.push(FilterBuilder::new().exec_id_eq(id).build());
        }
    }
    filters
}

pub(super) fn build_sql_include_filters(sf: &SqlFilters) -> Vec<Filter> {
    sf.includes
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|p| FilterBuilder::new().sql_contains(p.clone()).build())
        .collect()
}

pub(super) fn build_sql_exclude_filters(sf: &SqlFilters) -> Vec<Filter> {
    sf.excludes
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|p| FilterBuilder::new().sql_contains(p.clone()).build())
        .collect()
}

// ===== Pre-scan: 单文件扫描（rayon 并行 + 文件内去重）=====

/// 扫描单个日志文件，返回满足事务级过滤条件的去重 `trxid` 列表。
///
/// 通过流式迭代器逐条处理记录（内存占用与文件大小无关），单条记录解析失败时跳过并记录警告，
/// 不影响同文件其余记录的扫描。可被上层跨文件的 `par_iter()` 安全调用（文件级并行）。
pub(super) fn scan_log_file_for_matches(file_path: &str, cfg: &Config) -> Vec<String> {
    let filters = match &cfg.filter {
        Some(f) if f.has_transaction_filters() => f,
        _ => return Vec::new(),
    };

    let records = match open_log_file(std::path::Path::new(file_path)) {
        Ok(it) => it,
        Err(e) => {
            log::warn!("Pre-scan: failed to parse '{file_path}': {e}");
            return Vec::new();
        }
    };

    let indicator_filters = build_indicator_filters(&filters.indicators);
    let sql_include_filters = build_sql_include_filters(&filters.sql);
    let sql_exclude_filters = build_sql_exclude_filters(&filters.sql);
    let has_sql_filters = filters.sql.has_filters();
    let mut trxids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for result in records {
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                log::warn!("Pre-scan: skipping malformed record in '{file_path}': {e}");
                continue;
            }
        };
        let indicator_match =
            !indicator_filters.is_empty() && indicator_filters.iter().any(|f| f.matches(&record));

        // SQL match is an independent path — not subordinate to indicator match.
        // Both filter types evaluate independently; trxid is collected if either matches.
        let sql_match = has_sql_filters && {
            let include_ok = sql_include_filters.is_empty()
                || sql_include_filters.iter().any(|f| f.matches(&record));
            let exclude_ok = sql_exclude_filters.is_empty()
                || !sql_exclude_filters.iter().any(|f| f.matches(&record));
            include_ok && exclude_ok
        };

        if indicator_match || sql_match {
            trxids.insert(record.trxid.clone());
        }
    }
    trxids.into_iter().collect()
}

// ===== Pre-scan: 跨文件编排（文件级 rayon 并行）=====

pub(super) fn scan_for_trxids_by_transaction_filters(
    log_files: &[std::path::PathBuf],
    cfg: &Config,
    jobs: usize,
) -> Result<Vec<String>> {
    use rayon::prelude::*;

    log::info!(
        "Pre-scanning {} files for transaction-level filters...",
        log_files.len()
    );

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build()
        .map_err(|e| Error::Io(std::io::Error::other(format!("rayon thread pool: {e}"))))?;

    let matched: std::collections::HashSet<String> = tokio::task::block_in_place(|| {
        pool.install(|| {
            log_files
                .par_iter()
                .flat_map(|file| {
                    if let Some(path) = file.to_str() {
                        scan_log_file_for_matches(path, cfg)
                    } else {
                        log::warn!(
                            "Pre-scan: skipping file with non-UTF8 path: {}",
                            file.display()
                        );
                        Vec::new()
                    }
                })
                .collect()
        })
    });

    Ok(matched.into_iter().collect())
}

// ===== 内存预算并发控制 =====
//
// 多文件并行场景下的内存预算控制。
//
// 旧的 `AsyncLogParser` 实现会把整份文件一次性读入内存解析成 `Vec<Sqllog>`（无流式 API），
// 并行解析的峰值内存约为 `jobs × 单文件大小`：`jobs` 越大、文件越大，越容易突破进程内存上限。
//
// 本模块在"按文件大小动态降低并发度"的策略下，把并行解析阶段的峰值内存控制在
// [`DEFAULT_MEMORY_BUDGET_BYTES`]（2GB）以内：取参与并行的文件中最大的一个作为
// 单任务内存占用的保守估计（乘以 [`PARSE_MEMORY_AMPLIFICATION`] 放大系数，
// 覆盖解析后 `Sqllog` 结构体中每个字段额外的 `String` 分配开销），
// 用 `budget / 单任务估计内存` 反推允许的最大并发数，并与用户请求的 `jobs` 取较小值。
//
// 不处理的场景：单个文件本身的预估内存就超过预算（即使并发数降到 1 也无法满足）。
// 解析器只提供一次性读取整个文件的 API，没有流式/分块接口，无法在不进一步拆分文件的前提下
// 把单文件解析的峰值内存压低；这种情况下退回到 `jobs = 1`，仅做警告，不阻止运行。

/// 并行解析阶段的默认内存预算：2GB。
pub(super) const DEFAULT_MEMORY_BUDGET_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// 解析后内存相对原始文本的放大系数（保守估计）：
/// `Sqllog` 的每个字段都是独立的 `String`/`Vec` 分配，加上 `Vec<Sqllog>` 本身的容量开销，
/// 实测文本到结构体的膨胀通常在 2~3 倍之间；取 3 作为安全上界。
const PARSE_MEMORY_AMPLIFICATION: u64 = 3;

/// 根据参与并行的文件大小，把用户请求的 `jobs` 降低到内存预算允许的范围内。
///
/// 返回值始终 `>= 1`（即使单文件预估内存已超过预算，也保留至少 1 个并发，仅靠调用方记录警告）。
pub(super) fn effective_jobs_for_memory_budget(
    files: &[std::path::PathBuf],
    requested_jobs: usize,
    budget_bytes: u64,
) -> usize {
    if requested_jobs <= 1 {
        return requested_jobs.max(1);
    }
    let max_size = files.iter().filter_map(|f| file_size(f)).max().unwrap_or(0);
    if max_size == 0 {
        return requested_jobs;
    }
    let per_task_bytes = max_size.saturating_mul(PARSE_MEMORY_AMPLIFICATION).max(1);
    let budget_jobs = usize::try_from((budget_bytes / per_task_bytes).max(1)).unwrap_or(usize::MAX);
    requested_jobs.min(budget_jobs).max(1)
}

fn file_size(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|m| m.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn write_temp_file(dir: &std::path::Path, name: &str, size: usize) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, vec![b'a'; size]).unwrap();
        path
    }

    #[test]
    fn test_requested_jobs_of_one_is_unaffected() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_temp_file(dir.path(), "a.log", 10 * 1024 * 1024);
        assert_eq!(
            effective_jobs_for_memory_budget(&[f], 1, 2 * 1024 * 1024 * 1024),
            1
        );
    }

    #[test]
    fn test_small_files_keep_requested_jobs() {
        let dir = tempfile::tempdir().unwrap();
        let files: Vec<_> = (0..8)
            .map(|i| write_temp_file(dir.path(), &format!("f{i}.log"), 1024 * 1024))
            .collect();
        // 1MB files * 3x amplification = 3MB per task; well within a 2GB budget at 8 jobs.
        assert_eq!(
            effective_jobs_for_memory_budget(&files, 8, 2 * 1024 * 1024 * 1024),
            8
        );
    }

    #[test]
    fn test_large_files_cap_jobs_below_requested() {
        let dir = tempfile::tempdir().unwrap();
        // Each file is 500MB; amplified to 1.5GB per task. A 2GB budget allows only 1 concurrent task.
        let files: Vec<_> = (0..8)
            .map(|i| write_temp_file(dir.path(), &format!("big{i}.log"), 500 * 1024 * 1024))
            .collect();
        let jobs = effective_jobs_for_memory_budget(&files, 8, 2 * 1024 * 1024 * 1024);
        assert_eq!(jobs, 1);
    }

    #[test]
    fn test_single_huge_file_falls_back_to_one_job_not_zero() {
        let dir = tempfile::tempdir().unwrap();
        // 4GB nominal size (amplified to 12GB) vastly exceeds a 2GB budget; must not return 0.
        let f = write_temp_file(dir.path(), "huge.log", 4096); // small actual file, fake huge via metadata is not possible portably
        let jobs = effective_jobs_for_memory_budget(&[f], 8, 1); // budget of 1 byte forces the floor
        assert_eq!(jobs, 1, "must always return at least 1 job");
    }

    #[test]
    fn test_missing_files_are_ignored_and_default_to_requested_jobs() {
        let missing = PathBuf::from("/nonexistent/path/should/not/exist.log");
        assert_eq!(
            effective_jobs_for_memory_budget(&[missing], 8, 2 * 1024 * 1024 * 1024),
            8
        );
    }
}
