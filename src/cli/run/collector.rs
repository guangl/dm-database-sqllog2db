use crate::error::{
    Error, ErrorStats, ParseErrorRecord, ParserError, Result, classify_error_kind,
    truncate_to_120_chars,
};
use crate::pipeline::Pipeline;
use crate::pipeline::normalizer::ParamBuffer;
use dm_database_parser_sqllog::{LogParserBuilder, ParseError, Sqllog};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// 线程内解析单个日志文件，收集记录为 Vec，不写出到任何存储。
///
/// PARAMS 记录（`record.tag.is_none()`）在 `do_normalize` 时无论是否通过过滤都必须
/// 更新 `params_buf`，以便后续 DML 记录能正确替换参数（mirror processor.rs 第 75-143 行）。
///
/// 返回 `(rows, file_stats)`，与顺序路径的错误统计对齐（含 `parse_error_records`）。
pub(super) fn collect_log_file(
    file: &Path,
    pipeline: &Pipeline,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    interrupted: &Arc<AtomicBool>,
) -> Result<(Vec<(Sqllog, Option<String>)>, ErrorStats)> {
    let file_str = file.to_string_lossy();
    let parser = LogParserBuilder::new(file_str.as_ref())
        .build()
        .map_err(|e| {
            Error::Parser(ParserError::InvalidPath {
                path: file_str.into_owned().into(),
                reason: format!("{e}"),
                line_number: None,
            })
        })?;

    let mut params_buf = ParamBuffer::default();
    let mut ns_scratch = Vec::with_capacity(4096);
    let mut rows: Vec<(Sqllog, Option<String>)> = Vec::new();
    let mut file_stats = ErrorStats::default();

    for result in parser.iter() {
        if interrupted.load(Ordering::Relaxed) {
            break;
        }
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                let (line_number, raw_ref) = match &e {
                    ParseError::InvalidFormat { raw, line_number } => (*line_number, raw.as_str()),
                    _ => (0u64, ""),
                };
                let kind = classify_error_kind(raw_ref);
                file_stats.add_parse_error_with_kind(kind);
                if file_stats.parse_error_records.len() < 10_000 {
                    file_stats.parse_error_records.push(ParseErrorRecord {
                        line_number,
                        raw_truncated: truncate_to_120_chars(raw_ref),
                        kind,
                    });
                }
                log::warn!("{} | {e:?}", file.display());
                continue;
            }
        };
        process_record(
            record,
            pipeline,
            do_normalize,
            placeholder_override,
            &mut params_buf,
            &mut ns_scratch,
            &mut rows,
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
) {
    let passes = pipeline.is_empty() || pipeline.run_with_meta(&record);
    let needs_processing = passes || (do_normalize && record.tag.is_none());
    if !needs_processing {
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
