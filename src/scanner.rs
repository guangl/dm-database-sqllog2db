use crate::error::{ErrorStats, Result};
use dm_database_parser_sqllog::AsyncLogParser;
use std::path::PathBuf;

/// 扫描一组日志文件，对每条成功解析的记录调用 `on_record` 回调。
///
/// 使用 `AsyncLogParser`，文件级解析失败静默 warn 并跳过该文件（0 条记录）。
/// 其他 IO 错误返回 `Err`，终止整个扫描。
pub(crate) async fn scan_files<F>(
    log_files: &[PathBuf],
    on_record: &mut F,
    _stats: &mut ErrorStats,
) -> Result<()>
where
    F: FnMut(&dm_database_parser_sqllog::Sqllog),
{
    for file_path in log_files {
        log::info!("scanner: scanning {}", file_path.display());

        let records = match AsyncLogParser::new(file_path).parse().await {
            Ok(r) => r,
            Err(e) => {
                let current_idx = log_files.iter().position(|f| f == file_path).unwrap_or(0);
                let remaining = log_files.len() - current_idx - 1;
                log::warn!(
                    "scanner: skipping {} ({} file(s) not yet scanned): {}",
                    file_path.display(),
                    remaining,
                    e
                );
                continue;
            }
        };

        for record in &records {
            on_record(record);
        }
    }
    Ok(())
}
