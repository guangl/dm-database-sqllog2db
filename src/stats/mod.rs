//! SQL 统计分析模块（v1.13）：提供 SQL 标准化与统计聚合。

pub mod aggregate;
pub mod config;
pub mod normalize;
pub mod output;

// Public re-exports for lib API consumers; both may be unused in the bin target.
#[allow(unused_imports)]
pub use config::StatsConfig;
#[allow(unused_imports)]
pub use config::validate_time_str;

use crate::config::Config;
use crate::error::{Error, ErrorStats, ParserError, Result};
use aggregate::StatsAccumulator;

/// 执行统计分析：流式扫描日志文件，聚合慢 SQL 与高频 SQL，写入 CSV 或 `SQLite` 输出。
///
/// `top_n` 必须 ≥ 1（由 Phase 51 的 CLI 层保证）。
pub fn run_stats(cfg: &Config, top_n: u32) -> Result<()> {
    debug_assert!(top_n >= 1, "top_n must be >= 1 (Phase 51 CLI validation)");
    config::validate_stats_time_range(&cfg.stats)?;
    let log_files = crate::parser::SqllogParser::new(cfg.sqllog.inputs.clone()).log_files()?;
    if log_files.is_empty() {
        return Err(Error::Parser(ParserError::NoFilesFound {
            inputs: cfg.sqllog.inputs.clone(),
        }));
    }
    let mut accumulator =
        StatsAccumulator::new(top_n, cfg.stats.from.clone(), cfg.stats.to.clone());
    scan_files_into_accumulator(&log_files, &mut accumulator)?;
    let (slow_rows, frequent_rows) = accumulator.into_results();
    write_stats_output(cfg, &slow_rows, &frequent_rows)
}

/// 扫描全部日志文件，逐条喂给聚合器（解析失败计入 `ErrorStats` 不终止）。
fn scan_files_into_accumulator(
    log_files: &[std::path::PathBuf],
    accumulator: &mut StatsAccumulator,
) -> Result<()> {
    let mut scan_stats = ErrorStats::default();
    crate::scanner::scan_files(
        log_files,
        &mut |record| accumulator.update(record),
        &mut scan_stats,
    )?;
    if scan_stats.has_errors() {
        log::info!(
            "stats: {} parse error(s) encountered during scan",
            scan_stats.parse_errors
        );
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, CsvExporterConfig, ExporterConfig, SqllogConfig};
    use crate::error::ConfigError;

    fn make_csv_config(log_path: &str, csv_path: &str) -> Config {
        Config {
            sqllog: SqllogConfig {
                inputs: vec![log_path.to_string()],
                path_deprecated: None,
            },
            exporter: ExporterConfig {
                csv: Some(CsvExporterConfig {
                    file: csv_path.to_string(),
                    overwrite: true,
                    ..CsvExporterConfig::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn write_test_log(path: &std::path::Path, count: usize) {
        use std::fmt::Write as _;
        let mut buf = String::with_capacity(count * 180);
        for idx in 0..count {
            writeln!(
                buf,
                "2025-01-15 10:30:28.001 (EP[0] sess:0x{idx:04x} user:TESTUSER trxid:{idx} stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT * FROM t WHERE id={idx}. EXECTIME: {exec}(ms) ROWCOUNT: 1(rows) EXEC_ID: {idx}.",
                exec = (idx * 7) % 100 + 1,
            ).unwrap();
        }
        std::fs::write(path, buf).unwrap();
    }

    #[test]
    fn test_run_stats_csv_mode_selected_when_only_csv_configured() {
        let dir = tempfile::TempDir::new().unwrap();
        let log_file = dir.path().join("test.log");
        write_test_log(&log_file, 3);
        let csv_path = dir.path().join("out").join("data.csv");
        let cfg = make_csv_config(log_file.to_str().unwrap(), csv_path.to_str().unwrap());
        run_stats(&cfg, 10).unwrap();
        assert!(dir.path().join("out").join("slow_sql.csv").exists());
    }

    #[test]
    fn test_run_stats_propagates_no_files_found() {
        let dir = tempfile::TempDir::new().unwrap();
        let csv_path = dir.path().join("out.csv");
        // 传入存在但为空目录（log_files 返回空列表触发 NoFilesFound）
        let empty_dir = dir.path().join("empty");
        std::fs::create_dir_all(&empty_dir).unwrap();
        let cfg = make_csv_config(empty_dir.to_str().unwrap(), csv_path.to_str().unwrap());
        let result = run_stats(&cfg, 5);
        assert!(result.is_err(), "empty input should return Err");
    }

    #[test]
    fn test_run_stats_skips_parse_errors() {
        let dir = tempfile::TempDir::new().unwrap();
        let log_file = dir.path().join("mixed.log");
        // 含 1 条非法行 + 1 条合法 DML 记录
        let content = "this is not a valid log line\n\
            2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT id FROM orders. EXECTIME: 5(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n";
        std::fs::write(&log_file, content).unwrap();
        let csv_path = dir.path().join("out.csv");
        let cfg = make_csv_config(log_file.to_str().unwrap(), csv_path.to_str().unwrap());
        // 应返回 Ok（非法行不终止流程）
        run_stats(&cfg, 5).unwrap();
        let slow_csv = dir.path().join("slow_sql.csv");
        let slow_content = std::fs::read_to_string(slow_csv).unwrap();
        assert!(
            slow_content.lines().count() >= 2,
            "should contain header + at least 1 data row from valid record"
        );
    }

    #[test]
    fn test_run_stats_rejects_invalid_from() {
        let dir = tempfile::TempDir::new().unwrap();
        let csv_path = dir.path().join("out.csv");
        let mut cfg = make_csv_config(dir.path().to_str().unwrap(), csv_path.to_str().unwrap());
        cfg.stats.from = Some("bad".to_string());
        let result = run_stats(&cfg, 5);
        assert!(result.is_err(), "invalid from should return Err");
        match result.unwrap_err() {
            Error::Config(ConfigError::InvalidValue { field, .. }) => {
                assert_eq!(field, "stats.from");
            }
            other => panic!("expected ConfigError::InvalidValue, got: {other:?}"),
        }
    }

    #[test]
    fn test_run_stats_rejects_invalid_to() {
        let dir = tempfile::TempDir::new().unwrap();
        let csv_path = dir.path().join("out.csv");
        let mut cfg = make_csv_config(dir.path().to_str().unwrap(), csv_path.to_str().unwrap());
        cfg.stats.to = Some("20240101".to_string());
        let result = run_stats(&cfg, 5);
        assert!(result.is_err(), "invalid to should return Err");
        match result.unwrap_err() {
            Error::Config(ConfigError::InvalidValue { field, .. }) => {
                assert_eq!(field, "stats.to");
            }
            other => panic!("expected ConfigError::InvalidValue, got: {other:?}"),
        }
    }
}
