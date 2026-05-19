use super::Config;
use crate::error::{ConfigError, Error, Result};

impl Config {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── apply_one (private method tests — must stay in this module) ─────────
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
        assert!(
            cfg.apply_one("pipeline.template_analysis.enabled", "true")
                .is_err()
        );
        assert!(cfg.apply_one("pipeline.normalize.enable", "true").is_err());
        assert!(cfg.apply_one("pipeline.fields", "sql,ts").is_err());
    }
}
