use std::collections::HashMap;
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
    pub records_exported: usize, // 累计成功导出/处理的记录数（D-09，watch 模式累计统计用）
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
            Error::Export(e) => matches!(e, ExportError::DatabaseFailed { .. }),
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
                ExportError::DatabaseFailed { .. } => ErrorSeverity::Critical,
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

    // ===== ConfigError 全 5 个变体 =====

    #[test]
    fn test_config_not_found_is_fatal_critical_suggestion() {
        let err = Error::Config(ConfigError::NotFound(PathBuf::from("/no/file")));
        assert!(err.is_fatal(), "ConfigError::NotFound should be fatal");
        assert_eq!(
            err.severity(),
            ErrorSeverity::Critical,
            "ConfigError::NotFound should have Critical severity"
        );
        assert!(
            err.suggestion().contains("init"),
            "ConfigError::NotFound suggestion should contain 'init', got: {}",
            err.suggestion()
        );
    }

    #[test]
    fn test_config_parse_failed_suggestion_mentions_toml() {
        let err = Error::Config(ConfigError::ParseFailed {
            path: PathBuf::from("/cfg.toml"),
            reason: "unexpected key".into(),
        });
        assert!(err.is_fatal(), "ConfigError::ParseFailed should be fatal");
        assert_eq!(
            err.severity(),
            ErrorSeverity::Critical,
            "ConfigError::ParseFailed should have Critical severity"
        );
        assert!(
            err.suggestion().contains("TOML"),
            "ConfigError::ParseFailed suggestion should contain 'TOML', got: {}",
            err.suggestion()
        );
    }

    #[test]
    fn test_config_invalid_log_level_suggestion() {
        let err = Error::Config(ConfigError::InvalidLogLevel {
            level: "verbose".into(),
            valid_levels: vec!["info".into(), "debug".into()],
        });
        assert!(
            err.is_fatal(),
            "ConfigError::InvalidLogLevel should be fatal"
        );
        assert!(
            err.suggestion().contains("log levels"),
            "ConfigError::InvalidLogLevel suggestion should contain 'log levels', got: {}",
            err.suggestion()
        );
    }

    #[test]
    fn test_config_invalid_value_suggestion() {
        let err = Error::Config(ConfigError::InvalidValue {
            field: "buffer_size".into(),
            value: "-1".into(),
            reason: "must be positive".into(),
        });
        assert!(err.is_fatal(), "ConfigError::InvalidValue should be fatal");
        assert!(
            err.suggestion().contains("field value"),
            "ConfigError::InvalidValue suggestion should contain 'field value', got: {}",
            err.suggestion()
        );
    }

    #[test]
    fn test_config_no_exporters_suggestion() {
        let err = Error::Config(ConfigError::NoExporters);
        assert!(err.is_fatal(), "ConfigError::NoExporters should be fatal");
        assert!(
            err.suggestion().contains("exporter"),
            "ConfigError::NoExporters suggestion should contain 'exporter', got: {}",
            err.suggestion()
        );
    }

    // ===== FileError 全 3 个变体 =====

    #[test]
    fn test_file_already_exists_is_fatal() {
        let err = Error::File(FileError::AlreadyExists {
            path: PathBuf::from("/out.csv"),
        });
        assert!(err.is_fatal(), "FileError::AlreadyExists should be fatal");
        assert_eq!(
            err.severity(),
            ErrorSeverity::Critical,
            "FileError::AlreadyExists should have Critical severity"
        );
        assert!(
            err.suggestion().contains("--force"),
            "FileError::AlreadyExists suggestion should contain '--force', got: {}",
            err.suggestion()
        );
    }

    #[test]
    fn test_file_write_failed_not_fatal_error_severity() {
        let err = Error::File(FileError::WriteFailed {
            path: PathBuf::from("/out.csv"),
            reason: "permission denied".into(),
        });
        assert!(
            !err.is_fatal(),
            "FileError::WriteFailed should not be fatal"
        );
        assert_eq!(
            err.severity(),
            ErrorSeverity::Error,
            "FileError::WriteFailed should have Error severity"
        );
        assert!(
            err.suggestion().contains("disk space"),
            "FileError::WriteFailed suggestion should contain 'disk space', got: {}",
            err.suggestion()
        );
    }

    #[test]
    fn test_file_create_directory_failed_is_fatal() {
        let err = Error::File(FileError::CreateDirectoryFailed {
            path: PathBuf::from("/no/dir"),
            reason: "permission denied".into(),
        });
        assert!(
            err.is_fatal(),
            "FileError::CreateDirectoryFailed should be fatal"
        );
        assert_eq!(
            err.severity(),
            ErrorSeverity::Critical,
            "FileError::CreateDirectoryFailed should have Critical severity"
        );
        assert!(
            err.suggestion().contains("parent directory"),
            "FileError::CreateDirectoryFailed suggestion should contain 'parent directory', got: {}",
            err.suggestion()
        );
    }

    // ===== ExportError 全 2 个变体 =====

    #[test]
    fn test_export_write_failed_not_fatal_error_severity() {
        let err = Error::Export(ExportError::WriteFailed {
            path: PathBuf::from("/out.db"),
            reason: "disk full".into(),
        });
        assert!(
            !err.is_fatal(),
            "ExportError::WriteFailed should not be fatal"
        );
        assert_eq!(
            err.severity(),
            ErrorSeverity::Error,
            "ExportError::WriteFailed should have Error severity"
        );
        assert!(
            err.suggestion().contains("disk space"),
            "ExportError::WriteFailed suggestion should contain 'disk space', got: {}",
            err.suggestion()
        );
    }

    #[test]
    fn test_export_database_failed_is_fatal_critical() {
        let err = Error::Export(ExportError::DatabaseFailed {
            reason: "locked".into(),
        });
        assert!(
            err.is_fatal(),
            "ExportError::DatabaseFailed should be fatal"
        );
        assert_eq!(
            err.severity(),
            ErrorSeverity::Critical,
            "ExportError::DatabaseFailed should have Critical severity"
        );
        assert!(
            err.suggestion().contains("SQLite"),
            "ExportError::DatabaseFailed suggestion should contain 'SQLite', got: {}",
            err.suggestion()
        );
    }

    // ===== Error::Io 和 Error::Interrupted =====

    #[test]
    fn test_io_error_is_fatal_critical() {
        let err = Error::Io(std::io::Error::other("test io error"));
        assert!(err.is_fatal(), "Error::Io should be fatal");
        assert_eq!(
            err.severity(),
            ErrorSeverity::Critical,
            "Error::Io should have Critical severity"
        );
        assert!(
            err.suggestion().contains("filesystem"),
            "Error::Io suggestion should contain 'filesystem', got: {}",
            err.suggestion()
        );
    }

    #[test]
    fn test_interrupted_is_fatal_critical() {
        let err = Error::Interrupted;
        assert!(err.is_fatal(), "Error::Interrupted should be fatal");
        assert_eq!(
            err.severity(),
            ErrorSeverity::Critical,
            "Error::Interrupted should have Critical severity"
        );
        assert!(
            err.suggestion().contains("interrupted"),
            "Error::Interrupted suggestion should contain 'interrupted', got: {}",
            err.suggestion()
        );
    }

    // ===== ParserError 3 个变体（不含已覆盖的 NoFilesFound）=====

    #[test]
    fn test_parser_path_not_found_suggestion() {
        let err = Error::Parser(ParserError::PathNotFound {
            path: PathBuf::from("/missing.log"),
        });
        assert!(
            !err.is_fatal(),
            "ParserError::PathNotFound should not be fatal"
        );
        assert_eq!(
            err.severity(),
            ErrorSeverity::Warning,
            "ParserError::PathNotFound should have Warning severity"
        );
        assert!(
            err.suggestion().contains("log file exists"),
            "ParserError::PathNotFound suggestion should contain 'log file exists', got: {}",
            err.suggestion()
        );
    }

    #[test]
    fn test_parser_invalid_path_suggestion() {
        let err = Error::Parser(ParserError::InvalidPath {
            path: PathBuf::from("/bad\0path"),
            reason: "null byte in path".into(),
            line_number: Some(42),
        });
        assert!(
            !err.is_fatal(),
            "ParserError::InvalidPath should not be fatal"
        );
        assert!(
            err.suggestion().contains("path format"),
            "ParserError::InvalidPath suggestion should contain 'path format', got: {}",
            err.suggestion()
        );
    }

    #[test]
    fn test_parser_read_dir_failed_is_fatal() {
        let err = Error::Parser(ParserError::ReadDirFailed {
            path: PathBuf::from("/locked/dir"),
            reason: "permission denied".into(),
        });
        assert!(
            err.is_fatal(),
            "ParserError::ReadDirFailed should be fatal (only fatal Parser variant)"
        );
        assert!(
            err.suggestion().contains("directory permissions"),
            "ParserError::ReadDirFailed suggestion should contain 'directory permissions', got: {}",
            err.suggestion()
        );
    }

    // ===== ErrorStats =====

    #[test]
    fn test_error_stats_add_and_count() {
        let mut s = ErrorStats::default();
        s.add_parse_error();
        s.add_export_error();
        assert_eq!(s.total_errors, 2);
        assert_eq!(s.parse_errors, 1);
        assert_eq!(s.export_errors, 1);
        assert!(
            s.has_errors(),
            "has_errors should be true after adding errors"
        );
        assert!(
            !s.has_fatal(),
            "has_fatal should be false when no fatal set"
        );
    }

    #[test]
    fn test_error_stats_set_fatal_and_has_fatal() {
        let mut s = ErrorStats::default();
        assert!(!s.has_fatal());
        s.set_fatal("catastrophic failure".into());
        assert!(s.has_fatal(), "has_fatal should be true after set_fatal");
        assert_eq!(s.fatal_error.as_deref(), Some("catastrophic failure"));
    }

    #[test]
    fn test_error_stats_merge_first_fatal_wins() {
        let mut a = ErrorStats::default();
        a.set_fatal("first".into());
        let mut b = ErrorStats::default();
        b.set_fatal("second".into());
        a.merge(&b);
        assert_eq!(
            a.fatal_error.as_deref(),
            Some("first"),
            "merge must not overwrite an existing fatal_error"
        );
    }

    #[test]
    fn test_error_stats_merge_propagates_fatal_when_base_has_none() {
        let mut a = ErrorStats::default();
        let mut b = ErrorStats::default();
        b.set_fatal("only".into());
        a.merge(&b);
        assert_eq!(
            a.fatal_error.as_deref(),
            Some("only"),
            "merge should propagate fatal_error when base has none"
        );
    }

    #[test]
    fn test_error_stats_merge_accumulates_counts() {
        let mut a = ErrorStats::default();
        a.add_parse_error();
        a.add_parse_error();
        let mut b = ErrorStats::default();
        b.add_export_error();
        a.merge(&b);
        assert_eq!(a.total_errors, 3, "merge should sum total_errors");
        assert_eq!(
            a.parse_errors, 2,
            "parse_errors should carry over from base"
        );
        assert_eq!(a.export_errors, 1, "export_errors should add from other");
    }

    #[test]
    fn test_error_stats_default_has_no_errors() {
        let s = ErrorStats::default();
        assert!(!s.has_errors(), "default ErrorStats should have no errors");
        assert!(!s.has_fatal(), "default ErrorStats should have no fatal");
    }

    // ===== ErrorSeverity Display =====

    #[test]
    fn test_error_severity_display_strings() {
        assert_eq!(
            format!("{}", ErrorSeverity::Warning),
            "WARNING",
            "ErrorSeverity::Warning should display as 'WARNING'"
        );
        assert_eq!(
            format!("{}", ErrorSeverity::Error),
            "ERROR",
            "ErrorSeverity::Error should display as 'ERROR'"
        );
        assert_eq!(
            format!("{}", ErrorSeverity::Critical),
            "CRITICAL",
            "ErrorSeverity::Critical should display as 'CRITICAL'"
        );
    }

    // ===== ErrorStats by_type merge =====

    #[test]
    fn test_error_stats_by_type_merge() {
        let mut a = ErrorStats::default();
        *a.by_type.entry(ErrorKind::EncodingError).or_insert(0) += 2;

        let mut b = ErrorStats::default();
        *b.by_type.entry(ErrorKind::EncodingError).or_insert(0) += 3;
        *b.by_type.entry(ErrorKind::FieldMissing).or_insert(0) += 1;

        a.merge(&b);

        assert_eq!(
            a.by_type
                .get(&ErrorKind::EncodingError)
                .copied()
                .unwrap_or(0),
            5,
            "merge 后 EncodingError 计数应为 5"
        );
        assert_eq!(
            a.by_type
                .get(&ErrorKind::FieldMissing)
                .copied()
                .unwrap_or(0),
            1,
            "merge 后 FieldMissing 计数应为 1"
        );
    }

    #[test]
    fn test_error_stats_merge_propagates_filtered_and_records() {
        let mut a = ErrorStats {
            filtered_out: 2,
            parse_error_records: vec![ParseErrorRecord {
                line_number: 1,
                raw_truncated: "raw_a".into(),
                kind: ErrorKind::ParseFailed,
            }],
            ..Default::default()
        };

        let b = ErrorStats {
            filtered_out: 5,
            parse_error_records: vec![
                ParseErrorRecord {
                    line_number: 2,
                    raw_truncated: "raw_b1".into(),
                    kind: ErrorKind::EncodingError,
                },
                ParseErrorRecord {
                    line_number: 3,
                    raw_truncated: "raw_b2".into(),
                    kind: ErrorKind::FieldMissing,
                },
            ],
            ..Default::default()
        };

        a.merge(&b);

        assert_eq!(a.filtered_out, 7, "merge 后 filtered_out 应为 7");
        assert_eq!(
            a.parse_error_records.len(),
            3,
            "merge 后 parse_error_records 应有 3 条"
        );
    }
}
