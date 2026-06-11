use crate::error::{Error, ErrorStats, ParserError, Result};
use crate::pipeline::Pipeline;
use crate::pipeline::normalizer::ParamBuffer;
use dm_database_parser_sqllog::{AsyncError, AsyncLogParser, ParseError, Sqllog};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// 线程内解析单个日志文件，收集记录为 Vec，不写出到任何存储。
pub(super) async fn collect_log_file(
    file: &Path,
    pipeline: &Pipeline,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    interrupted: &Arc<AtomicBool>,
) -> Result<(Vec<(Sqllog, Option<String>)>, ErrorStats)> {
    // Parse the file, distinguishing IO / not-found errors from line-level parse errors.
    // We do not pre-check file.exists() to avoid the TOCTOU race where the file could
    // disappear between the check and the open — instead we inspect the error variant.
    let records = match AsyncLogParser::new(file).parse().await {
        Ok(r) => r,
        Err(AsyncError::Parse(ParseError::InvalidFormat { .. })) => {
            // Line-level parse failure is non-fatal; treat as an empty file.
            log::warn!("collect_log_file: parse error in '{}'", file.display());
            return Ok((Vec::new(), ErrorStats::default()));
        }
        Err(e) => {
            // IO errors, FileNotFound, Panic — propagate as Err so the caller
            // can distinguish "file missing or inaccessible" from parse failures.
            return Err(Error::Parser(ParserError::InvalidPath {
                path: file.to_path_buf(),
                reason: e.to_string(),
                line_number: None,
            }));
        }
    };

    let mut params_buf = ParamBuffer::default();
    let mut ns_scratch = Vec::with_capacity(4096);
    let mut rows: Vec<(Sqllog, Option<String>)> = Vec::new();
    let mut file_stats = ErrorStats::default();

    for record in records {
        if interrupted.load(Ordering::Acquire) {
            break;
        }
        process_record(
            record,
            pipeline,
            do_normalize,
            placeholder_override,
            &mut params_buf,
            &mut ns_scratch,
            &mut rows,
            &mut file_stats,
        );
    }
    Ok((rows, file_stats))
}

fn process_record(
    record: Sqllog,
    pipeline: &Pipeline,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    params_buf: &mut ParamBuffer,
    ns_scratch: &mut Vec<u8>,
    rows: &mut Vec<(Sqllog, Option<String>)>,
    file_stats: &mut ErrorStats,
) {
    let passes = pipeline.is_empty() || pipeline.run_with_meta(&record);
    let needs_processing = passes || (do_normalize && record.tag.is_none());
    if !needs_processing {
        file_stats.filtered_out += 1;
        return;
    }
    if passes {
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
    } else {
        file_stats.filtered_out += 1;
        crate::pipeline::compute_normalized(
            &record,
            &record.sql,
            params_buf,
            placeholder_override,
            ns_scratch,
        );
    }
}
