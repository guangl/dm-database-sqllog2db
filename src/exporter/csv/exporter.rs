use super::super::ExportStats;
use super::super::ensure_parent_dir;
use crate::config;
use crate::error::{Error, ExportError, Result};
use std::fs::{File, OpenOptions};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WriteMode {
    Truncate,
    Append,
}

pub struct CsvExporter {
    pub(super) path: PathBuf,
    pub(super) write_mode: WriteMode,
    pub(super) writer: Option<BufWriter<File>>,
    pub(super) stats: ExportStats,
    pub(super) itoa_buf: itoa::Buffer,
    pub(super) line_buf: Vec<u8>,
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
            e.write_mode = WriteMode::Append;
        } else if config.overwrite {
            e.write_mode = WriteMode::Truncate;
        }
        e.include_performance_metrics = config.include_performance_metrics;
        e
    }

    pub(super) fn build_header(&self) -> Vec<u8> {
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

pub(super) fn writer_ref<'a>(
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

pub(super) fn open_for_write(path: &Path, write_mode: WriteMode) -> Result<(File, bool)> {
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
            .truncate(write_mode == WriteMode::Truncate)
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
