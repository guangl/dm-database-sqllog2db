use super::*;
use crate::config::{Config, CsvExporterConfig, ExporterConfig, SqllogConfig};

fn config_with_log_dir(dir: &str) -> Config {
    Config {
        sqllog: SqllogConfig {
            inputs: vec![dir.to_string()],
            path_deprecated: None,
        },
        ..Default::default()
    }
}

// ── PreflightResult ───────────────────────────────────────────

#[test]
fn test_preflight_result_no_errors() {
    let result = PreflightResult::default();
    assert!(!result.has_errors());
    assert!(!result.print_and_check());
}

#[test]
fn test_preflight_result_with_errors() {
    let mut result = PreflightResult::default();
    result.errors.push("some error".to_string());
    assert!(result.has_errors());
    assert!(result.print_and_check());
}

#[test]
fn test_preflight_result_warnings_no_error() {
    let mut result = PreflightResult::default();
    result.warnings.push("some warning".to_string());
    assert!(!result.has_errors());
    assert!(!result.print_and_check());
}

// ── check: log dir ────────────────────────────────────────────

#[test]
fn test_check_nonexistent_log_dir_produces_error() {
    let cfg = config_with_log_dir("/this/path/definitely/does/not/exist");
    let result = check(&cfg);
    assert!(result.has_errors());
    assert!(result.errors[0].contains("不存在"));
}

#[test]
fn test_check_single_log_file_is_valid() {
    let dir = tempfile::TempDir::new().unwrap();
    let file_path = dir.path().join("test.log");
    std::fs::write(&file_path, "").unwrap();
    let cfg = config_with_log_dir(file_path.to_str().unwrap());
    let result = check(&cfg);
    assert!(!result.has_errors());
}

#[test]
fn test_check_log_dir_empty_produces_warning() {
    let dir = tempfile::TempDir::new().unwrap();
    let cfg = config_with_log_dir(dir.path().to_str().unwrap());
    let result = check(&cfg);
    assert!(!result.has_errors());
    assert!(!result.warnings.is_empty());
}

#[test]
fn test_check_log_dir_with_log_files_no_warning() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("test.log"), "").unwrap();
    let cfg = config_with_log_dir(dir.path().to_str().unwrap());
    let result = check(&cfg);
    assert!(!result.has_errors());
    assert!(result.warnings.is_empty());
}

#[test]
fn test_check_glob_pattern_with_matches() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("a.log"), "").unwrap();
    let pattern = format!("{}/*.log", dir.path().display());
    let cfg = config_with_log_dir(&pattern);
    let result = check(&cfg);
    assert!(!result.has_errors());
    assert!(result.warnings.is_empty());
}

#[test]
fn test_check_glob_pattern_no_matches_produces_warning() {
    let dir = tempfile::TempDir::new().unwrap();
    let pattern = format!("{}/nomatch*.log", dir.path().display());
    let cfg = config_with_log_dir(&pattern);
    let result = check(&cfg);
    assert!(!result.has_errors());
    assert!(!result.warnings.is_empty());
}

// ── check: output writable ────────────────────────────────────

#[test]
fn test_check_csv_output_in_existing_dir() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("test.log"), "").unwrap();
    let out_file = dir.path().join("out.csv");
    let mut cfg = config_with_log_dir(dir.path().to_str().unwrap());
    cfg.exporter = ExporterConfig {
        parquet: None,
        csv: Some(CsvExporterConfig {
            file: out_file.to_str().unwrap().to_string(),
            overwrite: false,
            append: false,
            ..CsvExporterConfig::default()
        }),
    };
    let result = check(&cfg);
    assert!(!result.has_errors());
}

#[test]
fn test_check_csv_existing_writable_file() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("test.log"), "").unwrap();
    let out_file = dir.path().join("out.csv");
    std::fs::write(&out_file, "").unwrap(); // pre-create file
    let mut cfg = config_with_log_dir(dir.path().to_str().unwrap());
    cfg.exporter = ExporterConfig {
        parquet: None,
        csv: Some(CsvExporterConfig {
            file: out_file.to_str().unwrap().to_string(),
            overwrite: false,
            append: false,
            ..CsvExporterConfig::default()
        }),
    };
    let result = check(&cfg);
    assert!(!result.has_errors());
}
