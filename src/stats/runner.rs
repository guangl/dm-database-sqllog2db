use super::aggregate::{self, StatsAccumulator};
use super::config as stats_config;
use super::output;
use crate::config::Config;
use crate::error::{Error, ErrorStats, ParserError, Result};

/// 执行统计分析：流式扫描日志文件，聚合慢 SQL 与高频 SQL，写入 CSV 或 `SQLite` 输出。
///
/// `top_n` 必须 ≥ 1（由 Phase 51 的 CLI 层保证）。
///
/// # Errors
///
/// 时间范围非法、未找到任何日志文件、未配置导出器或结果写出失败时返回错误。
pub fn run_stats(cfg: &Config, top_n: u32) -> Result<()> {
    debug_assert!(top_n >= 1, "top_n must be >= 1 (Phase 51 CLI validation)");
    stats_config::validate_stats_time_range(&cfg.stats)?;
    let log_files = crate::parser::SqllogParser::new(cfg.sqllog.inputs.clone()).log_files()?;
    if log_files.is_empty() {
        return Err(Error::Parser(ParserError::NoFilesFound {
            inputs: cfg.sqllog.inputs.clone(),
        }));
    }
    let mut accumulator =
        StatsAccumulator::new(top_n, cfg.stats.from.clone(), cfg.stats.to.clone());
    scan_files_into_accumulator(&log_files, &mut accumulator);
    let (slow_rows, frequent_rows) = accumulator.into_results();
    write_stats_output(cfg, &slow_rows, &frequent_rows)
}

/// 扫描全部日志文件，逐条喂给聚合器（解析失败计入 `ErrorStats` 不终止）。
fn scan_files_into_accumulator(
    log_files: &[std::path::PathBuf],
    accumulator: &mut StatsAccumulator,
) {
    let mut scan_stats = ErrorStats::default();
    crate::scanner::scan_files(
        log_files,
        &mut |record| accumulator.update(record),
        &mut scan_stats,
    );
    if scan_stats.has_errors() {
        log::warn!(
            "stats: {} parse error(s) encountered during scan",
            scan_stats.parse_errors
        );
    }
}

/// 按 CSV 优先策略，将结果写入输出目标（CSV 优先于 `SQLite`）。
fn write_stats_output(
    cfg: &Config,
    slow_rows: &[aggregate::SlowSqlRow],
    frequent_rows: &[aggregate::FrequentSqlRow],
) -> Result<()> {
    if let Some(csv_cfg) = cfg.exporter.csv.as_ref() {
        let csv_dir = std::path::Path::new(&csv_cfg.file)
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        output::write_csv_stats(slow_rows, frequent_rows, csv_dir)?;
        log::info!(
            "stats: wrote {} slow rows and {} frequent rows to {}",
            slow_rows.len(),
            frequent_rows.len(),
            csv_dir.display()
        );
        return Ok(());
    }
    if let Some(sqlite_cfg) = cfg.exporter.sqlite.as_ref() {
        output::write_sqlite_stats(slow_rows, frequent_rows, &sqlite_cfg.database_url)?;
        log::info!(
            "stats: wrote {} slow rows and {} frequent rows to {}",
            slow_rows.len(),
            frequent_rows.len(),
            sqlite_cfg.database_url
        );
        return Ok(());
    }
    Err(Error::Config(crate::error::ConfigError::NoExporters))
}
