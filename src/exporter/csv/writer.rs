use super::super::{f32_ms_to_i64, strip_ip_prefix};
use crate::error::{Error, ExportError, Result};
use dm_database_parser_sqllog::{MetaParts, PerformanceMetrics, Sqllog};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// 将字节序列写入 `buf`，对其中的 `"` 字符进行 CSV 转义（变为 `""`）。
/// 使用 memchr 跳过无引号的大段内容，避免逐字节循环。
#[inline]
pub(crate) fn write_csv_escaped(buf: &mut Vec<u8>, bytes: &[u8]) {
    let mut remaining = bytes;
    while let Some(pos) = memchr::memchr(b'"', remaining) {
        buf.extend_from_slice(&remaining[..=pos]); // 含引号本身
        buf.push(b'"'); // 转义第二个引号
        remaining = &remaining[pos + 1..];
    }
    buf.extend_from_slice(remaining);
}

/// 热路径：使用预解析的 `MetaParts` 和 `PerformanceMetrics` 直接格式化并写入。
/// 接收各字段的独立可变引用，允许 Rust 同时分开借用 self 的多个字段。
#[inline]
pub(super) fn write_record_preparsed(
    itoa_buf: &mut itoa::Buffer,
    line_buf: &mut Vec<u8>,
    sqllog: &Sqllog<'_>,
    meta: &MetaParts<'_>,
    pm: &PerformanceMetrics<'_>,
    writer: &mut BufWriter<File>,
    path: &Path,
    normalize: bool,
    normalized_sql: Option<&str>,
    field_mask: crate::pipeline::FieldMask,
    ordered_indices: &[usize],
    include_performance_metrics: bool,
) -> Result<()> {
    line_buf.clear();
    let sql_len = pm.sql.len();
    let ns_len = if normalize {
        normalized_sql.map_or(0, str::len)
    } else {
        0
    };
    let needed = 128 + sql_len + ns_len;
    if line_buf.capacity() < needed {
        line_buf.reserve(needed - line_buf.len());
    }

    // 全量掩码快速路径：所有字段直接顺序写入，无分支判断
    if field_mask == crate::pipeline::FieldMask::ALL {
        line_buf.extend_from_slice(sqllog.ts.as_ref().as_bytes());
        line_buf.push(b',');
        line_buf.extend_from_slice(itoa_buf.format(meta.ep).as_bytes());
        line_buf.push(b',');
        line_buf.extend_from_slice(meta.sess_id.as_ref().as_bytes());
        line_buf.push(b',');
        line_buf.extend_from_slice(meta.thrd_id.as_ref().as_bytes());
        line_buf.push(b',');
        line_buf.extend_from_slice(meta.username.as_ref().as_bytes());
        line_buf.push(b',');
        line_buf.extend_from_slice(meta.trxid.as_ref().as_bytes());
        line_buf.push(b',');
        line_buf.extend_from_slice(meta.statement.as_ref().as_bytes());
        line_buf.push(b',');
        line_buf.extend_from_slice(meta.appname.as_ref().as_bytes());
        line_buf.push(b',');
        line_buf.extend_from_slice(strip_ip_prefix(meta.client_ip.as_ref()).as_bytes());
        line_buf.push(b',');
        if let Some(tag) = &sqllog.tag {
            line_buf.extend_from_slice(tag.as_ref().as_bytes());
        }
        line_buf.push(b',');
        line_buf.push(b'"');
        write_csv_escaped(line_buf, pm.sql.as_bytes());
        line_buf.push(b'"');
        if include_performance_metrics {
            line_buf.push(b',');
            if pm.exec_id != 0 || pm.exectime > 0.0 {
                line_buf.extend_from_slice(itoa_buf.format(f32_ms_to_i64(pm.exectime)).as_bytes());
                line_buf.push(b',');
                line_buf.extend_from_slice(itoa_buf.format(i64::from(pm.rowcount)).as_bytes());
                line_buf.push(b',');
                line_buf.extend_from_slice(itoa_buf.format(pm.exec_id).as_bytes());
            } else {
                line_buf.extend_from_slice(b",,");
            }
        }
        if normalize {
            line_buf.push(b',');
            if let Some(ns) = normalized_sql {
                line_buf.push(b'"');
                write_csv_escaped(line_buf, ns.as_bytes());
                line_buf.push(b'"');
            }
        }
    } else {
        // 投影路径：按 ordered_indices 指定的字段顺序写入
        let mut need_sep = false;

        macro_rules! w_sep {
            () => {
                if need_sep {
                    line_buf.push(b',');
                }
                need_sep = true;
            };
        }

        let has_metrics = pm.exec_id != 0 || pm.exectime > 0.0;
        for &idx in ordered_indices {
            match idx {
                0 => {
                    w_sep!();
                    line_buf.extend_from_slice(sqllog.ts.as_ref().as_bytes());
                }
                1 => {
                    w_sep!();
                    line_buf.extend_from_slice(itoa_buf.format(meta.ep).as_bytes());
                }
                2 => {
                    w_sep!();
                    line_buf.extend_from_slice(meta.sess_id.as_ref().as_bytes());
                }
                3 => {
                    w_sep!();
                    line_buf.extend_from_slice(meta.thrd_id.as_ref().as_bytes());
                }
                4 => {
                    w_sep!();
                    line_buf.extend_from_slice(meta.username.as_ref().as_bytes());
                }
                5 => {
                    w_sep!();
                    line_buf.extend_from_slice(meta.trxid.as_ref().as_bytes());
                }
                6 => {
                    w_sep!();
                    line_buf.extend_from_slice(meta.statement.as_ref().as_bytes());
                }
                7 => {
                    w_sep!();
                    line_buf.extend_from_slice(meta.appname.as_ref().as_bytes());
                }
                8 => {
                    w_sep!();
                    line_buf.extend_from_slice(strip_ip_prefix(meta.client_ip.as_ref()).as_bytes());
                }
                9 => {
                    w_sep!();
                    if let Some(tag) = &sqllog.tag {
                        line_buf.extend_from_slice(tag.as_ref().as_bytes());
                    }
                }
                10 => {
                    w_sep!();
                    line_buf.push(b'"');
                    write_csv_escaped(line_buf, pm.sql.as_bytes());
                    line_buf.push(b'"');
                }
                11 => {
                    if !include_performance_metrics {
                        continue;
                    }
                    w_sep!();
                    if has_metrics {
                        line_buf.extend_from_slice(
                            itoa_buf.format(f32_ms_to_i64(pm.exectime)).as_bytes(),
                        );
                    }
                }
                12 => {
                    if !include_performance_metrics {
                        continue;
                    }
                    w_sep!();
                    if has_metrics {
                        line_buf
                            .extend_from_slice(itoa_buf.format(i64::from(pm.rowcount)).as_bytes());
                    }
                }
                13 => {
                    if !include_performance_metrics {
                        continue;
                    }
                    w_sep!();
                    if has_metrics {
                        line_buf.extend_from_slice(itoa_buf.format(pm.exec_id).as_bytes());
                    }
                }
                // D-03：normalize=false 时跳过 normalized_sql，与 header 逻辑一致
                14 if normalize => {
                    w_sep!();
                    if let Some(ns) = normalized_sql {
                        line_buf.push(b'"');
                        write_csv_escaped(line_buf, ns.as_bytes());
                        line_buf.push(b'"');
                    }
                }
                _ => {}
            }
        }
        // 消费 need_sep，避免"最后一次赋值从未被读取"的编译警告
        let _ = need_sep;
    }

    line_buf.push(b'\n');

    writer.write_all(line_buf).map_err(|e| {
        Error::Export(ExportError::WriteFailed {
            path: path.to_path_buf(),
            reason: format!("write failed: {e}"),
        })
    })
}

/// 兼容路径：从 `Sqllog` 内部解析再转调热路径（测试/批量导出使用）。
#[inline]
pub(super) fn write_record(
    itoa_buf: &mut itoa::Buffer,
    line_buf: &mut Vec<u8>,
    sqllog: &Sqllog<'_>,
    writer: &mut BufWriter<File>,
    path: &Path,
    normalize: bool,
    normalized_sql: Option<&str>,
    field_mask: crate::pipeline::FieldMask,
    ordered_indices: &[usize],
    include_performance_metrics: bool,
) -> Result<()> {
    let meta = sqllog.parse_meta();
    let pm = if include_performance_metrics {
        sqllog.parse_performance_metrics()
    } else {
        PerformanceMetrics {
            sql: sqllog.body(),
            exectime: 0.0,
            rowcount: 0,
            exec_id: 0,
        }
    };
    write_record_preparsed(
        itoa_buf,
        line_buf,
        sqllog,
        &meta,
        &pm,
        writer,
        path,
        normalize,
        normalized_sql,
        field_mask,
        ordered_indices,
        include_performance_metrics,
    )
}
