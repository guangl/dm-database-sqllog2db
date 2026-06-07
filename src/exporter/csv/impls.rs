use super::super::{ExportStats, Exporter};
use super::exporter::{CsvExporter, open_for_write, writer_ref};
use super::writer::{write_record, write_record_preparsed};
use crate::error::{Error, ExportError, Result};
use dm_database_parser_sqllog::Sqllog;
use std::io::{BufWriter, Write};

impl Exporter for CsvExporter {
    fn initialize(&mut self) -> Result<()> {
        let (file, append_mode) = open_for_write(&self.path, self.write_mode)?;

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
                    path: self.path.clone(),
                    reason: format!("write header failed: {e}"),
                })
            })?;
        }

        self.writer = Some(writer);
        Ok(())
    }

    fn export(&mut self, sqllog: &Sqllog) -> Result<()> {
        let writer = writer_ref(&mut self.writer, &self.path)?;
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

    fn export_one_normalized(&mut self, sqllog: &Sqllog, normalized: Option<&str>) -> Result<()> {
        let writer = writer_ref(&mut self.writer, &self.path)?;
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
        sqllog: &Sqllog,
        include_pm: bool,
        normalized: Option<&str>,
    ) -> Result<()> {
        let writer = writer_ref(&mut self.writer, &self.path)?;
        write_record_preparsed(
            &mut self.itoa_buf,
            &mut self.line_buf,
            sqllog,
            writer,
            &self.path,
            self.normalize,
            normalized,
            self.field_mask,
            &self.ordered_indices,
            include_pm,
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
}

impl Drop for CsvExporter {
    fn drop(&mut self) {
        if self.writer.is_some() {
            let _ = self.finalize();
        }
    }
}
