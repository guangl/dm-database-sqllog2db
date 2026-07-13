use crate::engine::run::{ProcessOutcome, RunContext};
use crate::error::{ErrorStats, Result};
use crate::exporter::ExporterManager;
use crate::pipeline::Pipeline;
use crate::streaming::open_log_file;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn parse_and_write_sqlite(
    file: &std::path::Path,
    em: &mut ExporterManager,
    pipeline: &Pipeline,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    interrupted: &Arc<AtomicBool>,
) -> Result<(usize, ErrorStats)> {
    let records = match open_log_file(file) {
        Ok(it) => it,
        Err(e) => {
            log::warn!("parse failed for '{}': {e}", file.display());
            let mut file_stats = ErrorStats::default();
            file_stats.add_parse_error();
            return Ok((0, file_stats));
        }
    };

    let mut file_stats = ErrorStats::default();
    let count = crate::engine::record::iterate_records(
        records,
        pipeline,
        do_normalize,
        placeholder_override,
        interrupted,
        &mut file_stats,
        |record, normalized| em.export_one_preparsed(record, true, normalized),
    )?;

    Ok((count, file_stats))
}

/// `SQLite` 并行处理：逐文件通过 `AsyncLogParser` 解析后写入，`SQLite` 写入本身必须串行。
///
/// 注：`SQLite` 写入无法并行（`rusqlite Connection` 非 `Send`），且 `ctx.field_mask` /
/// `ctx.ordered_indices` 不在此直接使用——`ExporterManager::from_config` 会从 config 内部读取
/// 字段投影配置。请确保 orchestrator 计算的投影与 `from_config` 内部逻辑保持一致。
pub(crate) fn process_sqlite_parallel(
    ctx: &RunContext<'_>,
    log_files: &[PathBuf],
    interrupted: &Arc<AtomicBool>,
) -> Result<ProcessOutcome> {
    let mut em = ExporterManager::from_config(ctx.cfg)?;
    em.initialize()?;
    em.set_sqlite_wal_mode()?;

    let mut per_file_counts: Vec<(PathBuf, usize)> = Vec::with_capacity(log_files.len());
    let mut merged_stats = ErrorStats::default();

    for file in log_files {
        if interrupted.load(Ordering::Acquire) {
            break;
        }
        let (count, file_stats) = parse_and_write_sqlite(
            file,
            &mut em,
            &ctx.pipeline,
            ctx.do_normalize,
            ctx.placeholder_override,
            interrupted,
        )?;
        merged_stats.merge(&file_stats);
        per_file_counts.push((file.clone(), count));
    }

    em.finalize()?;
    Ok((per_file_counts, 0, merged_stats))
}
