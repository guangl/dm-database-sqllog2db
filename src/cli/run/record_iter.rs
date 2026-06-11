//! `parallel.rs` 与 `sqlite_parallel.rs` 共享的记录迭代 + 过滤 + 归一化 + 写出回调函数（STRUCT-04）。

use crate::error::{ErrorStats, Result};
use crate::pipeline::Pipeline;
use crate::pipeline::normalizer::ParamBuffer;
use dm_database_parser_sqllog::Sqllog;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// 迭代已解析的记录列表，对每条记录执行过滤、归一化，并通过 `on_pass` 回调写出通过过滤的记录。
///
/// - `records`：已解析的日志记录列表
/// - `pipeline`：过滤与处理管道
/// - `do_normalize`：是否启用 SQL 归一化
/// - `placeholder_override`：参数占位符覆盖配置
/// - `interrupted`：中断信号（Ctrl+C）
/// - `file_stats`：文件级错误统计，函数内累加 `filtered_out`
/// - `on_pass`：通过过滤时的写出回调，接收 `(&Sqllog, Option<&str>)`（记录与归一化 SQL）
///
/// 返回成功写出的记录数。
pub(super) fn iterate_records<F>(
    records: Vec<Sqllog>,
    pipeline: &Pipeline,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    interrupted: &Arc<AtomicBool>,
    file_stats: &mut ErrorStats,
    mut on_pass: F,
) -> Result<usize>
where
    F: FnMut(&Sqllog, Option<&str>) -> Result<()>,
{
    let mut params_buf = ParamBuffer::default();
    let mut ns_scratch: Vec<u8> = Vec::with_capacity(4096);
    let mut count = 0usize;

    for record in records {
        if interrupted.load(Ordering::Acquire) {
            break;
        }

        let passes = pipeline.is_empty() || pipeline.run_with_meta(&record);
        let needs_processing = passes || (do_normalize && record.tag.is_none());
        if !needs_processing {
            file_stats.filtered_out += 1;
            continue;
        }

        if passes {
            let normalized = if do_normalize && (!params_buf.is_empty() || record.tag.is_none()) {
                crate::pipeline::compute_normalized(
                    &record,
                    &record.sql,
                    &mut params_buf,
                    placeholder_override,
                    &mut ns_scratch,
                )
                .map(str::to_owned)
            } else {
                None
            };
            on_pass(&record, normalized.as_deref())?;
            count += 1;
        } else {
            file_stats.filtered_out += 1;
            crate::pipeline::compute_normalized(
                &record,
                &record.sql,
                &mut params_buf,
                placeholder_override,
                &mut ns_scratch,
            );
        }
    }

    Ok(count)
}
