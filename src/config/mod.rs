mod apply_one;
pub mod exporter;
pub mod logging;
pub mod resume;
pub mod sqllog;
mod validate;

pub use exporter::{CsvExporterConfig, ExporterConfig, SqliteExporterConfig};
pub use logging::{LOG_LEVELS, LoggingConfig};
pub use resume::ResumeConfig;
pub use sqllog::SqllogConfig;

use crate::error::{ConfigError, Error, Result};
use crate::pipeline::{FiltersFeature, NormalizeConfig, OutputConfig};
use serde::Deserialize;
use std::path::Path;

const PIPELINE_MIGRATION_HINT: &str = "配置格式已升级，请迁移以下字段：\n  [pipeline.charts] → [charts]\n  [pipeline.normalize] → [replace_parameters]\n  \
     [pipeline.filters.*] → [filter.*]\n  [pipeline.fields] → [output.fields]\n\
     详见 .planning/phases/18-template-chart-nesting/18-CONTEXT.md";

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub sqllog: SqllogConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub exporter: ExporterConfig,
    #[serde(default)]
    pub resume: ResumeConfig,
    #[serde(default)]
    pub replace_parameters: Option<NormalizeConfig>,
    #[serde(default)]
    pub filter: Option<FiltersFeature>,
    #[serde(default)]
    pub output: Option<OutputConfig>,
    /// 旧路径检测：捕获 `[pipeline]` 表（若用户仍用旧格式）。
    /// 非 None 时 validate() 会返回迁移错误，用户不应直接使用此字段。
    #[doc(hidden)]
    #[serde(rename = "pipeline", default)]
    pub pipeline_deprecated: Option<toml::Value>,
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|_| Error::Config(ConfigError::NotFound(path.to_path_buf())))?;
        toml::from_str(&content).map_err(|e| {
            Error::Config(ConfigError::ParseFailed {
                path: path.to_path_buf(),
                reason: e.to_string(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> Config {
        Config::default()
    }

    // ── apply_overrides ────────────────────────────────────────
    #[test]
    fn test_apply_overrides_sqllog_path() {
        let mut cfg = default_config();
        cfg.apply_overrides(&["sqllog.path=/tmp/logs".into()])
            .unwrap();
        assert_eq!(cfg.sqllog.path, "/tmp/logs");
    }

    #[test]
    fn test_apply_overrides_sqllog_directory_alias() {
        let mut cfg = default_config();
        cfg.apply_overrides(&["sqllog.directory=/tmp/logs".into()])
            .unwrap();
        assert_eq!(cfg.sqllog.path, "/tmp/logs");
    }

    #[test]
    fn test_apply_overrides_logging_level() {
        let mut cfg = default_config();
        cfg.apply_overrides(&["logging.level=debug".into()])
            .unwrap();
        assert_eq!(cfg.logging.level, "debug");
    }

    #[test]
    fn test_apply_overrides_csv_file() {
        let mut cfg = default_config();
        cfg.apply_overrides(&["exporter.csv.file=/tmp/out.csv".into()])
            .unwrap();
        assert_eq!(cfg.exporter.csv.unwrap().file, "/tmp/out.csv");
    }

    #[test]
    fn test_apply_overrides_csv_overwrite_false() {
        let mut cfg = default_config();
        cfg.apply_overrides(&["exporter.csv.overwrite=false".into()])
            .unwrap();
        assert!(!cfg.exporter.csv.unwrap().overwrite);
    }

    #[test]
    fn test_apply_overrides_sqlite_database_url() {
        let mut cfg = default_config();
        cfg.apply_overrides(&["exporter.sqlite.database_url=/tmp/out.db".into()])
            .unwrap();
        assert_eq!(cfg.exporter.sqlite.unwrap().database_url, "/tmp/out.db");
    }

    #[test]
    fn test_apply_overrides_unknown_key_returns_error() {
        let mut cfg = default_config();
        assert!(cfg.apply_overrides(&["unknown.key=value".into()]).is_err());
    }

    #[test]
    fn test_apply_overrides_bad_format_returns_error() {
        let mut cfg = default_config();
        assert!(cfg.apply_overrides(&["nodelimiter".into()]).is_err());
    }

    #[test]
    fn test_apply_overrides_invalid_bool() {
        let mut cfg = default_config();
        assert!(
            cfg.apply_overrides(&["exporter.csv.overwrite=maybe".into()])
                .is_err()
        );
    }

    #[test]
    fn test_apply_overrides_retention_days_invalid() {
        let mut cfg = default_config();
        assert!(
            cfg.apply_overrides(&["logging.retention_days=abc".into()])
                .is_err()
        );
    }

    #[test]
    fn test_apply_overrides_csv_append() {
        let mut cfg = default_config();
        cfg.apply_overrides(&["exporter.csv.append=true".into()])
            .unwrap();
        assert!(cfg.exporter.csv.unwrap().append);
    }

    #[test]
    fn test_apply_overrides_sqlite_table_name() {
        let mut cfg = default_config();
        cfg.apply_overrides(&["exporter.sqlite.table_name=my_table".into()])
            .unwrap();
        assert_eq!(cfg.exporter.sqlite.unwrap().table_name, "my_table");
    }

    #[test]
    fn test_apply_overrides_sqlite_overwrite() {
        let mut cfg = default_config();
        cfg.apply_overrides(&["exporter.sqlite.overwrite=false".into()])
            .unwrap();
        assert!(!cfg.exporter.sqlite.unwrap().overwrite);
    }

    #[test]
    fn test_apply_overrides_sqlite_append() {
        let mut cfg = default_config();
        cfg.apply_overrides(&["exporter.sqlite.append=true".into()])
            .unwrap();
        assert!(cfg.exporter.sqlite.unwrap().append);
    }

    // ── ExporterConfig ─────────────────────────────────────────
    #[test]
    fn test_exporter_config_has_any_csv() {
        let cfg = ExporterConfig::default();
        assert!(cfg.csv.is_some());
    }

    #[test]
    fn test_exporter_config_default_no_sqlite() {
        let cfg = ExporterConfig::default();
        assert!(cfg.sqlite.is_none());
    }

    // ── from_file ──────────────────────────────────────────────
    #[test]
    fn test_from_file_not_found() {
        let result = Config::from_file("/nonexistent/path/config.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_file_valid_toml() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
[sqllog]
directory = "sqllogs"
[exporter.csv]
file = "out.csv"
"#,
        )
        .unwrap();
        let cfg = Config::from_file(&path).unwrap();
        assert_eq!(cfg.sqllog.path, "sqllogs");
        assert_eq!(cfg.exporter.csv.unwrap().file, "out.csv");
    }

    #[test]
    fn test_from_file_invalid_toml_returns_error() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, "not valid toml ][[").unwrap();
        let result = Config::from_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_logging_config_values() {
        let cfg = LoggingConfig::default();
        assert_eq!(cfg.file, "logs/sqllog2db.log");
        assert_eq!(cfg.level, "info");
        assert_eq!(cfg.retention_days, 7);
    }

    #[test]
    fn test_default_sqlite_exporter_values() {
        let cfg = SqliteExporterConfig::default();
        assert_eq!(cfg.table_name, "sqllog_records");
        assert_eq!(cfg.database_url, "export/sqllog2db.db");
        assert!(cfg.overwrite);
        assert!(!cfg.append);
    }

    #[test]
    fn test_csv_exporter_default_include_performance_metrics_true() {
        let cfg = CsvExporterConfig::default();
        assert!(cfg.include_performance_metrics);
    }

    #[test]
    fn test_csv_toml_default_include_performance_metrics() {
        let toml = r#"
[sqllog]
directory = "sqllogs"
[exporter.csv]
file = "/tmp/x.csv"
overwrite = true
append = false
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert!(
            cfg.exporter
                .csv
                .as_ref()
                .unwrap()
                .include_performance_metrics,
        );
    }

    #[test]
    fn test_config_has_3_top_level_optional_fields() {
        // 确保 3 个顶层字段默认值为 None
        let cfg = default_config();
        assert!(cfg.replace_parameters.is_none());
        assert!(cfg.filter.is_none());
        assert!(cfg.output.is_none());
    }
}
