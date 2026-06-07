use crate::error::{Error, ErrorStats, Result};
use crate::exporter::ExporterManager;
use crate::pipeline::{FieldMask, Pipeline};
use dm_database_parser_sqllog::Sqllog;
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

type ParseResults = Vec<Result<Option<(PathBuf, Vec<(Sqllog, Option<String>)>, ErrorStats)>>>;

/// 创建 rayon 线程池并并行解析所有文件，返回原始结果列表。
///
/// 每个文件调用 `super::collector::collect_log_file`；中断信号检查在每个任务开始前进行。
fn run_parallel_parse(
    log_files: &[PathBuf],
    pipeline: &Pipeline,
    jobs: usize,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    interrupted: &Arc<AtomicBool>,
) -> Result<ParseResults> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build()
        .map_err(|e| Error::Io(std::io::Error::other(e)))?;
    Ok(pool.install(|| {
        log_files
            .par_iter()
            .map(|file| {
                if interrupted.load(Ordering::Acquire) {
                    return Ok(None);
                }
                let (rows, file_stats) = super::collector::collect_log_file(
                    file,
                    pipeline,
                    do_normalize,
                    placeholder_override,
                    interrupted,
                )?;
                Ok(Some((file.clone(), rows, file_stats)))
            })
            .collect()
    }))
}

/// 在 rayon 线程池中并行解析所有文件，返回每个文件的 `(path, Vec<(Sqllog, Option<String>)>)`。
///
/// 返回 `(collected, skipped_files, merged_stats)`，其中 `collected` 保留 path 与记录的对应关系。
fn parallel_collect(
    log_files: &[PathBuf],
    pipeline: &Pipeline,
    jobs: usize,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    interrupted: &Arc<AtomicBool>,
) -> Result<(
    Vec<(PathBuf, Vec<(Sqllog, Option<String>)>)>,
    usize,
    ErrorStats,
)> {
    let results = run_parallel_parse(
        log_files,
        pipeline,
        jobs,
        do_normalize,
        placeholder_override,
        interrupted,
    )?;

    let mut collected: Vec<(PathBuf, Vec<(Sqllog, Option<String>)>)> =
        Vec::with_capacity(log_files.len());
    let mut first_err: Option<Error> = None;
    let mut skipped = 0usize;
    let mut merged_stats = ErrorStats::default();
    for result in results {
        match result {
            Ok(Some((path, rows, file_stats))) => {
                merged_stats.merge(&file_stats);
                collected.push((path, rows));
            }
            Ok(None) => skipped += 1,
            Err(e) => {
                log::warn!("parallel collect error: {e}");
                if first_err.is_none() {
                    first_err = Some(e);
                }
            }
        }
    }
    if let Some(e) = first_err {
        return Err(e);
    }
    Ok((collected, skipped, merged_stats))
}

/// `SQLite` 并行处理：每个文件独立在 rayon 线程上解析，主线程按文件原始顺序写入 `SQLite`。
///
/// 返回：`(per_file_counts, skipped_files, parse_stats)`，其中 `per_file_counts` 是每文件
/// `(path, count)` 列表，`parse_stats` 汇总所有文件的解析错误统计。
/// 适用条件：SQLite 导出 + 多文件 + jobs > 1 + 非 stdin 管道。
///
/// `_field_mask`, `_ordered_indices` 保留参数签名以便日后扩展：
///   - `_field_mask` / `_ordered_indices`：已通过 `cfg` 传入 `ExporterManager`，无需重复传递。
///
/// 注意：并行模式暂不支持每文件进度显示；外层 `run_sqlite_parallel` 的 verbose eprintln 已足够。
#[allow(clippy::too_many_arguments)]
pub(super) fn process_sqlite_parallel(
    log_files: &[PathBuf],
    cfg: &crate::config::Config,
    pipeline: &Pipeline,
    jobs: usize,
    interrupted: &Arc<AtomicBool>,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    _field_mask: FieldMask,
    _ordered_indices: &[usize],
) -> Result<(Vec<(PathBuf, usize)>, usize, ErrorStats)> {
    let (collected, skipped, parallel_stats) = parallel_collect(
        log_files,
        pipeline,
        jobs,
        do_normalize,
        placeholder_override,
        interrupted,
    )?;

    if parallel_stats.parse_errors > 0 {
        log::warn!(
            "SQLite parallel: {} parse error(s) across all files",
            parallel_stats.parse_errors
        );
    }

    // 写入 SQLite 并同时构建每文件 (path, count) 列表
    let mut exporter_manager = ExporterManager::from_config(cfg)?;
    exporter_manager.initialize()?;
    exporter_manager.set_sqlite_wal_mode()?;

    let mut per_file_counts: Vec<(PathBuf, usize)> = Vec::with_capacity(collected.len());
    for (file_path, file_rows) in collected {
        let count = file_rows.len();
        for (record, normalized) in file_rows {
            exporter_manager.export_one_preparsed(&record, true, normalized.as_deref())?;
        }
        per_file_counts.push((file_path, count));
    }

    exporter_manager.finalize()?;
    Ok((per_file_counts, skipped, parallel_stats))
}
