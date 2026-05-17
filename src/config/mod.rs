pub mod exporter;
pub mod logging;
pub mod resume;
pub mod sqllog;

pub use exporter::{CsvExporterConfig, ExporterConfig, SqliteExporterConfig};
pub use logging::{LOG_LEVELS, LoggingConfig};
pub use resume::ResumeConfig;
pub use sqllog::SqllogConfig;

use crate::error::{ConfigError, Error, Result};
use crate::pipeline::{
    ChartsConfig, FiltersFeature, NormalizeConfig, OutputConfig, TemplateConfig,
};
use serde::Deserialize;
use std::path::Path;

const PIPELINE_MIGRATION_HINT: &str = "配置格式已升级，请迁移以下字段：\n  [pipeline.template_analysis] → [template]\n  \
     [pipeline.charts] → [charts]\n  [pipeline.normalize] → [replace_parameters]\n  \
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
    pub template: Option<TemplateConfig>,
    #[serde(default)]
    pub filter: Option<FiltersFeature>,
    #[serde(default)]
    pub charts: Option<ChartsConfig>,
    #[serde(default)]
    pub output: Option<OutputConfig>,
    /// 旧路径检测：捕获 [pipeline] 表（若用户仍用旧格式）。
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

    pub fn validate(&self) -> Result<()> {
        if self.pipeline_deprecated.is_some() {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "[pipeline]".to_string(),
                value: String::new(),
                reason: PIPELINE_MIGRATION_HINT.to_string(),
            }));
        }
        self.logging.validate()?;
        self.exporter.validate()?;
        self.sqllog.validate()?;
        self.validate_filter()?;
        self.validate_output_fields()?;
        self.validate_charts()?;
        self.validate_template()?;
        Ok(())
    }

    /// 等价于 `validate()` 但额外返回已编译的过滤器对，供调用方复用，
    /// 消除 `run` 子命令路径中 regex 的双重编译。
    ///
    /// 返回值语义：
    /// - `Ok(None)`：无过滤器配置 或 `filter.enable == false`
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
        if self.pipeline_deprecated.is_some() {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "[pipeline]".to_string(),
                value: String::new(),
                reason: PIPELINE_MIGRATION_HINT.to_string(),
            }));
        }
        self.logging.validate()?;
        self.exporter.validate()?;
        self.sqllog.validate()?;

        let compiled = if let Some(filters) = &self.filter {
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

        self.validate_output_fields()?;
        self.validate_charts()?;
        self.validate_template()?;
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

            "filter.enable" => {
                self.filter.get_or_insert_with(Default::default).enable = parse_bool(value)?;
            }
            "replace_parameters.enable" => {
                self.replace_parameters
                    .get_or_insert_with(Default::default)
                    .enable = parse_bool(value)?;
            }
            "template.enable" => {
                self.template.get_or_insert_with(Default::default).enable = parse_bool(value)?;
            }
            "template.output_csv_path" => {
                self.template
                    .get_or_insert_with(Default::default)
                    .output_csv_path = value.to_string();
            }
            "template.output_sqlite_table" => {
                self.template
                    .get_or_insert_with(Default::default)
                    .output_sqlite_table = value.to_string();
            }

            "charts.output_dir" => {
                if value.trim().is_empty() {
                    return Err(Error::Config(ConfigError::InvalidValue {
                        field: key.to_string(),
                        value: value.to_string(),
                        reason: "charts output_dir cannot be empty".to_string(),
                    }));
                }
                self.charts.get_or_insert_with(Default::default).output_dir = value.to_string();
            }
            "charts.top_n" => {
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
                self.charts.get_or_insert_with(Default::default).top_n = parsed;
            }
            "charts.frequency_bar" => {
                self.charts
                    .get_or_insert_with(Default::default)
                    .frequency_bar = parse_bool(value)?;
            }
            "charts.latency_hist" => {
                self.charts
                    .get_or_insert_with(Default::default)
                    .latency_hist = parse_bool(value)?;
            }
            "charts.trend_line" => {
                self.charts.get_or_insert_with(Default::default).trend_line = parse_bool(value)?;
            }
            "charts.user_pie" => {
                self.charts.get_or_insert_with(Default::default).user_pie = parse_bool(value)?;
            }

            "output.fields" => {
                let parsed: Vec<String> = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                self.output.get_or_insert_with(Default::default).fields = Some(parsed);
            }

            _ => return Err(unknown()),
        }
        Ok(())
    }

    fn validate_filter(&self) -> Result<()> {
        if let Some(filters) = &self.filter {
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

    fn validate_output_fields(&self) -> Result<()> {
        if let Some(names) = self.output.as_ref().and_then(|o| o.fields.as_ref()) {
            for name in names {
                if !crate::pipeline::FIELD_NAMES.contains(&name.as_str()) {
                    return Err(Error::Config(ConfigError::InvalidValue {
                        field: "output.fields".to_string(),
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

    fn validate_template(&self) -> Result<()> {
        if let Some(tmpl) = &self.template {
            let name = tmpl.output_sqlite_table.trim();
            if !name.is_empty() {
                let mut chars = name.chars();
                let valid = chars
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                    && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
                if !valid {
                    return Err(Error::Config(ConfigError::InvalidValue {
                        field: "template.output_sqlite_table".to_string(),
                        value: name.to_string(),
                        reason: "table name must start with a letter or underscore \
                                 and contain only ASCII alphanumeric or underscore"
                            .to_string(),
                    }));
                }
            }
        }
        Ok(())
    }

    fn validate_charts(&self) -> Result<()> {
        if let Some(charts) = &self.charts {
            let template_enabled = self.template.as_ref().is_some_and(|t| t.enable);
            if !template_enabled {
                return Err(Error::Config(ConfigError::InvalidValue {
                    field: "[charts]".to_string(),
                    value: String::new(),
                    reason: "启用 [charts] 需要先设置 [template]\nenable = true".to_string(),
                }));
            }
            if charts.output_dir.trim().is_empty() {
                return Err(Error::Config(ConfigError::InvalidValue {
                    field: "charts.output_dir".to_string(),
                    value: charts.output_dir.clone(),
                    reason: "charts output_dir cannot be empty".to_string(),
                }));
            }
            if charts.top_n == 0 {
                return Err(Error::Config(ConfigError::InvalidValue {
                    field: "charts.top_n".to_string(),
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
    use crate::pipeline::{ChartsConfig, NormalizeConfig, OutputConfig, TemplateConfig};

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
[filter]
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
            err_msg.contains("filter."),
            "error should mention filter field name, got: {err_msg}"
        );
    }

    #[test]
    fn test_validate_valid_regex_in_filters() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[filter]
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
[filter]
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
[filter]
enable = true
usernames = ["^admin.*"]
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let result = cfg.validate_and_compile().expect("config valid");
        let pair = result.expect("filter.enable=true should return Some");
        assert!(pair.0.has_any_filters());
    }

    #[test]
    fn test_validate_and_compile_invalid_regex_returns_err() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[filter]
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
            err_msg.contains("filter."),
            "error should mention filter field name, got: {err_msg}"
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
        cfg.output = Some(OutputConfig {
            fields: Some(vec!["nonexistent_field".into()]),
        });
        assert!(cfg.validate_and_compile().is_err());
    }

    #[test]
    fn test_validate_and_compile_matches_validate_on_ok() {
        let cfg = default_config();
        assert!(cfg.validate().is_ok());
        assert!(cfg.validate_and_compile().is_ok());
    }

    // ── [charts] 跨字段依赖校验 ───────────────────
    #[test]
    fn test_validate_charts_requires_template_analysis() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[charts]
output_dir = "charts/"
[template]
enable = false
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let result = cfg.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("[charts]"), "actual: {err_msg}");
        assert!(err_msg.contains("[template]"), "actual: {err_msg}");
    }

    #[test]
    fn test_validate_charts_with_template_analysis_enabled_passes() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[charts]
output_dir = "charts/"
[template]
enable = true
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
[charts]
output_dir = "charts/"
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let result = cfg.validate_and_compile();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("[charts]"), "actual: {err_msg}");
    }

    #[test]
    fn test_apply_one_charts_output_dir() {
        let mut cfg = Config::default();
        cfg.apply_one("charts.output_dir", "mydir")
            .expect("apply_one should succeed");
        assert_eq!(cfg.charts.unwrap().output_dir, "mydir");
    }

    #[test]
    fn test_apply_one_charts_top_n() {
        let mut cfg = Config::default();
        cfg.apply_one("charts.top_n", "20")
            .expect("apply_one should succeed");
        assert_eq!(cfg.charts.unwrap().top_n, 20);
    }

    #[test]
    fn test_apply_one_charts_top_n_invalid() {
        let mut cfg = Config::default();
        let result = cfg.apply_one("charts.top_n", "abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_one_charts_frequency_bar_false() {
        let mut cfg = Config::default();
        cfg.apply_one("charts.frequency_bar", "false")
            .expect("apply_one should succeed");
        assert!(!cfg.charts.unwrap().frequency_bar);
    }

    #[test]
    fn test_apply_one_charts_latency_hist_false() {
        let mut cfg = Config::default();
        cfg.apply_one("charts.latency_hist", "false")
            .expect("apply_one should succeed");
        assert!(!cfg.charts.unwrap().latency_hist);
    }

    // ── output_dir 非空校验 ─────────────────────────────
    #[test]
    fn test_validate_charts_empty_output_dir_is_rejected() {
        let mut cfg = default_config();
        cfg.template = Some(TemplateConfig {
            enable: true,
            ..TemplateConfig::default()
        });
        cfg.charts = Some(ChartsConfig {
            output_dir: String::new(),
            ..ChartsConfig::default()
        });
        let result = cfg.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("charts.output_dir"), "actual: {msg}");
    }

    #[test]
    fn test_validate_charts_whitespace_output_dir_is_rejected() {
        let mut cfg = default_config();
        cfg.template = Some(TemplateConfig {
            enable: true,
            ..TemplateConfig::default()
        });
        cfg.charts = Some(ChartsConfig {
            output_dir: "   ".into(),
            ..ChartsConfig::default()
        });
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_apply_one_charts_output_dir_empty_is_rejected() {
        let mut cfg = Config::default();
        let result = cfg.apply_one("charts.output_dir", "");
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_one_charts_output_dir_whitespace_is_rejected() {
        let mut cfg = Config::default();
        let result = cfg.apply_one("charts.output_dir", "   ");
        assert!(result.is_err());
    }

    // ── top_n = 0 校验 ──────────────────────────────────
    #[test]
    fn test_validate_charts_top_n_zero_is_rejected() {
        let mut cfg = default_config();
        cfg.template = Some(TemplateConfig {
            enable: true,
            ..TemplateConfig::default()
        });
        cfg.charts = Some(ChartsConfig {
            output_dir: "charts/".into(),
            top_n: 0,
            ..ChartsConfig::default()
        });
        let result = cfg.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("charts.top_n"), "actual: {msg}");
    }

    #[test]
    fn test_apply_one_charts_top_n_zero_is_rejected() {
        let mut cfg = Config::default();
        let result = cfg.apply_one("charts.top_n", "0");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_and_compile_charts_empty_output_dir_is_rejected() {
        let mut cfg = default_config();
        cfg.template = Some(TemplateConfig {
            enable: true,
            ..TemplateConfig::default()
        });
        cfg.charts = Some(ChartsConfig {
            output_dir: String::new(),
            ..ChartsConfig::default()
        });
        assert!(cfg.validate_and_compile().is_err());
    }

    #[test]
    fn test_validate_and_compile_charts_top_n_zero_is_rejected() {
        let mut cfg = default_config();
        cfg.template = Some(TemplateConfig {
            enable: true,
            ..TemplateConfig::default()
        });
        cfg.charts = Some(ChartsConfig {
            output_dir: "charts/".into(),
            top_n: 0,
            ..ChartsConfig::default()
        });
        assert!(cfg.validate_and_compile().is_err());
    }

    #[test]
    fn test_validate_new_nested_format_passes() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[filter]
enable = true
[filter.include]
users = ["admin"]
[filter.exclude]
users = ["guest"]
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert!(cfg.validate().is_ok());
    }

    // ── 旧路径检测 ─────────────────────────────────────────────
    #[test]
    fn test_validate_legacy_pipeline_path_rejected() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[pipeline.template_analysis]
enabled = true
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let result = cfg.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("[pipeline.template_analysis] → [template]"),
            "actual: {err_msg}"
        );
        assert!(
            err_msg.contains("[pipeline.charts] → [charts]"),
            "actual: {err_msg}"
        );
        assert!(
            err_msg.contains("[pipeline.normalize] → [replace_parameters]"),
            "actual: {err_msg}"
        );
        assert!(
            err_msg.contains("[pipeline.filters.*] → [filter.*]"),
            "actual: {err_msg}"
        );
        assert!(
            err_msg.contains("[pipeline.fields] → [output.fields]"),
            "actual: {err_msg}"
        );
    }

    #[test]
    fn test_validate_new_top_level_format_passes() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[template]
enable = true
[filter]
enable = false
[output]
fields = ["ts", "sql", "username"]
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert!(cfg.validate().is_ok());
    }

    // ── apply_one 新路径测试 ──────────────────────────────────
    #[test]
    fn test_apply_one_template_enable() {
        let mut cfg = Config::default();
        cfg.apply_one("template.enable", "true")
            .expect("apply_one should succeed");
        assert!(cfg.template.unwrap().enable);
    }

    #[test]
    fn test_apply_one_template_output_csv_path() {
        let mut cfg = Config::default();
        cfg.apply_one("template.output_csv_path", "/tmp/a.csv")
            .expect("apply_one should succeed");
        assert_eq!(cfg.template.unwrap().output_csv_path, "/tmp/a.csv");
    }

    #[test]
    fn test_apply_one_template_output_sqlite_table() {
        let mut cfg = Config::default();
        cfg.apply_one("template.output_sqlite_table", "tpl")
            .expect("apply_one should succeed");
        assert_eq!(cfg.template.unwrap().output_sqlite_table, "tpl");
    }

    #[test]
    fn test_apply_one_replace_parameters_enable() {
        let mut cfg = Config::default();
        cfg.apply_one("replace_parameters.enable", "false")
            .expect("apply_one should succeed");
        assert!(!cfg.replace_parameters.unwrap().enable);
    }

    #[test]
    fn test_apply_one_filter_enable() {
        let mut cfg = Config::default();
        cfg.apply_one("filter.enable", "true")
            .expect("apply_one should succeed");
        assert!(cfg.filter.unwrap().enable);
    }

    #[test]
    fn test_apply_one_output_fields() {
        let mut cfg = Config::default();
        cfg.apply_one("output.fields", "sql,ts,username")
            .expect("apply_one should succeed");
        let fields = cfg.output.unwrap().fields.unwrap();
        assert_eq!(fields, vec!["sql", "ts", "username"]);
    }

    #[test]
    fn test_apply_one_rejects_legacy_pipeline_paths() {
        let mut cfg = Config::default();
        assert!(cfg.apply_one("pipeline.filters.enable", "true").is_err());
        assert!(cfg.apply_one("pipeline.charts.output_dir", "out/").is_err());
        assert!(
            cfg.apply_one("pipeline.template_analysis.enabled", "true")
                .is_err()
        );
        assert!(cfg.apply_one("pipeline.normalize.enable", "true").is_err());
        assert!(cfg.apply_one("pipeline.fields", "sql,ts").is_err());
    }

    // ── output.fields 校验 ───────────────────────────────────
    #[test]
    fn test_validate_output_fields_unknown_field_rejected() {
        let mut cfg = default_config();
        cfg.output = Some(OutputConfig {
            fields: Some(vec!["unknown_field".into()]),
        });
        let result = cfg.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("output.fields"), "actual: {msg}");
        assert!(msg.contains("unknown_field"), "actual: {msg}");
    }

    // ── validate_and_compile 新格式 ──────────────────────────
    #[test]
    fn test_validate_and_compile_new_format_filter_enabled() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[filter]
enable = true
usernames = ["^admin.*"]
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let result = cfg.validate_and_compile().expect("config valid");
        let pair = result.expect("filter.enable=true should return Some");
        assert!(pair.0.has_any_filters());
    }

    #[test]
    fn test_validate_and_compile_new_format_filter_disabled() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[filter]
enable = false
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let result = cfg.validate_and_compile().expect("config valid");
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_legacy_pipeline_rejected_in_validate_and_compile() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[pipeline.filters]
enable = true
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let result = cfg.validate_and_compile();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("[pipeline]"), "actual: {err_msg}");
    }

    #[test]
    fn test_apply_one_charts_all_bool_fields() {
        let mut cfg = Config::default();
        cfg.apply_one("charts.trend_line", "false")
            .expect("apply_one should succeed");
        assert!(!cfg.charts.as_ref().unwrap().trend_line);
        cfg.apply_one("charts.user_pie", "false")
            .expect("apply_one should succeed");
        assert!(!cfg.charts.as_ref().unwrap().user_pie);
    }

    // ── validate_template ─────────────────────────────────────────
    #[test]
    fn test_validate_template_sqlite_table_invalid_leading_digit() {
        let mut cfg = default_config();
        cfg.template = Some(TemplateConfig {
            enable: true,
            output_sqlite_table: "1bad".into(),
            ..TemplateConfig::default()
        });
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_template_sqlite_table_invalid_hyphen() {
        let mut cfg = default_config();
        cfg.template = Some(TemplateConfig {
            enable: true,
            output_sqlite_table: "my-table".into(),
            ..TemplateConfig::default()
        });
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_template_sqlite_table_valid() {
        let mut cfg = default_config();
        cfg.template = Some(TemplateConfig {
            enable: true,
            output_sqlite_table: "sql_templates".into(),
            ..TemplateConfig::default()
        });
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_validate_template_sqlite_table_empty_ok() {
        let mut cfg = default_config();
        cfg.template = Some(TemplateConfig {
            enable: true,
            output_sqlite_table: String::new(),
            ..TemplateConfig::default()
        });
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_config_has_5_top_level_optional_fields() {
        // 确保 5 个顶层字段默认值为 None
        let cfg = default_config();
        assert!(cfg.replace_parameters.is_none());
        assert!(cfg.template.is_none());
        assert!(cfg.filter.is_none());
        assert!(cfg.charts.is_none());
        assert!(cfg.output.is_none());
    }

    #[test]
    fn test_validate_and_compile_normalize_not_needed_for_compile() {
        // replace_parameters 不影响 validate_and_compile 的 compiled 结果
        let mut cfg = default_config();
        cfg.replace_parameters = Some(NormalizeConfig {
            enable: true,
            placeholders: vec![],
        });
        let result = cfg.validate_and_compile().expect("config valid");
        assert!(result.is_none()); // 没有 filter，返回 None
    }
}
