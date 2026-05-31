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

/// Accumulated error statistics for a processing run.
#[derive(Debug, Default, Clone)]
pub struct ErrorStats {
    pub total_errors: usize,
    pub parse_errors: usize,
    pub export_errors: usize,
    pub fatal_error: Option<String>,
}

impl ErrorStats {
    pub fn has_errors(&self) -> bool {
        self.total_errors > 0
    }

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
        self.total_errors += other.total_errors;
        self.parse_errors += other.parse_errors;
        self.export_errors += other.export_errors;
        if self.fatal_error.is_none() && other.fatal_error.is_some() {
            self.fatal_error.clone_from(&other.fatal_error);
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
    pub fn is_fatal(&self) -> bool {
        match self {
            Error::Config(_) | Error::Io(_) | Error::Interrupted => true,
            Error::File(e) => matches!(
                e,
                FileError::AlreadyExists { .. } | FileError::CreateDirectoryFailed { .. }
            ),
            Error::Parser(_) => false,
            Error::Export(e) => matches!(e, ExportError::DatabaseFailed { .. }),
        }
    }

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
                ExportError::DatabaseFailed { .. } => ErrorSeverity::Critical,
            },
        }
    }

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
                ConfigError::NoExporters => "Enable at least one exporter: [csv] or [sqlite].",
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
                ExportError::DatabaseFailed { .. } => {
                    "Verify the SQLite database file is accessible."
                }
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

    #[error("At least one exporter must be configured (csv/sqlite)")]
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

#[derive(Debug)]
pub enum ParserError {
    PathNotFound {
        path: PathBuf,
    },
    InvalidPath {
        path: PathBuf,
        reason: String,
        line_number: Option<u64>,
    },
    ReadDirFailed {
        path: PathBuf,
        reason: String,
    },
    NoFilesFound {
        inputs: Vec<String>,
    },
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParserError::PathNotFound { path } => {
                write!(f, "Path not found: {}", path.display())
            }
            ParserError::InvalidPath {
                path,
                reason,
                line_number,
            } => {
                write!(f, "Invalid path {}: {}", path.display(), reason)?;
                if let Some(line) = line_number {
                    write!(f, " (line {line})")?;
                }
                Ok(())
            }
            ParserError::ReadDirFailed { path, reason } => {
                write!(f, "Failed to read directory {}: {}", path.display(), reason)
            }
            ParserError::NoFilesFound { inputs } => {
                write!(f, "No log files found matching inputs: {inputs:?}")
            }
        }
    }
}

impl std::error::Error for ParserError {}

#[derive(Debug, Error)]
pub enum ExportError {
    /// 文件写入失败（CSV、错误日志等所有文件型导出器通用）
    #[error("Write failed {path}: {reason}")]
    WriteFailed { path: PathBuf, reason: String },

    /// `SQLite` 操作失败
    #[error("Database error: {reason}")]
    DatabaseFailed { reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_files_found_display_contains_inputs() {
        let err = ParserError::NoFilesFound {
            inputs: vec!["a.log".into(), "b/*.log".into()],
        };
        let display = format!("{err}");
        assert!(
            display.contains("a.log"),
            "Display should contain 'a.log', got: {display}"
        );
        assert!(
            display.contains("b/*.log"),
            "Display should contain 'b/*.log', got: {display}"
        );
    }

    #[test]
    fn test_no_files_found_suggestion_non_empty() {
        let err = Error::Parser(ParserError::NoFilesFound {
            inputs: vec!["x".into()],
        });
        let suggestion = err.suggestion();
        assert!(
            !suggestion.is_empty(),
            "suggestion() should not be empty for NoFilesFound"
        );
        assert!(
            suggestion.contains("glob"),
            "suggestion() should contain 'glob', got: {suggestion}"
        );
    }

    #[test]
    fn test_no_files_found_not_fatal() {
        let err = Error::Parser(ParserError::NoFilesFound {
            inputs: vec!["x".into()],
        });
        assert!(!err.is_fatal(), "NoFilesFound should not be fatal");
        assert_eq!(
            err.severity(),
            ErrorSeverity::Warning,
            "NoFilesFound should have Warning severity"
        );
    }
}
