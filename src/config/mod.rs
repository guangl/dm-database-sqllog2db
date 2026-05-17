pub mod exporter;
pub mod logging;
pub mod resume;
pub mod sqllog;

pub use exporter::{CsvExporterConfig, ExporterConfig, SqliteExporterConfig};
pub use logging::{LOG_LEVELS, LoggingConfig};
pub use resume::ResumeConfig;
pub use sqllog::SqllogConfig;

pub use crate::pipeline::PipelineConfig;

use crate::error::{ConfigError, Error, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub sqllog: SqllogConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub pipeline: PipelineConfig,
    #[serde(default)]
    pub exporter: ExporterConfig,
    #[serde(default)]
    pub resume: ResumeConfig,
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

    pub fn validate(&self) -> Result<()> {
        self.logging.validate()?;
        self.exporter.validate()?;
        self.sqllog.validate()?;
        self.validate_pipeline_filters()?;
        self.validate_pipeline_fields()?;
        self.validate_pipeline_charts()?;
        Ok(())
    }

    /// 等价于 `validate()` 但额外返回已编译的过滤器对，供调用方复用，
    /// 消除 `run` 子命令路径中 regex 的双重编译。
    ///
    /// 返回值语义：
    /// - `Ok(None)`：无过滤器配置 或 `pipeline.filters.enable == false`
    /// - `Ok(Some((meta, sql)))`：过滤器已编译，调用方可直接传递给 `build_pipeline`
    /// - `Err(_)`：任意子校验失败
    pub fn validate_and_compile(
        &self,
    ) -> Result<
        Option<(
            crate::pipeline::CompiledMetaFilters,
            crate::pipeline::CompiledSqlFilters,
        )>,
    > {
        self.logging.validate()?;
        self.exporter.validate()?;
        self.sqllog.validate()?;

        let compiled = if let Some(filters) = &self.pipeline.filters {
            if filters.enable {
                let meta = crate::pipeline::CompiledMetaFilters::try_from_include_exclude(
                    &filters.include,
                    &filters.exclude,
                )?;
                let sql =
                    crate::pipeline::CompiledSqlFilters::try_from_sql_filters(&filters.record_sql)?;
                Some((meta, sql))
            } else {
                None
            }
        } else {
            None
        };

        self.validate_pipeline_fields()?;
        self.validate_pipeline_charts()?;
        Ok(compiled)
    }

    /// 将 `--set key=value` 覆盖应用到 config。
    /// 支持点路径，例如 `sqllog.path`、`exporter.csv.file`。
    pub fn apply_overrides(&mut self, overrides: &[String]) -> Result<()> {
        for item in overrides {
            let (key, value) = item.split_once('=').ok_or_else(|| {
                Error::Config(ConfigError::InvalidValue {
                    field: item.clone(),
                    value: String::new(),
                    reason: "expected KEY=VALUE format".to_string(),
                })
            })?;
            self.apply_one(key, value)?;
        }
        Ok(())
    }

    fn apply_one(&mut self, key: &str, value: &str) -> Result<()> {
        let unknown = || {
            Error::Config(ConfigError::InvalidValue {
                field: key.to_string(),
                value: value.to_string(),
                reason: format!("unknown config key '{key}'"),
            })
        };
        let parse_bool = |v: &str| -> Result<bool> {
            match v {
                "true" | "1" | "yes" => Ok(true),
                "false" | "0" | "no" => Ok(false),
                _ => Err(Error::Config(ConfigError::InvalidValue {
                    field: key.to_string(),
                    value: v.to_string(),
                    reason: "expected true/false".to_string(),
                })),
            }
        };

        match key {
            "sqllog.path" | "sqllog.directory" => self.sqllog.path = value.to_string(),
            "logging.level" => self.logging.level = value.to_string(),
            "logging.file" => self.logging.file = value.to_string(),
            "logging.retention_days" => {
                self.logging.retention_days = value.parse().map_err(|_| {
                    Error::Config(ConfigError::InvalidValue {
                        field: key.to_string(),
                        value: value.to_string(),
                        reason: "expected a positive integer".to_string(),
                    })
                })?;
            }

            "exporter.csv.file" => {
                self.exporter.csv.get_or_insert_with(Default::default).file = value.to_string();
            }
            "exporter.csv.overwrite" => {
                self.exporter
                    .csv
                    .get_or_insert_with(Default::default)
                    .overwrite = parse_bool(value)?;
            }
            "exporter.csv.append" => {
                self.exporter
                    .csv
                    .get_or_insert_with(Default::default)
                    .append = parse_bool(value)?;
            }
            "exporter.csv.include_performance_metrics" => {
                self.exporter
                    .csv
                    .get_or_insert_with(Default::default)
                    .include_performance_metrics = parse_bool(value)?;
            }

            "exporter.sqlite.database_url" => {
                self.exporter
                    .sqlite
                    .get_or_insert_with(Default::default)
                    .database_url = value.to_string();
            }
            "exporter.sqlite.table_name" => {
                self.exporter
                    .sqlite
                    .get_or_insert_with(Default::default)
                    .table_name = value.to_string();
            }
            "exporter.sqlite.overwrite" => {
                self.exporter
                    .sqlite
                    .get_or_insert_with(Default::default)
                    .overwrite = parse_bool(value)?;
            }
            "exporter.sqlite.append" => {
                self.exporter
                    .sqlite
                    .get_or_insert_with(Default::default)
                    .append = parse_bool(value)?;
            }
            "exporter.sqlite.batch_size" => {
                let parsed = value.parse::<usize>().map_err(|_| {
                    Error::Config(ConfigError::InvalidValue {
                        field: "exporter.sqlite.batch_size".to_string(),
                        value: value.to_string(),
                        reason: "expected a positive integer".to_string(),
                    })
                })?;
                self.exporter
                    .sqlite
                    .get_or_insert_with(Default::default)
                    .batch_size = parsed;
            }

            "pipeline.filters.enable" => {
                self.pipeline
                    .filters
                    .get_or_insert_with(Default::default)
                    .enable = parse_bool(value)?;
            }
            "pipeline.normalize.enable" => {
                self.pipeline
                    .normalize
                    .get_or_insert_with(Default::default)
                    .enable = parse_bool(value)?;
            }
            "pipeline.template_analysis.enabled" => {
                self.pipeline
                    .template_analysis
                    .get_or_insert_with(Default::default)
                    .enabled = parse_bool(value)?;
            }

            "pipeline.charts.output_dir" => {
                if value.trim().is_empty() {
                    return Err(Error::Config(ConfigError::InvalidValue {
                        field: key.to_string(),
                        value: value.to_string(),
                        reason: "charts output_dir cannot be empty".to_string(),
                    }));
                }
                self.pipeline
                    .charts
                    .get_or_insert_with(Default::default)
                    .output_dir = value.to_string();
            }
            "pipeline.charts.top_n" => {
                let parsed = value.parse::<usize>().map_err(|_| {
                    Error::Config(ConfigError::InvalidValue {
                        field: key.to_string(),
                        value: value.to_string(),
                        reason: "expected a positive integer".to_string(),
                    })
                })?;
                if parsed == 0 {
                    return Err(Error::Config(ConfigError::InvalidValue {
                        field: key.to_string(),
                        value: "0".to_string(),
                        reason: "top_n must be greater than 0".to_string(),
                    }));
                }
                self.pipeline
                    .charts
                    .get_or_insert_with(Default::default)
                    .top_n = parsed;
            }
            "pipeline.charts.frequency_bar" => {
                self.pipeline
                    .charts
                    .get_or_insert_with(Default::default)
                    .frequency_bar = parse_bool(value)?;
            }
            "pipeline.charts.latency_hist" => {
                self.pipeline
                    .charts
                    .get_or_insert_with(Default::default)
                    .latency_hist = parse_bool(value)?;
            }
            "pipeline.charts.trend_line" => {
                self.pipeline
                    .charts
                    .get_or_insert_with(Default::default)
                    .trend_line = parse_bool(value)?;
            }
            "pipeline.charts.user_pie" => {
                self.pipeline
                    .charts
                    .get_or_insert_with(Default::default)
                    .user_pie = parse_bool(value)?;
            }

            _ => return Err(unknown()),
        }
        Ok(())
    }

    fn validate_pipeline_filters(&self) -> Result<()> {
        if let Some(filters) = &self.pipeline.filters {
            if filters.enable {
                crate::pipeline::filters::CompiledMetaFilters::try_from_include_exclude(
                    &filters.include,
                    &filters.exclude,
                )?;
                crate::pipeline::filters::CompiledSqlFilters::try_from_sql_filters(
                    &filters.record_sql,
                )?;
            }
        }
        Ok(())
    }

    fn validate_pipeline_fields(&self) -> Result<()> {
        if let Some(names) = &self.pipeline.fields {
            for name in names {
                if !crate::pipeline::FIELD_NAMES.contains(&name.as_str()) {
                    return Err(Error::Config(ConfigError::InvalidValue {
                        field: "pipeline.fields".to_string(),
                        value: name.clone(),
                        reason: format!(
                            "unknown field '{name}'; valid fields: {}",
                            crate::pipeline::FIELD_NAMES.join(", ")
                        ),
                    }));
                }
            }
        }
        Ok(())
    }

    fn validate_pipeline_charts(&self) -> Result<()> {
        if let Some(charts) = &self.pipeline.charts {
            let ta_enabled = self
                .pipeline
                .template_analysis
                .as_ref()
                .is_some_and(|ta| ta.enabled);
            if !ta_enabled {
                return Err(Error::Config(ConfigError::InvalidValue {
                    field: "pipeline.charts".to_string(),
                    value: String::new(),
                    reason: "启用 [pipeline.charts] 需要先设置 [pipeline.template_analysis]\nenabled = true".to_string(),
                }));
            }
            if charts.output_dir.trim().is_empty() {
                return Err(Error::Config(ConfigError::InvalidValue {
                    field: "pipeline.charts.output_dir".to_string(),
                    value: charts.output_dir.clone(),
                    reason: "charts output_dir cannot be empty".to_string(),
                }));
            }
            if charts.top_n == 0 {
                return Err(Error::Config(ConfigError::InvalidValue {
                    field: "pipeline.charts.top_n".to_string(),
                    value: "0".to_string(),
                    reason: "top_n must be greater than 0".to_string(),
                }));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{ChartsConfig, PipelineConfig, TemplateAnalysisConfig};

    fn default_config() -> Config {
        Config::default()
    }

    // ── validate ───────────────────────────────────────────────
    #[test]
    fn test_validate_default_config_passes() {
        assert!(default_config().validate().is_ok());
    }

    #[test]
    fn test_validate_empty_logging_file() {
        let mut cfg = default_config();
        cfg.logging.file = "  ".into();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_empty_csv_file() {
        let mut cfg = default_config();
        cfg.exporter.csv = Some(CsvExporterConfig {
            file: "  ".into(),
            ..CsvExporterConfig::default()
        });
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_empty_sqlite_database_url() {
        let mut cfg = default_config();
        cfg.exporter.csv = None;
        cfg.exporter.sqlite = Some(SqliteExporterConfig {
            database_url: "  ".into(),
            ..SqliteExporterConfig::default()
        });
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_empty_sqlite_table_name() {
        let mut cfg = default_config();
        cfg.exporter.csv = None;
        cfg.exporter.sqlite = Some(SqliteExporterConfig {
            table_name: "  ".into(),
            ..SqliteExporterConfig::default()
        });
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_log_level() {
        let mut cfg = default_config();
        cfg.logging.level = "invalid".into();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_retention_days_zero() {
        let mut cfg = default_config();
        cfg.logging.retention_days = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_retention_days_over_365() {
        let mut cfg = default_config();
        cfg.logging.retention_days = 366;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_empty_sqllog_directory() {
        let mut cfg = default_config();
        cfg.sqllog.path = "  ".into();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_no_exporters() {
        let mut cfg = default_config();
        cfg.exporter.csv = None;
        assert!(cfg.validate().is_err());
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

    // ── regex validation ───────────────────────────────────────
    #[test]
    fn test_validate_invalid_regex_in_filters() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[pipeline.filters]
enable = true
usernames = ["[invalid"]
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let result = cfg.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("pipeline.filters.include.users"),
            "error should mention field name, got: {err_msg}"
        );
    }

    #[test]
    fn test_validate_valid_regex_in_filters() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[pipeline.filters]
enable = true
usernames = ["^admin.*"]
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_csv_exporter_default_include_performance_metrics_true() {
        let cfg = CsvExporterConfig::default();
        assert!(cfg.include_performance_metrics);
    }

    #[test]
    fn test_apply_one_csv_include_performance_metrics_false() {
        let mut cfg = Config::default();
        cfg.apply_one("exporter.csv.include_performance_metrics", "false")
            .expect("apply_one should succeed for valid bool");
        assert!(
            !cfg.exporter
                .csv
                .as_ref()
                .unwrap()
                .include_performance_metrics,
        );
    }

    #[test]
    fn test_apply_one_csv_include_performance_metrics_invalid() {
        let mut cfg = Config::default();
        let r = cfg.apply_one("exporter.csv.include_performance_metrics", "maybe");
        assert!(r.is_err());
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

    // ── table_name ASCII 标识符校验 ────────────────────────────
    #[test]
    fn test_validate_sqlite_table_name_valid_simple() {
        let mut cfg = default_config();
        cfg.exporter.sqlite = Some(SqliteExporterConfig {
            database_url: "/tmp/x.db".into(),
            table_name: "tbl".into(),
            ..SqliteExporterConfig::default()
        });
        cfg.exporter.csv = None;
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_validate_sqlite_table_name_valid_underscore_prefix() {
        let mut cfg = default_config();
        cfg.exporter.sqlite = Some(SqliteExporterConfig {
            database_url: "/tmp/x.db".into(),
            table_name: "_records".into(),
            ..SqliteExporterConfig::default()
        });
        cfg.exporter.csv = None;
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_validate_sqlite_table_name_valid_with_digits() {
        let mut cfg = default_config();
        cfg.exporter.sqlite = Some(SqliteExporterConfig {
            database_url: "/tmp/x.db".into(),
            table_name: "t1_log_2024".into(),
            ..SqliteExporterConfig::default()
        });
        cfg.exporter.csv = None;
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_validate_sqlite_table_name_rejects_leading_digit() {
        let mut cfg = default_config();
        cfg.exporter.sqlite = Some(SqliteExporterConfig {
            database_url: "/tmp/x.db".into(),
            table_name: "1tbl".into(),
            ..SqliteExporterConfig::default()
        });
        cfg.exporter.csv = None;
        let err = cfg.validate().unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("ASCII identifiers only"), "actual: {msg}");
        assert!(msg.contains("exporter.sqlite.table_name"), "actual: {msg}");
    }

    #[test]
    fn test_validate_sqlite_table_name_rejects_special_char() {
        let mut cfg = default_config();
        cfg.exporter.sqlite = Some(SqliteExporterConfig {
            database_url: "/tmp/x.db".into(),
            table_name: "tbl;DROP".into(),
            ..SqliteExporterConfig::default()
        });
        cfg.exporter.csv = None;
        let err = cfg.validate().unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("ASCII identifiers only"), "actual: {msg}");
    }

    #[test]
    fn test_validate_sqlite_table_name_rejects_quote() {
        let mut cfg = default_config();
        cfg.exporter.sqlite = Some(SqliteExporterConfig {
            database_url: "/tmp/x.db".into(),
            table_name: "tbl\"x".into(),
            ..SqliteExporterConfig::default()
        });
        cfg.exporter.csv = None;
        let err = cfg.validate().unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("ASCII identifiers only"), "actual: {msg}");
    }

    #[test]
    fn test_validate_sqlite_table_name_rejects_non_ascii() {
        let mut cfg = default_config();
        cfg.exporter.sqlite = Some(SqliteExporterConfig {
            database_url: "/tmp/x.db".into(),
            table_name: "日志表".into(),
            ..SqliteExporterConfig::default()
        });
        cfg.exporter.csv = None;
        let err = cfg.validate().unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("ASCII identifiers only"), "actual: {msg}");
    }

    #[test]
    fn test_validate_sqlite_table_name_rejects_space() {
        let mut cfg = default_config();
        cfg.exporter.sqlite = Some(SqliteExporterConfig {
            database_url: "/tmp/x.db".into(),
            table_name: "my tbl".into(),
            ..SqliteExporterConfig::default()
        });
        cfg.exporter.csv = None;
        let err = cfg.validate().unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("ASCII identifiers only"), "actual: {msg}");
    }

    // ── validate_and_compile ───────────────────────────────────────
    #[test]
    fn test_validate_and_compile_default_returns_none() {
        let cfg = default_config();
        let result = cfg.validate_and_compile().expect("default config valid");
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_and_compile_filters_disabled_returns_none() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[pipeline.filters]
enable = false
usernames = ["^admin.*"]
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let result = cfg.validate_and_compile().expect("config valid");
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_and_compile_returns_compiled_pair() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[pipeline.filters]
enable = true
usernames = ["^admin.*"]
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let result = cfg.validate_and_compile().expect("config valid");
        let pair = result.expect("filters.enable=true should return Some");
        assert!(pair.0.has_any_filters());
    }

    #[test]
    fn test_validate_and_compile_invalid_regex_returns_err() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[pipeline.filters]
enable = true
usernames = ["[invalid"]
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let result = cfg.validate_and_compile();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("pipeline.filters.include.users"),
            "error should mention field name, got: {err_msg}"
        );
    }

    #[test]
    fn test_validate_and_compile_invalid_log_level_returns_err() {
        let mut cfg = default_config();
        cfg.logging.level = "invalid".into();
        assert!(cfg.validate_and_compile().is_err());
    }

    #[test]
    fn test_validate_and_compile_unknown_field_returns_err() {
        let mut cfg = default_config();
        cfg.pipeline.fields = Some(vec!["nonexistent_field".into()]);
        assert!(cfg.validate_and_compile().is_err());
    }

    #[test]
    fn test_validate_and_compile_matches_validate_on_ok() {
        let cfg = default_config();
        assert!(cfg.validate().is_ok());
        assert!(cfg.validate_and_compile().is_ok());
    }

    // ── pipeline.charts 跨字段依赖校验 ───────────────────
    #[test]
    fn test_validate_charts_requires_template_analysis() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[pipeline.charts]
output_dir = "charts/"
[pipeline.template_analysis]
enabled = false
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let result = cfg.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("pipeline.charts"), "actual: {err_msg}");
        assert!(
            err_msg.contains("[pipeline.template_analysis]"),
            "actual: {err_msg}"
        );
    }

    #[test]
    fn test_validate_charts_with_template_analysis_enabled_passes() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[pipeline.charts]
output_dir = "charts/"
[pipeline.template_analysis]
enabled = true
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_validate_and_compile_charts_requires_template_analysis() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[pipeline.charts]
output_dir = "charts/"
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let result = cfg.validate_and_compile();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("pipeline.charts"), "actual: {err_msg}");
    }

    #[test]
    fn test_apply_one_charts_output_dir() {
        let mut cfg = Config::default();
        cfg.apply_one("pipeline.charts.output_dir", "mydir")
            .expect("apply_one should succeed");
        assert_eq!(cfg.pipeline.charts.unwrap().output_dir, "mydir");
    }

    #[test]
    fn test_apply_one_charts_top_n() {
        let mut cfg = Config::default();
        cfg.apply_one("pipeline.charts.top_n", "20")
            .expect("apply_one should succeed");
        assert_eq!(cfg.pipeline.charts.unwrap().top_n, 20);
    }

    #[test]
    fn test_apply_one_charts_top_n_invalid() {
        let mut cfg = Config::default();
        let result = cfg.apply_one("pipeline.charts.top_n", "abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_one_charts_frequency_bar_false() {
        let mut cfg = Config::default();
        cfg.apply_one("pipeline.charts.frequency_bar", "false")
            .expect("apply_one should succeed");
        assert!(!cfg.pipeline.charts.unwrap().frequency_bar);
    }

    #[test]
    fn test_apply_one_charts_latency_hist_false() {
        let mut cfg = Config::default();
        cfg.apply_one("pipeline.charts.latency_hist", "false")
            .expect("apply_one should succeed");
        assert!(!cfg.pipeline.charts.unwrap().latency_hist);
    }

    // ── output_dir 非空校验 ─────────────────────────────
    #[test]
    fn test_validate_charts_empty_output_dir_is_rejected() {
        let mut cfg = default_config();
        cfg.pipeline = PipelineConfig {
            template_analysis: Some(TemplateAnalysisConfig { enabled: true }),
            charts: Some(ChartsConfig {
                output_dir: String::new(),
                ..ChartsConfig::default()
            }),
            ..Default::default()
        };
        let result = cfg.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("pipeline.charts.output_dir"), "actual: {msg}");
    }

    #[test]
    fn test_validate_charts_whitespace_output_dir_is_rejected() {
        let mut cfg = default_config();
        cfg.pipeline = PipelineConfig {
            template_analysis: Some(TemplateAnalysisConfig { enabled: true }),
            charts: Some(ChartsConfig {
                output_dir: "   ".into(),
                ..ChartsConfig::default()
            }),
            ..Default::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_apply_one_charts_output_dir_empty_is_rejected() {
        let mut cfg = Config::default();
        let result = cfg.apply_one("pipeline.charts.output_dir", "");
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_one_charts_output_dir_whitespace_is_rejected() {
        let mut cfg = Config::default();
        let result = cfg.apply_one("pipeline.charts.output_dir", "   ");
        assert!(result.is_err());
    }

    // ── top_n = 0 校验 ──────────────────────────────────
    #[test]
    fn test_validate_charts_top_n_zero_is_rejected() {
        let mut cfg = default_config();
        cfg.pipeline = PipelineConfig {
            template_analysis: Some(TemplateAnalysisConfig { enabled: true }),
            charts: Some(ChartsConfig {
                output_dir: "charts/".into(),
                top_n: 0,
                ..ChartsConfig::default()
            }),
            ..Default::default()
        };
        let result = cfg.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("pipeline.charts.top_n"), "actual: {msg}");
    }

    #[test]
    fn test_apply_one_charts_top_n_zero_is_rejected() {
        let mut cfg = Config::default();
        let result = cfg.apply_one("pipeline.charts.top_n", "0");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_and_compile_charts_empty_output_dir_is_rejected() {
        let mut cfg = default_config();
        cfg.pipeline = PipelineConfig {
            template_analysis: Some(TemplateAnalysisConfig { enabled: true }),
            charts: Some(ChartsConfig {
                output_dir: String::new(),
                ..ChartsConfig::default()
            }),
            ..Default::default()
        };
        assert!(cfg.validate_and_compile().is_err());
    }

    #[test]
    fn test_validate_and_compile_charts_top_n_zero_is_rejected() {
        let mut cfg = default_config();
        cfg.pipeline = PipelineConfig {
            template_analysis: Some(TemplateAnalysisConfig { enabled: true }),
            charts: Some(ChartsConfig {
                output_dir: "charts/".into(),
                top_n: 0,
                ..ChartsConfig::default()
            }),
            ..Default::default()
        };
        assert!(cfg.validate_and_compile().is_err());
    }

    #[test]
    fn test_validate_new_nested_format_passes() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[pipeline.filters]
enable = true
[pipeline.filters.include]
users = ["admin"]
[pipeline.filters.exclude]
users = ["guest"]
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert!(cfg.validate().is_ok());
    }
}
