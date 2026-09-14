use super::*;
use crate::config::LoggingConfig;

fn make_logging_config(dir: &std::path::Path, level: &str) -> LoggingConfig {
    LoggingConfig {
        file: Some(dir.join("app.log").to_str().unwrap().to_string()),
        level: level.to_string(),
        retention_days: 7,
    }
}

#[test]
fn test_init_logging_valid_level_info() {
    let dir = tempfile::TempDir::new().unwrap();
    let cfg = make_logging_config(dir.path(), "info");
    // May fail silently if logger already registered in this process; that is intentional
    let result = init_logging(&cfg, false);
    assert!(result.is_ok());
    assert!(dir.path().join("app.log").exists());
}

#[test]
fn test_init_logging_invalid_level_returns_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let cfg = make_logging_config(dir.path(), "nonsense_level");
    let result = init_logging(&cfg, false);
    assert!(result.is_err());
}

#[test]
fn test_init_logging_creates_parent_dir() {
    let dir = tempfile::TempDir::new().unwrap();
    let nested = dir.path().join("sub/nested");
    let cfg = LoggingConfig {
        file: Some(nested.join("app.log").to_str().unwrap().to_string()),
        level: "warn".to_string(),
        retention_days: 7,
    };
    let result = init_logging(&cfg, false);
    assert!(result.is_ok());
    assert!(nested.exists());
}

#[test]
fn test_init_logging_all_valid_levels() {
    let dir = tempfile::TempDir::new().unwrap();
    for level in &["trace", "debug", "info", "warn", "error"] {
        let cfg = LoggingConfig {
            file: Some(
                dir.path()
                    .join(format!("{level}.log"))
                    .to_str()
                    .unwrap()
                    .to_string(),
            ),
            level: (*level).to_string(),
            retention_days: 7,
        };
        assert!(init_logging(&cfg, false).is_ok());
    }
}

#[test]
fn test_init_logging_with_stdout() {
    let dir = tempfile::TempDir::new().unwrap();
    let cfg = make_logging_config(dir.path(), "warn");
    // log_to_stdout=true exercises the stdout write path in SimpleLogger::log
    let result = init_logging(&cfg, true);
    assert!(result.is_ok());
}

#[test]
fn test_parse_log_level_all() {
    for level in &["trace", "debug", "info", "warn", "error"] {
        assert!(parse_log_level(level).is_ok());
    }
}

#[test]
fn test_parse_log_level_uppercase() {
    // parse_log_level lowercases, so uppercase should also work
    assert!(parse_log_level("INFO").is_ok());
    assert!(parse_log_level("DEBUG").is_ok());
}

#[test]
fn test_parse_log_level_invalid() {
    assert!(parse_log_level("verbose").is_err());
    assert!(parse_log_level("").is_err());
}
