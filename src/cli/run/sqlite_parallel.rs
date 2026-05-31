use crate::error::{Error, Result};
use crate::exporter::ExporterManager;
use crate::pipeline::normalizer::ParamBuffer;
use crate::pipeline::{CompiledSqlFilters, FieldMask, Pipeline};
use dm_database_parser_sqllog::{LogParserBuilder, Sqllog};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// 线程内解析单个日志文件，收集记录为 Vec，不写出到任何存储。
///
/// PARAMS 记录（`record.tag.is_none()`）在 `do_normalize` 时无论是否通过过滤都必须
/// 更新 `params_buf`，以便后续 DML 记录能正确替换参数（mirror processor.rs 第 75-143 行）。
///
/// 返回 `(rows, parse_error_count)`，与顺序路径的错误统计对齐。
fn collect_log_file(
    file: &Path,
    pipeline: &Pipeline,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    sql_record_filter: Option<&CompiledSqlFilters>,
    interrupted: &Arc<AtomicBool>,
) -> Result<(Vec<(Sqllog, Option<String>)>, usize)> {
    let file_str = file.to_string_lossy();
    let parser = LogParserBuilder::new(file_str.as_ref())
        .build()
        .map_err(|e| {
            crate::error::Error::Parser(crate::error::ParserError::InvalidPath {
                path: file_str.into_owned().into(),
                reason: format!("{e}"),
                line_number: None,
            })
        })?;

    let mut params_buf = ParamBuffer::default();
    let mut ns_scratch = Vec::with_capacity(4096);
    let mut rows: Vec<(Sqllog, Option<String>)> = Vec::new();
    let mut parse_errors: usize = 0;

    for result in parser.iter() {
        if interrupted.load(Ordering::Relaxed) {
            break;
        }
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                parse_errors += 1;
                log::warn!("{} | parse error: {e:?}", file.display());
                continue;
            }
        };
        process_record(
            record,
            pipeline,
            do_normalize,
            placeholder_override,
            sql_record_filter,
            &mut params_buf,
            &mut ns_scratch,
            &mut rows,
        );
    }
    Ok((rows, parse_errors))
}

#[allow(clippy::too_many_arguments)]
fn process_record(
    record: Sqllog,
    pipeline: &Pipeline,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    sql_record_filter: Option<&CompiledSqlFilters>,
    params_buf: &mut ParamBuffer,
    ns_scratch: &mut Vec<u8>,
    rows: &mut Vec<(Sqllog, Option<String>)>,
) {
    let passes = pipeline.is_empty() || pipeline.run_with_meta(&record);
    let needs_processing = passes || (do_normalize && record.tag.is_none());
    if !needs_processing {
        return;
    }
    if passes {
        let sql_filter_pass =
            sql_record_filter.is_none_or(|f| record.tag.is_none() || f.matches(&record.sql));
        if sql_filter_pass {
            let normalized = if do_normalize && (!params_buf.is_empty() || record.tag.is_none()) {
                crate::pipeline::compute_normalized(
                    &record,
                    &record.sql,
                    params_buf,
                    placeholder_override,
                    ns_scratch,
                )
                .map(str::to_owned)
            } else {
                None
            };
            rows.push((record, normalized));
        }
    } else {
        // 被过滤掉的 PARAMS 记录仍需更新 params_buf（do_normalize && record.tag.is_none()）
        crate::pipeline::compute_normalized(
            &record,
            &record.sql,
            params_buf,
            placeholder_override,
            ns_scratch,
        );
    }
}

/// 在 rayon 线程池中并行解析所有文件，返回每个文件的 `(path, Vec<(Sqllog, Option<String>)>)`。
///
/// 返回 `(collected, skipped_files, total_parse_errors)`，其中 `collected` 保留 path 与记录的对应关系。
fn parallel_collect(
    log_files: &[PathBuf],
    pipeline: &Pipeline,
    jobs: usize,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    sql_record_filter: Option<&CompiledSqlFilters>,
    interrupted: &Arc<AtomicBool>,
) -> Result<(Vec<(PathBuf, Vec<(Sqllog, Option<String>)>)>, usize, usize)> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build()
        .map_err(|e| Error::Io(std::io::Error::other(e)))?;

    let results: Vec<Result<Option<(PathBuf, Vec<(Sqllog, Option<String>)>, usize)>>> = pool
        .install(|| {
            log_files
                .par_iter()
                .map(|file| {
                    if interrupted.load(Ordering::Relaxed) {
                        return Ok(None);
                    }
                    let (rows, parse_errors) = collect_log_file(
                        file,
                        pipeline,
                        do_normalize,
                        placeholder_override,
                        sql_record_filter,
                        interrupted,
                    )?;
                    Ok(Some((file.clone(), rows, parse_errors)))
                })
                .collect()
        });

    let mut collected: Vec<(PathBuf, Vec<(Sqllog, Option<String>)>)> =
        Vec::with_capacity(log_files.len());
    let mut first_err: Option<Error> = None;
    let mut skipped = 0usize;
    let mut total_parse_errors = 0usize;
    for result in results {
        match result {
            Ok(Some((path, rows, parse_errors))) => {
                total_parse_errors += parse_errors;
                collected.push((path, rows));
            }
            Ok(None) => skipped += 1,
            Err(e) if first_err.is_none() => first_err = Some(e),
            Err(_) => {}
        }
    }
    if let Some(e) = first_err {
        return Err(e);
    }
    Ok((collected, skipped, total_parse_errors))
}

/// `SQLite` 并行处理：每个文件独立在 rayon 线程上解析，主线程按文件原始顺序写入 `SQLite`。
///
/// 返回：`(per_file_counts, skipped_files)`，其中 `per_file_counts` 是每文件 `(path, count)` 列表。
/// 适用条件：SQLite 导出 + 多文件 + jobs > 1 + 非 stdin 管道。
///
/// `_show_progress`, `_field_mask`, `_ordered_indices` 保留参数签名以与
/// `process_csv_parallel` 的调用方对称，方便日后扩展。
/// - `_show_progress`：进度显示尚未在 `SQLite` 并行路径中实现。
/// - `_field_mask` / `_ordered_indices`：已通过 `cfg` 传入 `ExporterManager`，无需重复传递。
#[allow(clippy::too_many_arguments)]
pub(super) fn process_sqlite_parallel(
    log_files: &[PathBuf],
    cfg: &crate::config::Config,
    pipeline: &Pipeline,
    jobs: usize,
    _show_progress: bool,
    interrupted: &Arc<AtomicBool>,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    _field_mask: FieldMask,
    _ordered_indices: &[usize],
    sql_record_filter: Option<&CompiledSqlFilters>,
) -> Result<(Vec<(PathBuf, usize)>, usize)> {
    let (collected, skipped, total_parse_errors) = parallel_collect(
        log_files,
        pipeline,
        jobs,
        do_normalize,
        placeholder_override,
        sql_record_filter,
        interrupted,
    )?;

    if total_parse_errors > 0 {
        log::warn!("SQLite parallel: {total_parse_errors} parse error(s) across all files");
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
    Ok((per_file_counts, skipped))
}
