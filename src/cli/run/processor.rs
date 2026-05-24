use crate::error::{ErrorStats, Result};
use crate::exporter::ExporterManager;
use crate::pipeline::normalizer::ParamBuffer;
use crate::pipeline::{CompiledSqlFilters, Pipeline};
use dm_database_parser_sqllog::LogParserBuilder;
use indicatif::ProgressBar;
use log::info;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// 处理单个日志文件，返回本文件实际导出的记录数。
///
/// `remaining`: 最多再导出多少条记录（跨文件的剩余配额），`None` 表示不限制。
/// `reset_pb`: 是否在文件开始时重置进度条计数；并行模式传 `false`，避免多线程互相重置。
pub(super) fn process_log_file(
    file_path: &str,
    file_index: usize,
    total_files: usize,
    exporter_manager: &mut ExporterManager,
    pipeline: &Pipeline,
    show_progress: bool,
    remaining: Option<usize>,
    interrupted: &Arc<AtomicBool>,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    params_buffer: &mut ParamBuffer,
    ns_scratch: &mut Vec<u8>,
    reset_pb: bool,
    sql_record_filter: Option<&CompiledSqlFilters>,
    pb: Option<&ProgressBar>,
) -> Result<(usize, ErrorStats)> {
    // 清除上一个文件留下的残余参数，同时复用已分配的 HashMap 容量。
    params_buffer.clear();

    // 从导出器读取性能指标标志：CSV 关闭时跳过性能指标输出（D-05/D-06）
    let include_pm = exporter_manager.csv_include_performance_metrics();

    let file_start = Instant::now();

    let file_name = std::path::Path::new(file_path).file_name().map_or_else(
        || file_path.to_string(),
        |n| n.to_string_lossy().into_owned(),
    );

    if reset_pb && show_progress {
        if let Some(pb) = pb {
            pb.set_message(format!("[{file_index}/{total_files}] {file_name}"));
            pb.set_position(0);
        }
    }

    let parser = LogParserBuilder::new(file_path).build().map_err(|e| {
        crate::error::Error::Parser(crate::error::ParserError::InvalidPath {
            path: file_path.into(),
            reason: format!("{e}"),
            line_number: None,
        })
    })?;

    let mut records_in_file = 0usize;
    let mut errors_in_file = 0usize;
    let mut file_stats = ErrorStats::default();

    'outer: for result in parser.iter() {
        match result {
            Ok(record) => {
                // 管线：使用 record 直接字段（parser 库已物化所有字段）。
                let passes = if pipeline.is_empty() {
                    true
                } else {
                    pipeline.run_with_meta(&record)
                };

                // PARAMS 记录（无 tag）在 do_normalize 时无论是否通过过滤都必须
                // 更新 params_buffer，以便后续匹配 DML 记录能正确替换参数。
                let needs_processing = passes || (do_normalize && record.tag.is_none());
                if needs_processing {
                    if passes {
                        // SQL 记录级过滤：只对 DML 记录（有 tag）生效，PARAMS 记录始终通过。
                        let sql_filter_pass = sql_record_filter
                            .is_none_or(|f| record.tag.is_none() || f.matches(&record.sql));
                        if sql_filter_pass {
                            // 快速路径：params_buffer 为空且当前是 DML 记录（有 tag），
                            // 则不可能存在待替换参数，完全跳过 compute_normalized。
                            let ns = if do_normalize
                                && (!params_buffer.is_empty() || record.tag.is_none())
                            {
                                crate::pipeline::compute_normalized(
                                    &record,
                                    &record.sql,
                                    params_buffer,
                                    placeholder_override,
                                    ns_scratch,
                                )
                            } else {
                                None
                            };

                            // 先检查配额，再聚合（CR-02：避免对未导出记录计入统计）
                            if let Some(remaining) = remaining {
                                if records_in_file >= remaining {
                                    break 'outer;
                                }
                            }

                            exporter_manager
                                .export_one_preparsed(&record, include_pm, ns)
                                .map_or_else(
                                    |e| {
                                        if e.is_fatal() {
                                            file_stats.set_fatal(e.to_string());
                                        } else {
                                            file_stats.add_export_error();
                                        }
                                        eprintln!("[{}] {file_path}: {e}", e.severity());
                                        log::warn!("{file_path} | export error: {e:?}");
                                    },
                                    |()| {},
                                );
                            records_in_file += 1;

                            // 每 1024 条更新进度并检查中断信号
                            if records_in_file.trailing_zeros() >= 10 {
                                if let Some(pb) = pb {
                                    pb.inc(1024);
                                }
                                if interrupted.load(Ordering::Relaxed) {
                                    break 'outer;
                                }
                            }
                        }
                    } else {
                        // 被过滤掉的 PARAMS 记录（do_normalize && record.tag.is_none()）：
                        // 仍需更新 params_buffer。
                        crate::pipeline::compute_normalized(
                            &record,
                            &record.sql,
                            params_buffer,
                            placeholder_override,
                            ns_scratch,
                        );
                    }
                }
            }
            Err(e) => {
                errors_in_file += 1;
                file_stats.add_parse_error();
                log::warn!("{file_path} | {e:?}");
            }
        }
    }

    if errors_in_file > 0 {
        log::warn!("{file_path}: {errors_in_file} parse errors");
    }

    let elapsed = file_start.elapsed().as_secs_f64();
    info!(
        "File {file_path}: {records_in_file} records, {errors_in_file} errors, total {elapsed:.2}s",
    );

    if show_progress {
        if let Some(pb) = pb {
            let errors_label = if errors_in_file > 0 {
                format!(", {errors_in_file} errors")
            } else {
                String::new()
            };
            pb.set_message(format!(
                "✓ [{file_index}/{total_files}] {file_path} — {records_in_file}{errors_label}, {elapsed:.2}s",
            ));
        }
    }

    Ok((records_in_file, file_stats))
}
