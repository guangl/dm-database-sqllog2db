use crate::error::{Error, ErrorStats, Result};
use crate::exporter::ExporterManager;
use crate::pipeline::{FieldMask, Pipeline, normalizer::ParamBuffer};
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
    let records = crate::async_rt::parse_file_sync(file).map_err(|e| {
        Error::Parser(crate::error::ParserError::InvalidPath {
            path: file.to_path_buf(),
            reason: format!("{e}"),
            line_number: None,
        })
    })?;

    let mut params_buf = ParamBuffer::default();
    let mut ns_scratch: Vec<u8> = Vec::with_capacity(4096);
    let mut file_stats = ErrorStats::default();
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
            em.export_one_preparsed(&record, true, normalized.as_deref())?;
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

    Ok((count, file_stats))
}

/// `SQLite` 并行处理：逐文件通过 `AsyncLogParser` 解析后写入，`SQLite` 写入本身必须串行。
#[allow(clippy::too_many_arguments)]
pub(super) fn process_sqlite_parallel(
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
