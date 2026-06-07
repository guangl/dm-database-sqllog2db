use crate::config::{
    Config, CsvExporterConfig, ExporterConfig, LoggingConfig, SqliteExporterConfig,
};

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

#[test]
fn test_config_default_stats_all_none() {
    let cfg = Config::default();
    assert!(cfg.stats.from.is_none());
    assert!(cfg.stats.to.is_none());
    assert!(cfg.stats.top.is_none());
}

#[test]
fn test_config_parses_stats_section() {
    let toml = "[stats]\nfrom = \"2024-01-01\"\nto = \"2024-01-31\"\ntop = 5\n[sqllog]\ninputs = [\"sqllogs\"]\n[exporter.csv]\nfile = \"out.csv\"";
    let cfg: Config = toml::from_str(toml).unwrap();
    assert_eq!(cfg.stats.from, Some("2024-01-01".to_string()));
    assert_eq!(cfg.stats.to, Some("2024-01-31".to_string()));
    assert_eq!(cfg.stats.top, Some(5));
}

#[test]
fn test_config_missing_stats_section_defaults_to_none() {
    let toml = "[sqllog]\ninputs=[\"sqllogs\"]\n[exporter.csv]\nfile=\"out.csv\"";
    let cfg: Config = toml::from_str(toml).unwrap();
    assert!(cfg.stats.from.is_none());
    assert!(cfg.stats.to.is_none());
    assert!(cfg.stats.top.is_none());
}
