use super::super::{ExportStats, Exporter, ensure_parent_dir, f32_ms_to_i64, strip_ip_prefix};
use crate::config::{ParquetCompression, ParquetExporterConfig};
use crate::error::{Error, ExportError, Result};
use arrow_array::{ArrayRef, Int32Array, Int64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use dm_database_parser_sqllog::Sqllog;
use parquet::arrow::ArrowWriter;
use parquet::basic::{Compression, ZstdLevel};
use parquet::file::properties::WriterProperties;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::sync::Arc;

const MAX_BUFFERED_BYTES: usize = 64 * 1024 * 1024;

enum ColumnBuffer {
    Utf8(Vec<Option<String>>),
    Int32(Vec<Option<i32>>),
    Int64(Vec<Option<i64>>),
}

impl ColumnBuffer {
    fn with_capacity(field_index: usize, capacity: usize) -> Self {
        match field_index {
            1 => Self::Int32(Vec::with_capacity(capacity)),
            11..=13 => Self::Int64(Vec::with_capacity(capacity)),
            _ => Self::Utf8(Vec::with_capacity(capacity)),
        }
    }

    fn take_array(&mut self) -> ArrayRef {
        match self {
            Self::Utf8(values) => Arc::new(StringArray::from(std::mem::take(values))),
            Self::Int32(values) => Arc::new(Int32Array::from(std::mem::take(values))),
            Self::Int64(values) => Arc::new(Int64Array::from(std::mem::take(values))),
        }
    }
}

pub struct ParquetExporter {
    path: PathBuf,
    overwrite: bool,
    compression: ParquetCompression,
    row_group_rows: usize,
    writer: Option<ArrowWriter<File>>,
    schema: Option<SchemaRef>,
    columns: Vec<ColumnBuffer>,
    buffered_rows: usize,
    buffered_bytes: usize,
    stats: ExportStats,
    pub(crate) normalize: bool,
    pub(crate) field_mask: crate::pipeline::FieldMask,
    pub(crate) ordered_indices: Vec<usize>,
}

impl std::fmt::Debug for ParquetExporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParquetExporter")
            .field("path", &self.path)
            .field("compression", &self.compression)
            .field("row_group_rows", &self.row_group_rows)
            .field("buffered_rows", &self.buffered_rows)
            .field("buffered_bytes", &self.buffered_bytes)
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}

impl ParquetExporter {
    #[must_use]
    pub fn from_config(config: &ParquetExporterConfig) -> Self {
        Self {
            path: PathBuf::from(&config.file),
            overwrite: config.overwrite,
            compression: config.compression,
            row_group_rows: config.row_group_rows,
            writer: None,
            schema: None,
            columns: Vec::new(),
            buffered_rows: 0,
            buffered_bytes: 0,
            stats: ExportStats::new(),
            normalize: true,
            field_mask: crate::pipeline::FieldMask::ALL,
            ordered_indices: (0..crate::pipeline::FIELD_NAMES.len()).collect(),
        }
    }

    fn active_indices(&self) -> Vec<usize> {
        self.ordered_indices
            .iter()
            .copied()
            .filter(|&idx| idx != 14 || self.normalize)
            .collect()
    }

    fn build_schema(indices: &[usize]) -> SchemaRef {
        let fields = indices
            .iter()
            .map(|&idx| {
                let data_type = match idx {
                    1 => DataType::Int32,
                    11..=13 => DataType::Int64,
                    _ => DataType::Utf8,
                };
                let nullable = matches!(idx, 9 | 11..=14);
                Field::new(crate::pipeline::FIELD_NAMES[idx], data_type, nullable)
            })
            .collect::<Vec<_>>();
        Arc::new(Schema::new(fields))
    }

    fn compression(&self) -> Compression {
        match self.compression {
            ParquetCompression::Zstd => {
                Compression::ZSTD(ZstdLevel::try_new(1).expect("zstd level 1 is always valid"))
            }
            ParquetCompression::Snappy => Compression::SNAPPY,
            ParquetCompression::Uncompressed => Compression::UNCOMPRESSED,
        }
    }

    fn append_record(&mut self, sqllog: &Sqllog, normalized: Option<&str>) {
        let has_metrics = sqllog.exec_id != 0 || sqllog.exectime > 0.0 || sqllog.rowcount != 0;
        for (column, &idx) in self.columns.iter_mut().zip(&self.ordered_indices) {
            match (column, idx) {
                (ColumnBuffer::Utf8(v), 0) => v.push(Some(sqllog.ts.clone())),
                (ColumnBuffer::Int32(v), 1) => v.push(Some(i32::from(sqllog.ep))),
                (ColumnBuffer::Utf8(v), 2) => v.push(Some(sqllog.sess_id.clone())),
                (ColumnBuffer::Utf8(v), 3) => v.push(Some(sqllog.thrd_id.clone())),
                (ColumnBuffer::Utf8(v), 4) => v.push(Some(sqllog.username.clone())),
                (ColumnBuffer::Utf8(v), 5) => v.push(Some(sqllog.trxid.clone())),
                (ColumnBuffer::Utf8(v), 6) => v.push(Some(sqllog.statement.clone())),
                (ColumnBuffer::Utf8(v), 7) => v.push(Some(sqllog.appname.clone())),
                (ColumnBuffer::Utf8(v), 8) => {
                    v.push(Some(strip_ip_prefix(&sqllog.client_ip).to_string()));
                }
                (ColumnBuffer::Utf8(v), 9) => v.push(sqllog.tag.clone()),
                (ColumnBuffer::Utf8(v), 10) => v.push(Some(sqllog.sql.clone())),
                (ColumnBuffer::Int64(v), 11) => {
                    v.push(has_metrics.then(|| f32_ms_to_i64(sqllog.exectime)));
                }
                (ColumnBuffer::Int64(v), 12) => {
                    v.push(has_metrics.then(|| i64::from(sqllog.rowcount)));
                }
                (ColumnBuffer::Int64(v), 13) => v.push(has_metrics.then_some(sqllog.exec_id)),
                (ColumnBuffer::Utf8(v), 14) => {
                    v.push(normalized.map(ToString::to_string));
                }
                _ => unreachable!("column buffer type matches validated field index"),
            }
        }
        self.buffered_rows += 1;
        self.buffered_bytes += sqllog.ts.len()
            + sqllog.sess_id.len()
            + sqllog.thrd_id.len()
            + sqllog.username.len()
            + sqllog.trxid.len()
            + sqllog.statement.len()
            + sqllog.appname.len()
            + sqllog.client_ip.len()
            + sqllog.tag.as_ref().map_or(0, String::len)
            + sqllog.sql.len()
            + normalized.map_or(0, str::len)
            + 32;
    }

    fn flush_batch(&mut self) -> Result<()> {
        if self.buffered_rows == 0 {
            return Ok(());
        }
        let arrays = self
            .columns
            .iter_mut()
            .map(ColumnBuffer::take_array)
            .collect();
        let schema = self.schema.as_ref().expect("initialized schema").clone();
        let batch = RecordBatch::try_new(schema, arrays)
            .map_err(|e| self.write_error(format!("build record batch failed: {e}")))?;
        let path = self.path.clone();
        self.writer
            .as_mut()
            .ok_or_else(|| {
                Error::Export(ExportError::WriteFailed {
                    path: path.clone(),
                    reason: "not initialized".to_string(),
                })
            })?
            .write(&batch)
            .map_err(|e| {
                Error::Export(ExportError::WriteFailed {
                    path,
                    reason: format!("write row group failed: {e}"),
                })
            })?;
        self.buffered_rows = 0;
        self.buffered_bytes = 0;
        Ok(())
    }

    fn write_error(&self, reason: String) -> Error {
        Error::Export(ExportError::WriteFailed {
            path: self.path.clone(),
            reason,
        })
    }
}

impl Exporter for ParquetExporter {
    fn initialize(&mut self) -> Result<()> {
        ensure_parent_dir(&self.path).map_err(|e| self.write_error(e.to_string()))?;
        let file = OpenOptions::new()
            .write(true)
            .create(self.overwrite)
            .create_new(!self.overwrite)
            .truncate(self.overwrite)
            .open(&self.path)
            .map_err(|e| self.write_error(format!("open failed: {e}")))?;

        let indices = self.active_indices();
        self.ordered_indices = indices;
        let schema = Self::build_schema(&self.ordered_indices);
        let properties = WriterProperties::builder()
            .set_compression(self.compression())
            .set_max_row_group_row_count(Some(self.row_group_rows))
            .set_max_row_group_bytes(Some(MAX_BUFFERED_BYTES))
            .build();
        let writer = ArrowWriter::try_new(file, schema.clone(), Some(properties))
            .map_err(|e| self.write_error(format!("initialize writer failed: {e}")))?;
        self.columns = self
            .ordered_indices
            .iter()
            .map(|&idx| ColumnBuffer::with_capacity(idx, self.row_group_rows))
            .collect();
        self.schema = Some(schema);
        self.writer = Some(writer);
        Ok(())
    }

    fn export(&mut self, sqllog: &Sqllog) -> Result<()> {
        self.export_one_normalized(sqllog, None)
    }

    fn export_one_normalized(&mut self, sqllog: &Sqllog, normalized: Option<&str>) -> Result<()> {
        if self.writer.is_none() {
            return Err(self.write_error("not initialized".to_string()));
        }
        self.append_record(sqllog, normalized);
        self.stats.record_success();
        if self.buffered_rows >= self.row_group_rows || self.buffered_bytes >= MAX_BUFFERED_BYTES {
            self.flush_batch()?;
        }
        Ok(())
    }

    fn export_one_preparsed(
        &mut self,
        sqllog: &Sqllog,
        _include_pm: bool,
        normalized: Option<&str>,
    ) -> Result<()> {
        self.export_one_normalized(sqllog, normalized)
    }

    fn finalize(&mut self) -> Result<()> {
        self.flush_batch()?;
        if let Some(writer) = self.writer.take() {
            writer
                .close()
                .map_err(|e| self.write_error(format!("finalize failed: {e}")))?;
        }
        Ok(())
    }

    fn stats_snapshot(&self) -> Option<ExportStats> {
        Some(self.stats)
    }
}

impl Drop for ParquetExporter {
    fn drop(&mut self) {
        if self.writer.is_some() {
            let _ = self.finalize();
        }
    }
}
