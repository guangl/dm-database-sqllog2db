use super::super::{ExportStats, Exporter};
use super::exporter::{CsvExporter, WriteMode, open_for_write, writer_ref};
use super::writer::write_record_preparsed;
use crate::error::{Error, ExportError, Result};
use dm_database_parser_sqllog::Sqllog;
use std::io::{BufWriter, Write};

impl Exporter for CsvExporter {
    fn initialize(&mut self) -> Result<()> {
        // 启用拆分时从编号 1 开始，并清理上一轮遗留的拆分文件
        if self.max_rows_per_file.is_some() {
            self.file_index = 1;
            if self.write_mode == WriteMode::Truncate {
                self.remove_stale_split_files();
            }
        }

        let current_path = self.current_file_path();
        let (file, append_mode) = open_for_write(&current_path, self.write_mode)?;

        // Determine whether to write a header AFTER opening the file, using the
        // actual file size rather than a pre-open exists() check. This eliminates
        // the TOCTOU window where a concurrent writer could create the file between
        // exists() and open(), causing a duplicate header row to be appended.
        // If metadata() fails (e.g. /dev/null), write the header to be safe.
        let file_is_empty = file.metadata().map_or(true, |meta| meta.len() == 0);

        let mut writer = BufWriter::with_capacity(16 * 1024 * 1024, file);

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
        self.rows_in_file = 0;
        Ok(())
    }

    fn export(&mut self, sqllog: &Sqllog) -> Result<()> {
        let path = self.current_file_path();
        let writer = writer_ref(&mut self.writer, &path)?;
        write_record_preparsed(
            &mut self.itoa_buf,
            &mut self.line_buf,
            sqllog,
            writer,
            &path,
            self.normalize,
            None,
            self.field_mask,
            &self.ordered_indices,
            self.include_performance_metrics,
        )?;
        self.stats.record_success();
        self.rows_in_file += 1;
        self.maybe_rotate()?;
        Ok(())
    }

    fn export_one_normalized(&mut self, sqllog: &Sqllog, normalized: Option<&str>) -> Result<()> {
        let path = self.current_file_path();
        let writer = writer_ref(&mut self.writer, &path)?;
        write_record_preparsed(
            &mut self.itoa_buf,
            &mut self.line_buf,
            sqllog,
            writer,
            &path,
            self.normalize,
            normalized,
            self.field_mask,
            &self.ordered_indices,
            self.include_performance_metrics,
        )?;
        self.stats.record_success();
        self.rows_in_file += 1;
        self.maybe_rotate()?;
        Ok(())
    }

    fn export_one_preparsed(
        &mut self,
        sqllog: &Sqllog,
        include_pm: bool,
        normalized: Option<&str>,
    ) -> Result<()> {
        let path = self.current_file_path();
        let writer = writer_ref(&mut self.writer, &path)?;
        write_record_preparsed(
            &mut self.itoa_buf,
            &mut self.line_buf,
            sqllog,
            writer,
            &path,
            self.normalize,
            normalized,
            self.field_mask,
            &self.ordered_indices,
            include_pm,
        )?;
        self.stats.record_success();
        self.rows_in_file += 1;
        self.maybe_rotate()?;
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

// ── 文件拆分辅助方法 ──────────────────────────────────────────────────────────

impl CsvExporter {
    /// 检查当前文件是否已写满，若是则滚动到下一个文件。
    fn maybe_rotate(&mut self) -> Result<()> {
        let Some(max_rows) = self.max_rows_per_file else { return Ok(()); };
        if self.rows_in_file < max_rows {
            return Ok(());
        }
        self.rotate_file()
    }

    /// 关闭当前文件，递增序号，创建新文件并写入表头。
    fn rotate_file(&mut self) -> Result<()> {
        // 刷新并关闭当前 writer
        if let Some(mut writer) = self.writer.take() {
            let old_path = self.current_file_path();
            writer.flush().map_err(|e| {
                Error::Export(ExportError::WriteFailed {
                    path: old_path,
                    reason: format!("flush before rotate failed: {e}"),
                })
            })?;
        }

        self.file_index += 1;
        self.rows_in_file = 0;

        let new_path = self.current_file_path();
        let (file, _) = open_for_write(&new_path, WriteMode::Truncate)?;
        let mut writer = BufWriter::with_capacity(16 * 1024 * 1024, file);

        let header = self.build_header();
        writer.write_all(&header).map_err(|e| {
            Error::Export(ExportError::WriteFailed {
                path: new_path,
                reason: format!("write header failed: {e}"),
            })
        })?;

        self.writer = Some(writer);
        Ok(())
    }
}
