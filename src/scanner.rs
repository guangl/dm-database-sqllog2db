//! 公共日志文件扫描模块（v1.15 Phase 56 D-01）：被 stats 与 run 共用，统一 parse error 处理（计数 + `log::warn!`）

use crate::error::{Error, ErrorStats, ParserError, Result};
use dm_database_parser_sqllog::LogParserBuilder;
use std::path::PathBuf;

/// 扫描一组日志文件，对每条成功解析的记录调用 `on_record` 回调。
///
/// - 文件路径不存在或无法打开时返回 `Err`，终止整个扫描。
/// - 单条记录解析失败时调用 `stats.add_parse_error()` 并输出 `log::warn!`，不终止迭代。
// Task 2 将使用此函数（stats/mod.rs 重构），届时 dead_code 警告自动消除。
#[allow(dead_code)]
pub(crate) fn scan_files<F>(
    log_files: &[PathBuf],
    on_record: &mut F,
    stats: &mut ErrorStats,
) -> Result<()>
where
    F: FnMut(&dm_database_parser_sqllog::Sqllog),
{
    for file_path in log_files {
        log::info!("scanner: scanning {}", file_path.display());

        let file_path_str = file_path.to_str().ok_or_else(|| {
            Error::Parser(ParserError::InvalidPath {
                path: file_path.clone(),
                reason: "non-UTF8 path".to_string(),
                line_number: None,
            })
        })?;

        let parser = LogParserBuilder::new(file_path_str)
            .build()
            .map_err(|err| {
                Error::Parser(ParserError::InvalidPath {
                    path: file_path.clone(),
                    reason: format!("{err}"),
                    line_number: None,
                })
            })?;

        let parse_errors_before = stats.parse_errors;

        for parse_result in parser.iter() {
            match parse_result {
                Ok(record) => on_record(&record),
                Err(err) => {
                    stats.add_parse_error();
                    log::warn!("parse error in {}: {err}", file_path.display());
                }
            }
        }

        let errors_in_file = stats.parse_errors - parse_errors_before;
        if errors_in_file > 0 {
            log::warn!(
                "{}: {} parse error(s) encountered",
                file_path.display(),
                errors_in_file
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorStats;

    #[test]
    fn test_scan_files_counts_parse_errors() {
        let dir = tempfile::TempDir::new().unwrap();
        let log_file = dir.path().join("mixed.log");
        // 含 1 条非法行 + 1 条合法 SEL 记录
        let content = "this is not a valid log line\n\
            2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT id FROM orders. EXECTIME: 5(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n";
        std::fs::write(&log_file, content).unwrap();

        let files = vec![log_file];
        let mut records_seen = 0usize;
        let mut stats = ErrorStats::default();
        scan_files(&files, &mut |_record| records_seen += 1, &mut stats).unwrap();

        assert_eq!(stats.parse_errors, 1, "parse error should be counted");
        assert_eq!(records_seen, 1, "valid record should pass through");
    }

    #[test]
    fn test_scan_files_returns_err_on_invalid_path() {
        let files = vec![PathBuf::from("/nonexistent/path/test.log")];
        let mut stats = ErrorStats::default();
        let result = scan_files(&files, &mut |_| {}, &mut stats);
        assert!(result.is_err(), "invalid path should return Err");
        // parse_errors 保持 0（文件打开失败不是 parse error）
        assert_eq!(stats.parse_errors, 0);
    }
}
