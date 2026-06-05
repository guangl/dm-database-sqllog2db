use super::Config;
use crate::error::{ConfigError, Error, Result};

impl Config {
    pub fn validate(&self) -> Result<()> {
        self.logging.validate()?;
        self.exporter.validate()?;
        self.sqllog.validate()?;
        self.validate_output_fields()?;
        self.validate_stats_time_fields()?;
        self.validate_error_log()?;
        Ok(())
    }

    fn validate_error_log(&self) -> Result<()> {
        if let Some(err_cfg) = &self.error {
            if err_cfg.file.trim().is_empty() {
                return Err(Error::Config(ConfigError::InvalidValue {
                    field: "error.file".to_string(),
                    value: err_cfg.file.clone(),
                    reason: "error log file path must not be empty or whitespace".to_string(),
                }));
            }
        }
        Ok(())
    }

    fn validate_stats_time_fields(&self) -> Result<()> {
        crate::stats::config::validate_stats_time_range(&self.stats)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CsvExporterConfig, SqliteExporterConfig};
    use crate::pipeline::OutputConfig;

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
    fn test_validate_rejects_whitespace_input_entry() {
        let mut cfg = default_config();
        cfg.sqllog.inputs = vec!["  ".to_string()];
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_no_exporters() {
        let mut cfg = default_config();
        cfg.exporter.csv = None;
        assert!(cfg.validate().is_err());
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

    #[test]
    fn test_validate_new_nested_format_passes() {
        let toml = r#"
[sqllog]
inputs = ["sqllogs"]
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

    #[test]
    fn test_validate_rejects_legacy_sqllog_path_key() {
        let toml = r#"
[sqllog]
path = "sqllogs"
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let result = cfg.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("sqllog.path"),
            "expect sqllog.path field name; got: {err_msg}"
        );
        assert!(
            err_msg.contains("inputs"),
            "expect migration hint to mention inputs; got: {err_msg}"
        );
    }

    #[test]
    fn test_validate_new_top_level_format_passes() {
        let toml = r#"
[sqllog]
inputs = ["sqllogs"]
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

    // ── stats.from / stats.to 时间格式校验 ───────────────────
    #[test]
    fn test_validate_rejects_invalid_stats_from() {
        let mut cfg = default_config();
        cfg.stats.from = Some("not-a-date".into());
        let result = cfg.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("stats.from"), "actual: {msg}");
        assert!(msg.contains("YYYY-MM-DD"), "actual: {msg}");
    }

    #[test]
    fn test_validate_rejects_invalid_stats_to() {
        let mut cfg = default_config();
        cfg.stats.to = Some("20240101".into());
        let result = cfg.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("stats.to"), "actual: {msg}");
        assert!(msg.contains("YYYY-MM-DD"), "actual: {msg}");
    }

    #[test]
    fn test_validate_accepts_valid_stats_time_strings() {
        let mut cfg = default_config();
        cfg.stats.from = Some("2024-01-01".into());
        cfg.stats.to = Some("2024-01-31 23:59:59".into());
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_validate_accepts_none_stats_time() {
        let cfg = default_config();
        // stats 默认全 None，不应触发验证错误
        assert!(cfg.validate().is_ok());
    }
}
