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
    /// 每文件最大行数。`None` = 单文件输出（原行为）。
    pub(super) max_rows_per_file: Option<usize>,
    /// 当前拆分文件已写入行数。
    pub(super) rows_in_file: usize,
    /// 当前拆分文件序号（0 = 未启用拆分，1+ = 拆分文件编号）。
    pub(super) file_index: usize,
}

impl std::fmt::Debug for CsvExporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CsvExporter")
            .field("path", &self.path)
            .field("stats", &self.stats)
            .field("max_rows_per_file", &self.max_rows_per_file)
            .field("file_index", &self.file_index)
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
            // 典型 DaMeng SQL + 字段开销约 1–4KB；writer.rs 的动态 reserve 兜底更长 SQL
            line_buf: Vec::with_capacity(4096),
            normalize: true,
            field_mask: crate::pipeline::FieldMask::ALL,
            ordered_indices: (0..crate::pipeline::FIELD_NAMES.len()).collect(),
            include_performance_metrics: true,
            max_rows_per_file: None,
            rows_in_file: 0,
            file_index: 0,
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
        e.max_rows_per_file = config.max_rows_per_file;
        e
    }

    /// 返回当前文件在磁盘上的完整路径。
    pub(super) fn current_file_path(&self) -> PathBuf {
        self.file_path_for_index(self.file_index)
    }

    /// 拆解输出路径为 `(父目录, 文件名主干, 扩展名)`，供拆分文件命名/清理复用。
    fn split_path_parts(&self) -> (PathBuf, String, String) {
        let parent = self
            .path
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let stem = self
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("sqllog")
            .to_string();
        let ext = self
            .path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("csv")
            .to_string();
        (parent, stem, ext)
    }

    /// 根据文件序号计算路径：
    /// - `index == 0` → 返回 `self.path`（单文件模式或未启用拆分时的路径）
    /// - `index >= 1` → 返回 `{parent}/{stem}_{index}.{ext}`
    fn file_path_for_index(&self, file_index: usize) -> PathBuf {
        if file_index == 0 || self.max_rows_per_file.is_none() {
            return self.path.clone();
        }
        let (parent, stem, ext) = self.split_path_parts();
        parent.join(format!("{stem}_{file_index}.{ext}"))
    }

    /// overwrite 模式下清理上一次运行遗留的拆分文件。
    ///
    /// 扫描父目录并删除所有形如 `{stem}_{数字}.{ext}` 的文件，不依赖序号连续，
    /// 因此上一轮留下的编号空洞（如缺 `_3` 但有 `_4`）也能被完全清理。
    pub(super) fn remove_stale_split_files(&self) {
        let (parent, stem, ext) = self.split_path_parts();
        let prefix = format!("{stem}_");
        let suffix = format!(".{ext}");
        let Ok(entries) = std::fs::read_dir(&parent) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            // 仅匹配 `{stem}_<纯数字>.{ext}`：主文件 `{stem}.{ext}`（无下划线段）
            // 与用户自有的 `{stem}_final.{ext}` 等非数字后缀都不会被误删。
            let Some(mid) = name
                .strip_prefix(&prefix)
                .and_then(|s| s.strip_suffix(&suffix))
            else {
                continue;
            };
            if !mid.is_empty() && mid.bytes().all(|b| b.is_ascii_digit()) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
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
