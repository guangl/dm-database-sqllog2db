//! Structured errors, severity and bounded run statistics.

use std::fmt;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// Error severity for fatal/non-fatal classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Warning,
    Error,
    Critical,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Warning => write!(f, "WARNING"),
            Self::Error => write!(f, "ERROR"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("File error: {0}")]
    File(#[from] FileError),

    #[error("SQL log parser error: {0}")]
    Parser(#[from] ParserError),

    #[error("Export error: {0}")]
    Export(#[from] ExportError),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Interrupted by user")]
    Interrupted,
}

impl Error {
    #[must_use]
    pub fn is_fatal(&self) -> bool {
        match self {
            Error::Config(_) | Error::Io(_) | Error::Interrupted => true,
            Error::File(e) => matches!(
                e,
                FileError::AlreadyExists { .. } | FileError::CreateDirectoryFailed { .. }
            ),
            Error::Parser(e) => matches!(e, ParserError::ReadDirFailed { .. }),
            Error::Export(e) => matches!(e, ExportError::Fatal { .. }),
        }
    }

    #[must_use]
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Error::Config(_) | Error::Io(_) | Error::Interrupted => ErrorSeverity::Critical,
            Error::File(e) => match e {
                FileError::WriteFailed { .. } => ErrorSeverity::Error,
                FileError::AlreadyExists { .. } | FileError::CreateDirectoryFailed { .. } => {
                    ErrorSeverity::Critical
                }
            },
            Error::Parser(_) => ErrorSeverity::Warning,
            Error::Export(e) => match e {
                ExportError::WriteFailed { .. } => ErrorSeverity::Error,
                ExportError::Fatal { .. } => ErrorSeverity::Critical,
            },
        }
    }

    #[must_use]
    pub fn suggestion(&self) -> &str {
        match self {
            Error::Config(e) => match e {
                ConfigError::NotFound(_) => {
                    "Create a config file with 'sqllog2db init' or check the file path."
                }
                ConfigError::ParseFailed { .. } => "Check TOML syntax in the configuration file.",
                ConfigError::InvalidLogLevel { .. } => {
                    "Valid log levels: error, warn, info, debug, trace."
                }
                ConfigError::InvalidValue { .. } => {
                    "Check the field value in the configuration file."
                }
                ConfigError::NoExporters => {
                    "Enable at least one exporter: [exporter.parquet] or [exporter.csv]."
                }
            },
            Error::File(e) => match e {
                FileError::AlreadyExists { .. } => {
                    "Use --force to overwrite, or choose a different output path."
                }
                FileError::WriteFailed { .. } => "Check disk space and file permissions.",
                FileError::CreateDirectoryFailed { .. } => "Check parent directory permissions.",
            },
            Error::Parser(e) => match e {
                ParserError::PathNotFound { .. } => {
                    "Verify the log file exists at the specified path."
                }
                ParserError::InvalidPath { .. } => "Check the path format or try an absolute path.",
                ParserError::ReadDirFailed { .. } => "Check directory permissions.",
                ParserError::NoFilesFound { .. } => {
                    "Verify the glob/path entries exist; ensure patterns match .log files in the current directory."
                }
            },
            Error::Export(e) => match e {
                ExportError::WriteFailed { .. } => {
                    "Check disk space and output directory permissions."
                }
                ExportError::Fatal { .. } => "Check the output file and export configuration.",
            },
            Error::Io(_) => "Check filesystem permissions and disk space.",
            Error::Interrupted => "Run was interrupted by user.",
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    NotFound(PathBuf),

    #[error("Failed to parse configuration file {path}: {reason}")]
    ParseFailed { path: PathBuf, reason: String },

    #[error("Invalid log level '{level}', valid values: {}", valid_levels.join(", "))]
    InvalidLogLevel {
        level: String,
        valid_levels: Vec<String>,
    },

    #[error("Invalid configuration value {field} = '{value}': {reason}")]
    InvalidValue {
        field: String,
        value: String,
        reason: String,
    },

    #[error("At least one exporter must be configured (parquet/csv)")]
    NoExporters,
}

#[derive(Debug, Error)]
pub enum FileError {
    #[error("File already exists: {path} (set overwrite=true to replace)")]
    AlreadyExists { path: PathBuf },

    #[error("Failed to write file {path}: {reason}")]
    WriteFailed { path: PathBuf, reason: String },

    #[error("Failed to create directory {path}: {reason}")]
    CreateDirectoryFailed { path: PathBuf, reason: String },
}

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("Path not found: {}", path.display())]
    PathNotFound { path: PathBuf },

    #[error("Invalid path {}: {reason}{}", path.display(), line_number.map_or_else(String::new, |n| format!(" (line {n})")))]
    InvalidPath {
        path: PathBuf,
        reason: String,
        line_number: Option<u64>,
    },

    #[error("Failed to read directory {}: {reason}", path.display())]
    ReadDirFailed { path: PathBuf, reason: String },

    #[error("No log files found matching inputs: {inputs:?}")]
    NoFilesFound { inputs: Vec<String> },
}

#[derive(Debug, Error)]
pub enum ExportError {
    /// 文件写入失败（CSV、错误日志等所有文件型导出器通用）
    #[error("Write failed {path}: {reason}")]
    WriteFailed { path: PathBuf, reason: String },

    /// 导出过程中不可恢复的错误
    #[error("Export failed: {reason}")]
    Fatal { reason: String },
}

#[cfg(test)]
#[path = "../../tests/unit/infrastructure/error.rs"]
mod tests;

use std::collections::HashMap;

/// 解析错误的分类枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    EncodingError,
    FieldMissing,
    ParseFailed,
}

impl ErrorKind {
    #[must_use]
    pub fn kind_display(self) -> &'static str {
        match self {
            Self::EncodingError => "encoding_error",
            Self::FieldMissing => "field_missing",
            Self::ParseFailed => "parse_failed",
        }
    }
}

/// 单条解析错误的详细记录。
#[derive(Debug, Clone)]
pub struct ParseErrorRecord {
    pub line_number: u64,
    pub raw_truncated: String,
    pub kind: ErrorKind,
}

/// Accumulated error statistics for a processing run.
#[derive(Debug, Default, Clone)]
pub struct ErrorStats {
    pub total_errors: usize,
    pub parse_errors: usize,
    pub export_errors: usize,
    pub fatal_error: Option<String>,
    pub by_type: HashMap<ErrorKind, u64>,
    pub filtered_out: u64,
    pub parse_error_records: Vec<ParseErrorRecord>,
    pub records_exported: usize, // 累计成功导出的记录数
}

impl ErrorStats {
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.total_errors > 0
    }

    #[must_use]
    pub fn has_fatal(&self) -> bool {
        self.fatal_error.is_some()
    }

    pub fn add_parse_error(&mut self) {
        self.total_errors += 1;
        self.parse_errors += 1;
    }

    pub fn add_export_error(&mut self) {
        self.total_errors += 1;
        self.export_errors += 1;
    }

    pub fn set_fatal(&mut self, msg: String) {
        self.fatal_error = Some(msg);
    }

    pub fn merge(&mut self, other: &ErrorStats) {
        const MAX_RECORDS: usize = 10_000;
        self.total_errors += other.total_errors;
        self.parse_errors += other.parse_errors;
        self.export_errors += other.export_errors;
        if self.fatal_error.is_none() && other.fatal_error.is_some() {
            self.fatal_error.clone_from(&other.fatal_error);
        }
        for (kind, count) in &other.by_type {
            *self.by_type.entry(*kind).or_insert(0) += count;
        }
        self.filtered_out += other.filtered_out;
        self.records_exported += other.records_exported;
        let remaining_cap = MAX_RECORDS.saturating_sub(self.parse_error_records.len());
        if remaining_cap > 0 {
            self.parse_error_records.extend(
                other
                    .parse_error_records
                    .iter()
                    .take(remaining_cap)
                    .cloned(),
            );
        }
    }
}
