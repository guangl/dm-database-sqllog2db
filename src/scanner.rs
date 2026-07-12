use crate::error::ErrorStats;
use crate::streaming::open_log_file;
use std::path::PathBuf;

/// 扫描一组日志文件，对每条成功解析的记录调用 `on_record` 回调。
///
/// 通过流式迭代器逐条读取，内存占用与文件大小无关。文件级打开失败（不存在/无权限）静默
/// warn 并跳过该文件；单条记录解析失败同样静默 warn 并跳过，不影响同文件其余记录。
pub(crate) fn scan_files<F>(
    log_files: &[PathBuf],
    on_record: &mut F,
    stats: &mut ErrorStats,
)
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
