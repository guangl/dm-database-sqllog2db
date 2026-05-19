use super::{Config, PIPELINE_MIGRATION_HINT};
use crate::error::{ConfigError, Error, Result};

impl Config {
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
        Ok(compiled)
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
    use crate::config::{CsvExporterConfig, SqliteExporterConfig};
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
