use super::*;
use crate::config::CsvExporterConfig;
use crate::pipeline::OutputConfig;

fn default_config() -> Config {
    Config {
        logging: Some(crate::config::LoggingConfig::default()),
        sqllog: crate::config::SqllogConfig {
            inputs: vec!["sqllogs".into()],
            ..Default::default()
        },
        exporter: crate::config::ExporterConfig {
            csv: Some(CsvExporterConfig::default()),
            ..Default::default()
        },
        ..Config::default()
    }
}

// ── validate ───────────────────────────────────────────────
#[test]
fn test_validate_default_config_passes() {
    assert!(default_config().validate().is_ok());
}

#[test]
fn test_validate_for_stats_does_not_require_exporter() {
    let mut cfg = default_config();
    cfg.exporter.parquet = None;
    cfg.exporter.csv = None;
    assert!(cfg.validate().is_err());
    assert!(cfg.validate_for_stats().is_ok());
}

#[test]
fn test_validate_empty_logging_file() {
    let mut cfg = default_config();
    cfg.logging.as_mut().unwrap().file = Some("  ".into());
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
fn test_validate_invalid_log_level() {
    let mut cfg = default_config();
    cfg.logging.as_mut().unwrap().level = "invalid".into();
    assert!(cfg.validate().is_err());
}

#[test]
fn test_validate_retention_days_zero() {
    let mut cfg = default_config();
    cfg.logging.as_mut().unwrap().retention_days = 0;
    assert!(cfg.validate().is_err());
}

#[test]
fn test_validate_retention_days_over_365() {
    let mut cfg = default_config();
    cfg.logging.as_mut().unwrap().retention_days = 366;
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
    cfg.exporter.parquet = None;
    cfg.exporter.csv = None;
    assert!(cfg.validate().is_err());
}

#[test]
fn test_validate_new_nested_format_passes() {
    let toml = r#"
[sqllog]
inputs = ["sqllogs"]
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
