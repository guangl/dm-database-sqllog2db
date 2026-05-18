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

    #[test]
    fn test_apply_one_charts_top_n_zero_is_rejected() {
        let mut cfg = Config::default();
        let result = cfg.apply_one("charts.top_n", "0");
        assert!(result.is_err());
    }

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
}
