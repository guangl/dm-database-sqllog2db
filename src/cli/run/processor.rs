use crate::color;
use crate::error::Result;
use crate::exporter::ExporterManager;
use crate::pipeline::normalizer::ParamBuffer;
use crate::pipeline::{CompiledSqlFilters, Pipeline};
use dm_database_parser_sqllog::LogParser;
use indicatif::{HumanCount, ProgressBar};
use log::info;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// 处理单个日志文件，返回本文件实际导出的记录数。
///
/// `limit`: 最多再导出多少条记录（跨文件的剩余配额），`None` 表示不限制。
/// `reset_pb`: 是否在文件开始时重置进度条计数；并行模式传 `false`，避免多线程互相重置。
pub(super) fn process_log_file(
    file_path: &str,
    file_index: usize,
    total_files: usize,
    exporter_manager: &mut ExporterManager,
    pipeline: &Pipeline,
    pb: &ProgressBar,
    limit: Option<usize>,
    interrupted: &Arc<AtomicBool>,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    params_buffer: &mut ParamBuffer,
    ns_scratch: &mut Vec<u8>,
    reset_pb: bool,
    sql_record_filter: Option<&CompiledSqlFilters>,
) -> Result<usize> {
    // 清除上一个文件留下的残余参数，同时复用已分配的 HashMap 容量。
    params_buffer.clear();

    // 从导出器读取性能指标标志：CSV 关闭时跳过 parse_performance_metrics()（D-05/D-06）
    let include_pm = exporter_manager.csv_include_performance_metrics();

    let file_start = Instant::now();

    let file_name = std::path::Path::new(file_path).file_name().map_or_else(
        || file_path.to_string(),
        |n| n.to_string_lossy().into_owned(),
    );

    if reset_pb {
        pb.set_prefix(format!("{file_index}/{total_files}"));
        pb.set_message(file_name.clone());
        pb.reset();
    }

    let parser = LogParser::from_path(file_path).map_err(|e| {
        crate::error::Error::Parser(crate::error::ParserError::InvalidPath {
            path: file_path.into(),
            reason: format!("{e}"),
        })
    })?;

    let mut records_in_file = 0usize;
    let mut errors_in_file = 0usize;
    // 用于攒批更新进度条，避免每条记录都触发原子操作
    let mut pb_pending: u64 = 0;

    'outer: for result in parser.iter() {
        match result {
            Ok(record) => {
                // 管线为空：零开销快速路径，所有记录都通过，不提前解析 meta。
                // 管线非空：提前解析 meta，与管线过滤器共享，消除 FilterProcessor
                //           内部的重复 parse_meta() 调用（对 pipeline_passthrough
                //           场景可减少约 50% 的 parse_meta 调用次数）。
                let (passes, cached_meta) = if pipeline.is_empty() {
                    (true, None)
                } else {
                    let meta = record.parse_meta();
                    let ok = pipeline.run_with_meta(&record, &meta);
                    (ok, Some(meta))
                };

                // PARAMS 记录（无 tag）在 do_normalize 时无论是否通过过滤都必须
                // 更新 params_buffer，以便后续匹配 DML 记录能正确替换参数。
                let needs_pm = passes || (do_normalize && record.tag.is_none());
                if needs_pm {
                    // 无管线时首次解析 meta；有管线时复用已解析结果，零额外开销。
                    let meta = cached_meta.unwrap_or_else(|| record.parse_meta());

                    if passes {
                        // DML 或通过过滤的 PARAMS：CSV 关闭性能指标时合成空 pm，
                        // 跳过 find_indicators_split（D-05/D-06）；SQL 字段来自 record.body()。
                        let pm = if include_pm {
                            record.parse_performance_metrics()
                        } else {
                            dm_database_parser_sqllog::PerformanceMetrics {
                                sql: record.body(),
                                exectime: 0.0,
                                rowcount: 0,
                                exec_id: 0,
                            }
                        };

                        // SQL 记录级过滤：只对 DML 记录（有 tag）生效，PARAMS 记录始终通过。
                        // 被过滤掉的 DML 直接丢弃，不影响 params_buffer。
                        let sql_filter_pass = sql_record_filter
                            .is_none_or(|f| record.tag.is_none() || f.matches(pm.sql.as_ref()));
                        if sql_filter_pass {
                            // 快速路径：params_buffer 为空且当前是 DML 记录（有 tag），
                            // 则不可能存在待替换参数，完全跳过 compute_normalized。
                            let ns = if do_normalize
                                && (!params_buffer.is_empty() || record.tag.is_none())
                            {
                                crate::pipeline::compute_normalized(
                                    &record,
                                    &meta,
                                    pm.sql.as_ref(),
                                    params_buffer,
                                    placeholder_override,
                                    ns_scratch,
                                )
                            } else {
                                None
                            };

                            // 先检查配额，再聚合（CR-02：避免对未导出记录计入统计）
                            if let Some(remaining) = limit {
                                if records_in_file >= remaining {
                                    break 'outer;
                                }
                            }

                            exporter_manager.export_one_preparsed(&record, &meta, &pm, ns)?;
                            records_in_file += 1;
                            pb_pending += 1;

                            // 每 4096 条更新一次进度条（减少原子操作频率）
                            if pb_pending >= 4096 {
                                pb.inc(pb_pending);
                                pb_pending = 0;
                            }

                            // 每 1024 条检查一次中断信号
                            if records_in_file.trailing_zeros() >= 10
                                && interrupted.load(Ordering::Relaxed)
                            {
                                break 'outer;
                            }
                        }
                    } else {
                        // 被过滤掉的 PARAMS 记录（needs_pm 成立说明 do_normalize &&
                        // record.tag.is_none() 为真）：对 PARAMS 记录而言
                        // pm.sql ≡ record.body()，直接复用，省去 parse_performance_metrics()。
                        crate::pipeline::compute_normalized(
                            &record,
                            &meta,
                            record.body().as_ref(),
                            params_buffer,
                            placeholder_override,
                            ns_scratch,
                        );
                    }
                }
            }
            Err(e) => {
                errors_in_file += 1;
                log::warn!("{file_path} | {e:?}");
            }
        }
    }

    // 将剩余未上报的进度刷新到进度条
    if pb_pending > 0 {
        pb.inc(pb_pending);
    }

    let elapsed = file_start.elapsed().as_secs_f64();
    info!(
        "File {file_path}: {records_in_file} records, {errors_in_file} errors, total {elapsed:.2}s",
    );

    let errors_label = if errors_in_file > 0 {
        color::yellow(format!(", {errors_in_file} errors"))
    } else {
        String::new()
    };
    pb.println(format!(
        "{} [{file_index}/{total_files}] {file_path} — {}{errors_label}, {elapsed:.2}s",
        color::green("✓"),
        color::green(HumanCount(records_in_file as u64)),
    ));

    Ok(records_in_file)
}
