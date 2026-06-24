//! 流式日志读取的共享入口。
//!
//! `dm-database-parser-sqllog` 自 2.0.x 起通过 `LogParserBuilder::build()?.iter()?`
//! 暴露同步流式迭代器 `LogIterator`：内部以 `BufReader` 逐行读取，内存占用与文件大小无关
//! （`LogParserBuilder::build()` 仅采样文件头尾各最多 64KB/4KB 探测编码，不会整文件加载）。
//!
//! 这取代了旧的 `AsyncLogParser::parse()` 一次性 `.collect()` 整个文件到 `Vec<Sqllog>` 的方式
//! （峰值内存与文件大小成正比，参见已废弃的 `cli::run::memory_budget` 注释）。
//!
//! 错误粒度变化：旧的 `.collect()` 语义下，文件中任意一条记录解析失败会导致整个文件返回 0 条
//! 记录（连坏记录之前的好记录也被丢弃）。新的逐记录流式迭代改为：坏记录单独跳过并计入错误统计，
//! 同文件其余好记录正常导出。

use dm_database_parser_sqllog::{FileEncodingHint, LogIterator, LogParserBuilder, ParseError};
use std::path::Path;

/// 打开日志文件并返回流式记录迭代器。
///
/// 显式传入 `FileEncodingHint::Auto`（而非留空触发 `LogParserBuilder` 默认的编码探测）：
/// 默认探测会对文件做 `seek(SeekFrom::End(0))` 以获取大小，这对常规文件无害，但对 `/dev/stdin`
/// 等管道类输入会失败（`Illegal seek`，管道不可 seek）。显式传入 `Auto` 与旧的
/// `AsyncLogParser`（内部固定传 `Auto`）行为一致，跳过整文件探测，编码改为逐行/逐记录探测。
///
/// 仅在文件级别失败时返回 `Err`（文件不存在、无权限等）；单条记录的解析失败由迭代器在逐条
/// `next()` 时以 `Err` 形式产出，由调用方决定如何处理（当前各调用点统一选择：跳过该记录、
/// 计入 `ErrorStats::add_parse_error`，继续处理后续记录）。
pub(crate) fn open_log_file(path: &Path) -> Result<LogIterator, ParseError> {
    LogParserBuilder::new(path)
        .encoding_hint(FileEncodingHint::Auto)
        .build()?
        .iter()
}
