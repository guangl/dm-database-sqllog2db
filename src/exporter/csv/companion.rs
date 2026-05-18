use super::super::ensure_parent_dir;
use crate::error::{Error, ExportError, Result};
use log::info;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use super::writer::write_csv_escaped;

/// 将 I/O 错误包装为 `ExportError::WriteFailed`
#[inline]
fn io_err(path: &Path, reason: String) -> Error {
    Error::Export(ExportError::WriteFailed {
        path: path.to_path_buf(),
        reason,
    })
}

/// 将单行模板统计序列化到 `buf`（`template_key` 含双引号包裹 + CSV 转义，数值用 itoa）
fn format_companion_row(
    buf: &mut Vec<u8>,
    itoa_buf: &mut itoa::Buffer,
    s: &crate::pipeline::TemplateStats,
) {
    buf.clear();
    buf.push(b'"');
    write_csv_escaped(buf, s.template_key.as_bytes());
    buf.push(b'"');
    buf.push(b',');
    buf.extend_from_slice(itoa_buf.format(s.count).as_bytes());
    buf.push(b',');
    buf.extend_from_slice(itoa_buf.format(s.avg_us).as_bytes());
    buf.push(b',');
    buf.extend_from_slice(itoa_buf.format(s.min_us).as_bytes());
    buf.push(b',');
    buf.extend_from_slice(itoa_buf.format(s.max_us).as_bytes());
    buf.push(b',');
    buf.extend_from_slice(itoa_buf.format(s.p50_us).as_bytes());
    buf.push(b',');
    buf.extend_from_slice(itoa_buf.format(s.p95_us).as_bytes());
    buf.push(b',');
    buf.extend_from_slice(itoa_buf.format(s.p99_us).as_bytes());
    buf.push(b',');
    buf.push(b'"');
    write_csv_escaped(buf, s.first_seen.as_bytes());
    buf.push(b'"');
    buf.push(b',');
    buf.push(b'"');
    write_csv_escaped(buf, s.last_seen.as_bytes());
    buf.push(b'"');
    buf.push(b'\n');
}

/// 将模板统计写入伴随 CSV 文件（D-10：始终覆盖写入）
pub(crate) fn write_companion_rows(
    path: &Path,
    stats: &[crate::pipeline::TemplateStats],
) -> Result<()> {
    ensure_parent_dir(path).map_err(|e| io_err(path, format!("create dir failed: {e}")))?;
    let file =
        File::create(path).map_err(|e| io_err(path, format!("create companion failed: {e}")))?;
    let mut writer = BufWriter::new(file);
    writer
        .write_all(
            b"template_key,count,avg_us,min_us,max_us,p50_us,p95_us,p99_us,first_seen,last_seen\n",
        )
        .map_err(|e| io_err(path, format!("write header failed: {e}")))?;
    let mut itoa_buf = itoa::Buffer::new();
    let mut line_buf: Vec<u8> = Vec::with_capacity(512);
    for s in stats {
        format_companion_row(&mut line_buf, &mut itoa_buf, s);
        writer
            .write_all(&line_buf)
            .map_err(|e| io_err(path, format!("write row failed: {e}")))?;
    }
    writer
        .flush()
        .map_err(|e| io_err(path, format!("flush failed: {e}")))?;
    Ok(())
}

/// 将模板统计写入 CSV 输出，通过 `csv_output_path` 指定路径（D-10：显式路径）。
/// `_sqlite_table_name` 在此实现中不适用。
pub(super) fn write_template_stats(
    stats: &[crate::pipeline::TemplateStats],
    csv_output_path: Option<&str>,
) -> Result<()> {
    let Some(path_str) = csv_output_path else {
        return Ok(());
    };
    if path_str.trim().is_empty() {
        return Ok(());
    }
    let path = Path::new(path_str);
    write_companion_rows(path, stats)?;
    info!("Template stats CSV written: {}", path.display());
    Ok(())
}
