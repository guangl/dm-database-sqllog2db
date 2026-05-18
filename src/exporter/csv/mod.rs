use super::ensure_parent_dir;
use super::{ExportStats, Exporter};
use crate::config;
use crate::error::{Error, ExportError, Result};
use dm_database_parser_sqllog::{MetaParts, PerformanceMetrics, Sqllog};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

mod companion;
mod writer;

pub(crate) use self::companion::write_companion_rows;
use self::writer::{write_record, write_record_preparsed};

#[allow(clippy::struct_excessive_bools)]
pub struct CsvExporter {
    path: PathBuf,
    overwrite: bool,
    append: bool,
    writer: Option<BufWriter<File>>,
    stats: ExportStats,
    itoa_buf: itoa::Buffer,
    line_buf: Vec<u8>,
    pub(crate) normalize: bool,
    pub(crate) field_mask: crate::pipeline::FieldMask,
    pub(crate) ordered_indices: Vec<usize>,
    /// 是否在输出中包含性能指标列（`exec_time_ms`/`row_count`/`exec_id`）。
    /// 关闭时 header 和数据行都跳过这三列；调用方（`cli/run.rs`）也应跳过 `parse_performance_metrics()`。
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
            overwrite: false,
            append: false,
            writer: None,
            stats: ExportStats::new(),
            itoa_buf: itoa::Buffer::new(),
            // 预分配 2 KiB：覆盖典型日志行（元数据 ~120 B + SQL ~500 B + normalized ~500 B）
            // 避免前几条记录触发 Vec 扩容。clear() 保留容量，运行期自动适配长 SQL。
            line_buf: Vec::with_capacity(2048),
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
            e.append = true;
        } else {
            e.overwrite = config.overwrite;
        }
        e.include_performance_metrics = config.include_performance_metrics;
        e
    }

    /// 根据 `ordered_indices` 和 `normalize` 标志生成 CSV 头行
    fn build_header(&self) -> Vec<u8> {
        use crate::pipeline::FIELD_NAMES;
        let mut header = Vec::with_capacity(128);
        let mut first = true;
        for &idx in &self.ordered_indices {
            // idx 14 (normalized_sql) 在 normalize=false 时跳过（与全量路径行为一致）
            if idx == 14 && !self.normalize {
                continue;
            }
            // idx 11/12/13 (exectime/rowcount/exec_id) 在 include_performance_metrics=false 时跳过
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

impl Exporter for CsvExporter {
    fn initialize(&mut self) -> Result<()> {
        ensure_parent_dir(&self.path).map_err(|e| {
            Error::Export(ExportError::WriteFailed {
                path: self.path.clone(),
                reason: format!("create dir failed: {e}"),
            })
        })?;

        let append_mode = self.append;
        let file_exists = self.path.exists();

        let file = if append_mode {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
        } else {
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(self.overwrite)
                .open(&self.path)
        }
        .map_err(|e| {
            Error::Export(ExportError::WriteFailed {
                path: self.path.clone(),
                reason: format!("open failed: {e}"),
            })
        })?;

        let mut writer = BufWriter::with_capacity(16 * 1024 * 1024, file);

        if !append_mode || !file_exists {
            let header = self.build_header();
            writer.write_all(&header).map_err(|e| {
                Error::Export(ExportError::WriteFailed {
                    path: self.path.clone(),
                    reason: format!("write header failed: {e}"),
                })
            })?;
        }

        self.writer = Some(writer);
        Ok(())
    }

    fn export(&mut self, sqllog: &Sqllog<'_>) -> Result<()> {
        let writer = self.writer.as_mut().ok_or_else(|| {
            Error::Export(ExportError::WriteFailed {
                path: self.path.clone(),
                reason: "not initialized".to_string(),
            })
        })?;
        write_record(
            &mut self.itoa_buf,
            &mut self.line_buf,
            sqllog,
            writer,
            &self.path,
            self.normalize,
            None,
            self.field_mask,
            &self.ordered_indices,
            self.include_performance_metrics,
        )?;
        self.stats.record_success();
        Ok(())
    }

    fn export_one_normalized(
        &mut self,
        sqllog: &Sqllog<'_>,
        normalized: Option<&str>,
    ) -> Result<()> {
        let writer = self.writer.as_mut().ok_or_else(|| {
            Error::Export(ExportError::WriteFailed {
                path: self.path.clone(),
                reason: "not initialized".to_string(),
            })
        })?;
        write_record(
            &mut self.itoa_buf,
            &mut self.line_buf,
            sqllog,
            writer,
            &self.path,
            self.normalize,
            normalized,
            self.field_mask,
            &self.ordered_indices,
            self.include_performance_metrics,
        )?;
        self.stats.record_success();
        Ok(())
    }

    fn export_one_preparsed(
        &mut self,
        sqllog: &Sqllog<'_>,
        meta: &MetaParts<'_>,
        pm: &PerformanceMetrics<'_>,
        normalized: Option<&str>,
    ) -> Result<()> {
        let writer = self.writer.as_mut().ok_or_else(|| {
            Error::Export(ExportError::WriteFailed {
                path: self.path.clone(),
                reason: "not initialized".to_string(),
            })
        })?;
        write_record_preparsed(
            &mut self.itoa_buf,
            &mut self.line_buf,
            sqllog,
            meta,
            pm,
            writer,
            &self.path,
            self.normalize,
            normalized,
            self.field_mask,
            &self.ordered_indices,
            self.include_performance_metrics,
        )?;
        self.stats.record_success();
        Ok(())
    }

    fn finalize(&mut self) -> Result<()> {
        if let Some(mut writer) = self.writer.take() {
            writer.flush().map_err(|e| {
                Error::Export(ExportError::WriteFailed {
                    path: self.path.clone(),
                    reason: format!("flush failed: {e}"),
                })
            })?;
        }
        Ok(())
    }

    fn stats_snapshot(&self) -> Option<ExportStats> {
        Some(self.stats)
    }

    fn write_template_stats(
        &mut self,
        stats: &[crate::pipeline::TemplateStats],
        csv_output_path: Option<&str>,
        _sqlite_table_name: Option<&str>,
    ) -> Result<()> {
        companion::write_template_stats(stats, csv_output_path)
    }
}

impl Drop for CsvExporter {
    fn drop(&mut self) {
        if self.writer.is_some() {
            let _ = self.finalize();
        }
    }
}

#[cfg(test)]
mod tests;
