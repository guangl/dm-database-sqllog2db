use crate::error::{ErrorStats, Result};
use crate::exporter::ExporterManager;
use crate::pipeline::{FieldMask, Pipeline};
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
/// 参数说明（被忽略的参数保留以与 `process_csv_parallel` 签名对齐，便于统一调用）：
///
/// - `_jobs`：`rusqlite Connection` 不实现 `Send`，`SQLite` 写入必须串行，无法并行化；
///   参数保留是为了与 CSV 路径的调用接口一致。
/// - `_field_mask` / `_ordered_indices`：`SQLite` 路径通过 `ExporterManager::from_config(cfg)`
///   内部读取字段投影配置，与 CSV 路径通过显式参数传入不同。如果 orchestrator 独立计算的
///   `field_mask` 与 `from_config` 内部逻辑出现分歧，`SQLite` 路径将以 config 为准，请确保
///   两者保持一致。
pub(crate) fn process_sqlite_parallel(
    log_files: &[PathBuf],
    cfg: &crate::config::Config,
    pipeline: &Pipeline,
    _jobs: usize,
    interrupted: &Arc<AtomicBool>,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    _field_mask: FieldMask,
    _ordered_indices: &[usize],
) -> Result<(Vec<(PathBuf, usize)>, usize, ErrorStats)> {
    let mut em = ExporterManager::from_config(cfg)?;
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
            pipeline,
            do_normalize,
            placeholder_override,
            interrupted,
        )?;
        merged_stats.merge(&file_stats);
        per_file_counts.push((file.clone(), count));
    }

    em.finalize()?;
    Ok((per_file_counts, 0, merged_stats))
}
