//! CSV export: lifecycle and field serialization.
use super::ExportStats;
use super::Exporter;
use super::ensure_parent_dir;
use super::{f32_ms_to_i64, strip_ip_prefix};
use crate::config;
use crate::error::{Error, ExportError, Result};
use crate::model::LogRecord;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriteMode {
    Truncate,
    Append,
}

pub struct CsvExporter {
    path: PathBuf,
    write_mode: WriteMode,
    writer: Option<BufWriter<File>>,
    stats: ExportStats,
    itoa_buf: itoa::Buffer,
    line_buf: Vec<u8>,
    pub(crate) normalize: bool,
    pub(crate) field_mask: crate::pipeline::FieldMask,
    pub(crate) ordered_indices: Vec<usize>,
    pub(crate) include_performance_metrics: bool,
}

impl std::fmt::Debug for CsvExporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CsvExporter")
            .field("path", &self.path)
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}

impl CsvExporter {
    #[must_use]
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            write_mode: WriteMode::Truncate,
            writer: None,
            stats: ExportStats::new(),
            itoa_buf: itoa::Buffer::new(),
            // 典型 DaMeng SQL + 字段开销约 1–4KB；写出时的动态 reserve 兜底更长 SQL
            line_buf: Vec::with_capacity(4096),
            normalize: true,
            field_mask: crate::pipeline::FieldMask::ALL,
            ordered_indices: (0..crate::pipeline::FIELD_NAMES.len()).collect(),
            include_performance_metrics: true,
        }
    }

    #[must_use]
    pub fn from_config(config: &config::CsvExporterConfig) -> Self {
        let mut e = Self::new(&config.file);
        if config.append {
            e.write_mode = WriteMode::Append;
        }
        e.include_performance_metrics = config.include_performance_metrics;
        e
    }

    /// 返回当前文件在磁盘上的完整路径。
    fn current_file_path(&self) -> PathBuf {
        self.path.clone()
    }

    fn build_header(&self) -> Vec<u8> {
        use crate::pipeline::FIELD_NAMES;
        let mut header = Vec::with_capacity(128);
        let mut first = true;
        for &idx in &self.ordered_indices {
            if idx == 14 && !self.normalize {
                continue;
            }
            if matches!(idx, 11..=13) && !self.include_performance_metrics {
                continue;
            }
            if !first {
                header.push(b',');
            }
            first = false;
            header.extend_from_slice(FIELD_NAMES[idx].as_bytes());
        }
        header.push(b'\n');
        header
    }
}

fn writer_ref<'a>(
    w: &'a mut Option<BufWriter<File>>,
    path: &Path,
) -> Result<&'a mut BufWriter<File>> {
    w.as_mut().ok_or_else(|| {
        Error::Export(ExportError::WriteFailed {
            path: path.to_path_buf(),
            reason: "not initialized".to_string(),
        })
    })
}

fn open_for_write(path: &Path, write_mode: WriteMode) -> Result<(File, bool)> {
    ensure_parent_dir(path).map_err(|e| {
        Error::Export(ExportError::WriteFailed {
            path: path.to_path_buf(),
            reason: format!("create dir failed: {e}"),
        })
    })?;

    let append_mode = write_mode == WriteMode::Append;

    let file = if append_mode {
        OpenOptions::new().create(true).append(true).open(path)
    } else {
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
    }
    .map_err(|e| {
        Error::Export(ExportError::WriteFailed {
            path: path.to_path_buf(),
            reason: format!("open failed: {e}"),
        })
    })?;

    Ok((file, append_mode))
}

impl Exporter for CsvExporter {
    fn initialize(&mut self) -> Result<()> {
        let current_path = self.current_file_path();
        let (file, append_mode) = open_for_write(&current_path, self.write_mode)?;

        // Determine whether to write a header AFTER opening the file, using the
        // actual file size rather than a pre-open exists() check. This eliminates
        // the TOCTOU window where a concurrent writer could create the file between
        // exists() and open(), causing a duplicate header row to be appended.
        // If metadata() fails (e.g. /dev/null), write the header to be safe.
        let file_is_empty = file.metadata().map_or(true, |meta| meta.len() == 0);

        let mut writer = BufWriter::with_capacity(1024 * 1024, file);

        if !append_mode || file_is_empty {
            let header = self.build_header();
            writer.write_all(&header).map_err(|e| {
                Error::Export(ExportError::WriteFailed {
                    path: current_path,
                    reason: format!("write header failed: {e}"),
                })
            })?;
        }

        self.writer = Some(writer);
        Ok(())
    }

    fn export(&mut self, sqllog: &LogRecord) -> Result<()> {
        self.export_one_preparsed(sqllog, self.include_performance_metrics, None)
    }

    fn export_one_normalized(
        &mut self,
        sqllog: &LogRecord,
        normalized: Option<&str>,
    ) -> Result<()> {
        self.export_one_preparsed(sqllog, self.include_performance_metrics, normalized)
    }

    fn export_one_preparsed(
        &mut self,
        sqllog: &LogRecord,
        include_pm: bool,
        normalized: Option<&str>,
    ) -> Result<()> {
        let path = &self.path;
        let writer = writer_ref(&mut self.writer, path)?;
        write_record_preparsed(
            &mut self.itoa_buf,
            &mut self.line_buf,
            sqllog,
            writer,
            path,
            normalized,
            &CsvLayout {
                normalize: self.normalize,
                field_mask: self.field_mask,
                ordered_indices: &self.ordered_indices,
                include_performance_metrics: include_pm,
            },
        )?;
        self.stats.record_success();
        Ok(())
    }

    fn finalize(&mut self) -> Result<()> {
        if let Some(mut writer) = self.writer.take() {
            let path = self.current_file_path();
            writer.flush().map_err(|e| {
                Error::Export(ExportError::WriteFailed {
                    path,
                    reason: format!("flush failed: {e}"),
                })
            })?;
        }
        Ok(())
    }

    fn stats_snapshot(&self) -> Option<ExportStats> {
        Some(self.stats)
    }
}

impl Drop for CsvExporter {
    fn drop(&mut self) {
        if self.writer.is_some() {
            let _ = self.finalize();
        }
    }
}

#[cfg(all(test, feature = "sqllog"))]
#[path = "../../../tests/unit/infrastructure/output/csv.rs"]
mod tests;

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

/// ALL 快速路径：写入所有字段（固定顺序，无字段掩码开销）。
#[inline]
fn write_all_fields(
    itoa_buf: &mut itoa::Buffer,
    line_buf: &mut Vec<u8>,
    sqllog: &LogRecord,
    normalize: bool,
    normalized_sql: Option<&str>,
    include_performance_metrics: bool,
) {
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
        if sqllog.exec_id != 0 || sqllog.exectime > 0.0 || sqllog.rowcount != 0 {
            line_buf.extend_from_slice(itoa_buf.format(f32_ms_to_i64(sqllog.exectime)).as_bytes());
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
}

/// 自定义字段路径：按 `ordered_indices` 写入选定字段。
#[inline]
fn write_selected_fields(
    itoa_buf: &mut itoa::Buffer,
    line_buf: &mut Vec<u8>,
    sqllog: &LogRecord,
    normalize: bool,
    normalized_sql: Option<&str>,
    ordered_indices: &[usize],
    include_performance_metrics: bool,
) {
    let mut need_sep = false;
    let sep = |buf: &mut Vec<u8>, sep_flag: &mut bool| {
        if *sep_flag {
            buf.push(b',');
        }
        *sep_flag = true;
    };
    let has_metrics = sqllog.exec_id != 0 || sqllog.exectime > 0.0 || sqllog.rowcount != 0;
    for &idx in ordered_indices {
        match idx {
            0 => {
                sep(line_buf, &mut need_sep);
                line_buf.extend_from_slice(sqllog.ts.as_bytes());
            }
            1 => {
                sep(line_buf, &mut need_sep);
                line_buf.extend_from_slice(itoa_buf.format(sqllog.ep).as_bytes());
            }
            2 => {
                sep(line_buf, &mut need_sep);
                line_buf.extend_from_slice(sqllog.sess_id.as_bytes());
            }
            3 => {
                sep(line_buf, &mut need_sep);
                line_buf.extend_from_slice(sqllog.thrd_id.as_bytes());
            }
            4 => {
                sep(line_buf, &mut need_sep);
                line_buf.extend_from_slice(sqllog.username.as_bytes());
            }
            5 => {
                sep(line_buf, &mut need_sep);
                line_buf.extend_from_slice(sqllog.trxid.as_bytes());
            }
            6 => {
                sep(line_buf, &mut need_sep);
                line_buf.extend_from_slice(sqllog.statement.as_bytes());
            }
            7 => {
                sep(line_buf, &mut need_sep);
                line_buf.extend_from_slice(sqllog.appname.as_bytes());
            }
            8 => {
                sep(line_buf, &mut need_sep);
                line_buf.extend_from_slice(strip_ip_prefix(&sqllog.client_ip).as_bytes());
            }
            9 => {
                sep(line_buf, &mut need_sep);
                if let Some(ref tag) = sqllog.tag {
                    line_buf.extend_from_slice(tag.as_bytes());
                }
            }
            10 => {
                sep(line_buf, &mut need_sep);
                line_buf.push(b'"');
                write_csv_escaped(line_buf, sqllog.sql.as_bytes());
                line_buf.push(b'"');
            }
            11 if include_performance_metrics => {
                sep(line_buf, &mut need_sep);
                if has_metrics {
                    line_buf.extend_from_slice(
                        itoa_buf.format(f32_ms_to_i64(sqllog.exectime)).as_bytes(),
                    );
                }
            }
            12 if include_performance_metrics => {
                sep(line_buf, &mut need_sep);
                if has_metrics {
                    line_buf
                        .extend_from_slice(itoa_buf.format(i64::from(sqllog.rowcount)).as_bytes());
                }
            }
            13 if include_performance_metrics => {
                sep(line_buf, &mut need_sep);
                if has_metrics {
                    line_buf.extend_from_slice(itoa_buf.format(sqllog.exec_id).as_bytes());
                }
            }
            14 if normalize => {
                sep(line_buf, &mut need_sep);
                if let Some(ns) = normalized_sql {
                    line_buf.push(b'"');
                    write_csv_escaped(line_buf, ns.as_bytes());
                    line_buf.push(b'"');
                }
            }
            _ => {}
        }
    }
}

/// CSV 行布局：字段投影与归一化/性能指标开关（每次写入的常量配置）。
pub(in crate::infrastructure::output::csv) struct CsvLayout<'a> {
    pub(in crate::infrastructure::output::csv) normalize: bool,
    pub(in crate::infrastructure::output::csv) field_mask: crate::pipeline::FieldMask,
    pub(in crate::infrastructure::output::csv) ordered_indices: &'a [usize],
    pub(in crate::infrastructure::output::csv) include_performance_metrics: bool,
}

/// 热路径：使用已解析的统一记录直接格式化并写入。
#[inline]
pub(in crate::infrastructure::output::csv) fn write_record_preparsed(
    itoa_buf: &mut itoa::Buffer,
    line_buf: &mut Vec<u8>,
    sqllog: &LogRecord,
    writer: &mut BufWriter<File>,
    path: &Path,
    normalized_sql: Option<&str>,
    layout: &CsvLayout<'_>,
) -> Result<()> {
    line_buf.clear();
    let ns_len = if layout.normalize {
        normalized_sql.map_or(0, str::len)
    } else {
        0
    };
    let needed = 128 + sqllog.sql.len() + ns_len;
    if line_buf.capacity() < needed {
        line_buf.reserve(needed - line_buf.len());
    }
    if layout.field_mask == crate::pipeline::FieldMask::ALL {
        write_all_fields(
            itoa_buf,
            line_buf,
            sqllog,
            layout.normalize,
            normalized_sql,
            layout.include_performance_metrics,
        );
    } else {
        write_selected_fields(
            itoa_buf,
            line_buf,
            sqllog,
            layout.normalize,
            normalized_sql,
            layout.ordered_indices,
            layout.include_performance_metrics,
        );
    }
    line_buf.push(b'\n');
    writer.write_all(line_buf).map_err(|e| {
        Error::Export(ExportError::WriteFailed {
            path: path.to_path_buf(),
            reason: format!("write failed: {e}"),
        })
    })
}
