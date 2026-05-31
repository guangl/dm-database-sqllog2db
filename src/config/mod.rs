pub mod exporter;
pub mod logging;
pub mod sqllog;
mod validate;

pub use exporter::{CsvExporterConfig, ExporterConfig, SqliteExporterConfig};
pub use logging::{LOG_LEVELS, LoggingConfig};
pub use sqllog::SqllogConfig;

use crate::error::{ConfigError, Error, Result};
use crate::pipeline::{FiltersFeature, NormalizeConfig, OutputConfig};
use serde::Deserialize;
use std::path::Path;

const PIPELINE_MIGRATION_HINT: &str = "配置格式已升级，请迁移以下字段：\n  [pipeline.normalize] → [replace_parameters]\n  \
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
    /// 旧路径检测：捕获 `[template]` 表（若用户仍用旧格式）。
    /// 非 None 时 validate() 会返回废弃错误，用户不应直接使用此字段。
    #[doc(hidden)]
    #[serde(rename = "template", default)]
    pub template_deprecated: Option<toml::Value>,
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
inputs = ["sqllogs"]
[exporter.csv]
file = "out.csv"
"#,
        )
        .unwrap();
        let cfg = Config::from_file(&path).unwrap();
        assert_eq!(cfg.sqllog.inputs, vec!["sqllogs".to_string()]);
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
inputs = ["sqllogs"]
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
        let cfg = Config::default();
        assert!(cfg.replace_parameters.is_none());
        assert!(cfg.filter.is_none());
        assert!(cfg.output.is_none());
    }
}
