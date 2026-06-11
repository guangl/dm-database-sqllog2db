use crate::error::{Error, ErrorStats, ParserError, Result};
use crate::pipeline::Pipeline;
use crate::pipeline::normalizer::ParamBuffer;
use dm_database_parser_sqllog::{AsyncLogParser, Sqllog};
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
    // Check path exists first so IO errors are distinct from parse errors
    if !file.exists() {
        return Err(Error::Parser(ParserError::InvalidPath {
            path: file.to_path_buf(),
            reason: "file not found".to_string(),
            line_number: None,
        }));
    }
    let records = match AsyncLogParser::new(file).parse().await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("collect_log_file: parse error in '{}': {e}", file.display());
            return Ok((Vec::new(), ErrorStats::default()));
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
