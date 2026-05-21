use super::super::{f32_ms_to_i64, strip_ip_prefix};
use crate::error::{Error, ExportError, Result};
use dm_database_parser_sqllog::Sqllog;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// 将字节序列写入 `buf`，对其中的 `"` 字符进行 CSV 转义。
#[inline]
pub(crate) fn write_csv_escaped(buf: &mut Vec<u8>, bytes: &[u8]) {
    let mut remaining = bytes;
    while let Some(pos) = memchr::memchr(b'"', remaining) {
        buf.extend_from_slice(&remaining[..=pos]);
        buf.push(b'"');
        remaining = &remaining[pos + 1..];
    }
    buf.extend_from_slice(remaining);
}

/// 热路径：使用已解析的 `Sqllog` 直接格式化并写入。
#[inline]
pub(super) fn write_record_preparsed(
    itoa_buf: &mut itoa::Buffer,
    line_buf: &mut Vec<u8>,
    sqllog: &Sqllog,
    writer: &mut BufWriter<File>,
    path: &Path,
    normalize: bool,
    normalized_sql: Option<&str>,
    field_mask: crate::pipeline::FieldMask,
    ordered_indices: &[usize],
    include_performance_metrics: bool,
) -> Result<()> {
    line_buf.clear();
    let sql_len = sqllog.sql.len();
    let ns_len = if normalize {
        normalized_sql.map_or(0, str::len)
    } else {
        0
    };
    let needed = 128 + sql_len + ns_len;
    if line_buf.capacity() < needed {
        line_buf.reserve(needed - line_buf.len());
    }

    if field_mask == crate::pipeline::FieldMask::ALL {
        line_buf.extend_from_slice(sqllog.ts.as_bytes());
        line_buf.push(b',');
        line_buf.extend_from_slice(itoa_buf.format(sqllog.ep).as_bytes());
        line_buf.push(b',');
        line_buf.extend_from_slice(sqllog.sess_id.as_bytes());
        line_buf.push(b',');
        line_buf.extend_from_slice(sqllog.thrd_id.as_bytes());
        line_buf.push(b',');
        line_buf.extend_from_slice(sqllog.username.as_bytes());
        line_buf.push(b',');
        line_buf.extend_from_slice(sqllog.trxid.as_bytes());
        line_buf.push(b',');
        line_buf.extend_from_slice(sqllog.statement.as_bytes());
        line_buf.push(b',');
        line_buf.extend_from_slice(sqllog.appname.as_bytes());
        line_buf.push(b',');
        line_buf.extend_from_slice(strip_ip_prefix(&sqllog.client_ip).as_bytes());
        line_buf.push(b',');
        if let Some(ref tag) = sqllog.tag {
            line_buf.extend_from_slice(tag.as_bytes());
        }
        line_buf.push(b',');
        line_buf.push(b'"');
        write_csv_escaped(line_buf, sqllog.sql.as_bytes());
        line_buf.push(b'"');
        if include_performance_metrics {
            line_buf.push(b',');
            if sqllog.exec_id != 0 || sqllog.exectime > 0.0 {
                line_buf
                    .extend_from_slice(itoa_buf.format(f32_ms_to_i64(sqllog.exectime)).as_bytes());
                line_buf.push(b',');
                line_buf.extend_from_slice(itoa_buf.format(i64::from(sqllog.rowcount)).as_bytes());
                line_buf.push(b',');
                line_buf.extend_from_slice(itoa_buf.format(sqllog.exec_id).as_bytes());
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
        let mut need_sep = false;

        macro_rules! w_sep {
            () => {
                if need_sep {
                    line_buf.push(b',');
                }
                need_sep = true;
            };
        }

        let has_metrics = sqllog.exec_id != 0 || sqllog.exectime > 0.0;
        for &idx in ordered_indices {
            match idx {
                0 => {
                    w_sep!();
                    line_buf.extend_from_slice(sqllog.ts.as_bytes());
                }
                1 => {
                    w_sep!();
                    line_buf.extend_from_slice(itoa_buf.format(sqllog.ep).as_bytes());
                }
                2 => {
                    w_sep!();
                    line_buf.extend_from_slice(sqllog.sess_id.as_bytes());
                }
                3 => {
                    w_sep!();
                    line_buf.extend_from_slice(sqllog.thrd_id.as_bytes());
                }
                4 => {
                    w_sep!();
                    line_buf.extend_from_slice(sqllog.username.as_bytes());
                }
                5 => {
                    w_sep!();
                    line_buf.extend_from_slice(sqllog.trxid.as_bytes());
                }
                6 => {
                    w_sep!();
                    line_buf.extend_from_slice(sqllog.statement.as_bytes());
                }
                7 => {
                    w_sep!();
                    line_buf.extend_from_slice(sqllog.appname.as_bytes());
                }
                8 => {
                    w_sep!();
                    line_buf.extend_from_slice(strip_ip_prefix(&sqllog.client_ip).as_bytes());
                }
                9 => {
                    w_sep!();
                    if let Some(ref tag) = sqllog.tag {
                        line_buf.extend_from_slice(tag.as_bytes());
                    }
                }
                10 => {
                    w_sep!();
                    line_buf.push(b'"');
                    write_csv_escaped(line_buf, sqllog.sql.as_bytes());
                    line_buf.push(b'"');
                }
                11 => {
                    if !include_performance_metrics {
                        continue;
                    }
                    w_sep!();
                    if has_metrics {
                        line_buf.extend_from_slice(
                            itoa_buf.format(f32_ms_to_i64(sqllog.exectime)).as_bytes(),
                        );
                    }
                }
                12 => {
                    if !include_performance_metrics {
                        continue;
                    }
                    w_sep!();
                    if has_metrics {
                        line_buf.extend_from_slice(
                            itoa_buf.format(i64::from(sqllog.rowcount)).as_bytes(),
                        );
                    }
                }
                13 => {
                    if !include_performance_metrics {
                        continue;
                    }
                    w_sep!();
                    if has_metrics {
                        line_buf.extend_from_slice(itoa_buf.format(sqllog.exec_id).as_bytes());
                    }
                }
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

/// 兼容路径：从 `Sqllog` 直接写入（v1.1.0 所有字段已物化）。
#[inline]
pub(super) fn write_record(
    itoa_buf: &mut itoa::Buffer,
    line_buf: &mut Vec<u8>,
    sqllog: &Sqllog,
    writer: &mut BufWriter<File>,
    path: &Path,
    normalize: bool,
    normalized_sql: Option<&str>,
    field_mask: crate::pipeline::FieldMask,
    ordered_indices: &[usize],
    include_performance_metrics: bool,
) -> Result<()> {
    write_record_preparsed(
        itoa_buf,
        line_buf,
        sqllog,
        writer,
        path,
        normalize,
        normalized_sql,
        field_mask,
        ordered_indices,
        include_performance_metrics,
    )
}
