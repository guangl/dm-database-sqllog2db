use crate::error::{Error, ErrorStats, ParserError, Result};
use std::path::PathBuf;

/// 扫描一组日志文件，对每条成功解析的记录调用 `on_record` 回调。
///
/// 使用 `AsyncLogParser`，单条记录解析错误被静默丢弃。
/// 文件级错误（找不到文件、IO 错误）返回 `Err`，终止整个扫描。
pub(crate) fn scan_files<F>(
    log_files: &[PathBuf],
    on_record: &mut F,
    _stats: &mut ErrorStats,
) -> Result<()>
where
    F: FnMut(&dm_database_parser_sqllog::Sqllog),
{
    for file_path in log_files {
        log::info!("scanner: scanning {}", file_path.display());

        let records = crate::async_rt::parse_file_sync(file_path).map_err(|e| {
            let current_idx = log_files.iter().position(|f| f == file_path).unwrap_or(0);
            let remaining = log_files.len() - current_idx - 1;
            log::warn!(
                "scanner: aborting scan at {} ({} file(s) not yet scanned): {}",
                file_path.display(),
                remaining,
                e
            );
            Error::Parser(ParserError::InvalidPath {
                path: file_path.clone(),
                reason: format!("{e}"),
                line_number: None,
            })
        })?;

        for record in &records {
            on_record(record);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorStats;

    #[test]
    fn test_scan_files_valid_records_pass_through() {
        let dir = tempfile::TempDir::new().unwrap();
        let log_file = dir.path().join("mixed.log");
        let content = "this is not a valid log line\n\
            2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT id FROM orders. EXECTIME: 5(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n";
        std::fs::write(&log_file, content).unwrap();

        let files = vec![log_file];
        let mut records_seen = 0usize;
        let mut stats = ErrorStats::default();
        scan_files(&files, &mut |_record| records_seen += 1, &mut stats).unwrap();

        assert_eq!(records_seen, 1, "valid record should pass through");
    }

    #[test]
    fn test_scan_files_returns_err_on_invalid_path() {
        let files = vec![PathBuf::from("/nonexistent/path/test.log")];
        let mut stats = ErrorStats::default();
        let result = scan_files(&files, &mut |_| {}, &mut stats);
        assert!(result.is_err(), "invalid path should return Err");
        assert_eq!(stats.parse_errors, 0);
    }
}
