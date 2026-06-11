use crate::error::{ErrorStats, Result};
use dm_database_parser_sqllog::AsyncLogParser;
use std::path::PathBuf;

/// 扫描一组日志文件，对每条成功解析的记录调用 `on_record` 回调。
///
/// 使用 `AsyncLogParser`，文件级解析失败静默 warn 并跳过该文件（0 条记录）。
/// 注意：`AsyncLogParser::parse()` 将整个文件一次性读入内存后再返回记录列表，
/// 内存占用与文件大小成正比（非流式），大文件（如 1.1GB）时内存压力较大。
/// 其他 IO 错误返回 `Err`，终止整个扫描。
pub(crate) async fn scan_files<F>(
    log_files: &[PathBuf],
    on_record: &mut F,
    stats: &mut ErrorStats,
) -> Result<()>
where
    F: FnMut(&dm_database_parser_sqllog::Sqllog),
{
    for (idx, file_path) in log_files.iter().enumerate() {
        log::info!("scanner: scanning {}", file_path.display());

        let records = match AsyncLogParser::new(file_path).parse().await {
            Ok(r) => r,
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

        for record in &records {
            on_record(record);
        }
    }
    Ok(())
}
