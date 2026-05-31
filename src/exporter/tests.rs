use super::*;

// ── ExportStats ────────────────────────────────────────────
#[test]
fn test_export_stats_default() {
    let s = ExportStats::new();
    assert_eq!(s.exported, 0);
    assert_eq!(s.total(), 0);
}

#[test]
fn test_export_stats_record_success() {
    let mut s = ExportStats::new();
    s.record_success();
    s.record_success();
    assert_eq!(s.exported, 2);
    assert_eq!(s.total(), 2);
}

#[test]
fn test_export_stats_total_includes_all() {
    let mut s = ExportStats::new();
    s.exported = 5;
    s.skipped = 2;
    s.failed = 1;
    assert_eq!(s.total(), 8);
}

// ── strip_ip_prefix ────────────────────────────────────────
#[test]
fn test_strip_ip_prefix_with_prefix() {
    assert_eq!(strip_ip_prefix("::ffff:192.168.1.1"), "192.168.1.1");
}

#[test]
fn test_strip_ip_prefix_uppercase() {
    assert_eq!(strip_ip_prefix("::FFFF:10.0.0.1"), "10.0.0.1");
}

#[test]
fn test_strip_ip_prefix_no_prefix() {
    assert_eq!(strip_ip_prefix("192.168.1.1"), "192.168.1.1");
}

#[test]
fn test_strip_ip_prefix_ipv6() {
    assert_eq!(strip_ip_prefix("2001:db8::1"), "2001:db8::1");
}

#[test]
fn test_strip_ip_prefix_empty() {
    assert_eq!(strip_ip_prefix(""), "");
}

// ── f32_ms_to_i64 ──────────────────────────────────────────
#[test]
fn test_f32_ms_to_i64_normal() {
    assert_eq!(f32_ms_to_i64(100.0_f32), 100);
}

#[test]
fn test_f32_ms_to_i64_nan() {
    assert_eq!(f32_ms_to_i64(f32::NAN), 0);
}

#[test]
fn test_f32_ms_to_i64_pos_infinity() {
    assert_eq!(f32_ms_to_i64(f32::INFINITY), 0);
}

#[test]
fn test_f32_ms_to_i64_neg_infinity() {
    assert_eq!(f32_ms_to_i64(f32::NEG_INFINITY), 0);
}

#[test]
fn test_f32_ms_to_i64_zero() {
    assert_eq!(f32_ms_to_i64(0.0), 0);
}

#[test]
fn test_f32_ms_to_i64_negative() {
    assert_eq!(f32_ms_to_i64(-50.0), -50);
}

// ── ensure_parent_dir ──────────────────────────────────────
#[test]
fn test_ensure_parent_dir_existing() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("out.csv");
    // Parent exists → should not error
    ensure_parent_dir(&path).unwrap();
}

#[test]
fn test_ensure_parent_dir_creates_new() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("sub/dir/out.csv");
    ensure_parent_dir(&path).unwrap();
    assert!(dir.path().join("sub/dir").exists());
}

// ── ExporterManager constructors ───────────────────────────
#[test]
fn test_from_csv_constructor() {
    let exporter = CsvExporter::new(std::path::PathBuf::from("/tmp/test.csv"));
    let manager = ExporterManager::from_csv(exporter);
    assert_eq!(manager.name(), "CSV");
}

#[test]
fn test_from_config_sqlite_path() {
    use crate::config::SqliteExporterConfig as SqliteExporterCfg;
    use crate::config::{Config, ExporterConfig, SqllogConfig};
    let cfg = Config {
        exporter: ExporterConfig {
            csv: None,
            sqlite: Some(SqliteExporterCfg {
                database_url: "/tmp/test_mod.db".to_string(),
                table_name: "records".to_string(),
                overwrite: true,
                append: false,
                batch_size: 10_000,
            }),
        },
        sqllog: SqllogConfig {
            inputs: vec!["sqllogs".to_string()],
            path_deprecated: None,
        },
        ..Default::default()
    };
    let manager = ExporterManager::from_config(&cfg).unwrap();
    assert_eq!(manager.name(), "SQLite");
}

#[test]
fn test_from_config_no_exporters_error() {
    use crate::config::{Config, ExporterConfig, SqllogConfig};
    let cfg = Config {
        exporter: ExporterConfig {
            csv: None,
            sqlite: None,
        },
        sqllog: SqllogConfig {
            inputs: vec!["sqllogs".to_string()],
            path_deprecated: None,
        },
        ..Default::default()
    };
    let result = ExporterManager::from_config(&cfg);
    assert!(result.is_err());
}

#[test]
fn test_exporter_manager_log_stats_no_panic() {
    let exporter = CsvExporter::new(std::path::PathBuf::from("/tmp/test.csv"));
    let manager = ExporterManager::from_csv(exporter);
    manager.log_stats();
}

#[test]
fn test_exporter_manager_debug_format() {
    let exporter = CsvExporter::new(std::path::PathBuf::from("/tmp/test.csv"));
    let manager = ExporterManager::from_csv(exporter);
    let s = format!("{manager:?}");
    assert!(s.contains("ExporterManager"));
}

#[test]
fn test_f32_ms_to_i64_large_positive() {
    // Value larger than i64::MAX should return i64::MAX
    let result = f32_ms_to_i64(f32::MAX);
    assert_eq!(result, i64::MAX);
}

#[test]
fn test_strip_ip_prefix_colon_non_ffff() {
    // Starts with ':' but not the exact ffff prefix
    assert_eq!(strip_ip_prefix("::1"), "::1");
}
