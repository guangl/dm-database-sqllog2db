use crate::error::ErrorStats;
use crate::streaming::open_log_file;
use std::path::PathBuf;

/// 扫描一组日志文件，对每条成功解析的记录调用 `on_record` 回调。
///
/// 通过流式迭代器逐条读取，内存占用与文件大小无关。文件级打开失败（不存在/无权限）静默
/// warn 并跳过该文件；单条记录解析失败同样静默 warn 并跳过，不影响同文件其余记录。
pub(crate) fn scan_files<F>(log_files: &[PathBuf], on_record: &mut F, stats: &mut ErrorStats)
where
    F: FnMut(&dm_database_parser_sqllog::Sqllog),
{
    for (idx, file_path) in log_files.iter().enumerate() {
        log::info!("scanner: scanning {}", file_path.display());

        let records = match open_log_file(file_path) {
            Ok(it) => it,
            Err(e) => {
                let remaining = log_files.len() - idx - 1;
                log::warn!(
                    "scanner: skipping {} ({} file(s) not yet scanned): {}",
                    file_path.display(),
                    remaining,
                    e
                );
                stats.add_parse_error();
                continue;
            }
        };

        for result in records {
            match result {
                Ok(record) => on_record(&record),
                Err(e) => {
                    log::warn!(
                        "scanner: skipping malformed record in '{}': {e}",
                        file_path.display()
                    );
                    stats.add_parse_error();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_RECORD: &str = "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT 1. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n";

    #[test]
    fn scan_files_visits_records_from_every_file() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.log");
        let second = dir.path().join("second.log");
        std::fs::write(&first, VALID_RECORD).unwrap();
        std::fs::write(&second, VALID_RECORD).unwrap();

        let mut seen = 0;
        let mut stats = ErrorStats::default();
        scan_files(&[first, second], &mut |_| seen += 1, &mut stats);

        assert_eq!(seen, 2);
        assert_eq!(stats.parse_errors, 0);
        assert_eq!(stats.total_errors, 0);
    }

    #[test]
    fn scan_files_skips_unopenable_file_and_continues() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.log");
        let valid = dir.path().join("valid.log");
        std::fs::write(&valid, VALID_RECORD).unwrap();

        let mut seen = 0;
        let mut stats = ErrorStats::default();
        scan_files(&[missing, valid], &mut |_| seen += 1, &mut stats);

        assert_eq!(seen, 1, "a bad file must not hide records in later files");
        assert_eq!(stats.parse_errors, 1);
        assert_eq!(stats.total_errors, 1);
    }

    #[test]
    fn scan_files_skips_malformed_record_and_continues() {
        let dir = tempfile::tempdir().unwrap();
        let mixed = dir.path().join("mixed.log");
        std::fs::write(&mixed, format!("not a DM SQL log record\n{VALID_RECORD}")).unwrap();

        let mut seen = 0;
        let mut stats = ErrorStats::default();
        scan_files(&[mixed], &mut |_| seen += 1, &mut stats);

        assert_eq!(seen, 1, "valid records after a malformed one must survive");
        assert_eq!(stats.parse_errors, 1);
        assert_eq!(stats.total_errors, 1);
    }
}
