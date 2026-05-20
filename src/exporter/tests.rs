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

// ── ExporterKind DryRun ────────────────────────────────────
#[test]
fn test_dry_run_exporter_counts_records() {
    let mut e = ExporterKind::DryRun {
        stats: ExportStats::default(),
    };
    e.initialize().unwrap();
    // Manually add some counts
    if let ExporterKind::DryRun { ref mut stats } = e {
        stats.exported = 5;
    }
    let snap = e.stats_snapshot().unwrap();
    assert_eq!(snap.exported, 5);
}

// ── ExporterManager constructors ───────────────────────────
#[test]
fn test_from_csv_constructor() {
    let exporter = CsvExporter::new(std::path::PathBuf::from("/tmp/test.csv"));
    let manager = ExporterManager::from_csv(exporter);
    assert_eq!(manager.name(), "CSV");
}

#[test]
fn test_dry_run_constructor() {
    let manager = ExporterManager::dry_run();
    assert_eq!(manager.name(), "dry-run");
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
            path: "sqllogs".to_string(),
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
            path: "sqllogs".to_string(),
        },
        ..Default::default()
    };
    let result = ExporterManager::from_config(&cfg);
    assert!(result.is_err());
}

#[test]
fn test_log_stats_with_flush_operations() {
    let mut stats = ExportStats::new();
    stats.exported = 10;
    stats.flush_operations = 2;
    stats.last_flush_size = 5;
    // Use ExporterKind::DryRun to test stats_snapshot
    let e = ExporterKind::DryRun { stats };
    let snap = e.stats_snapshot().unwrap();
    assert_eq!(snap.flush_operations, 2);
    assert_eq!(snap.last_flush_size, 5);
}

#[test]
fn test_exporter_manager_log_stats_no_panic() {
    let manager = ExporterManager::dry_run();
    // Just verify it doesn't panic
    manager.log_stats();
}

#[test]
fn test_exporter_manager_debug_format() {
    let manager = ExporterManager::dry_run();
    let s = format!("{manager:?}");
    assert!(s.contains("ExporterManager"));
}

#[test]
fn test_dry_run_export_via_trait() {
    use dm_database_parser_sqllog::LogParser;
    let dir = tempfile::TempDir::new().unwrap();
    let log = dir.path().join("t.log");
    std::fs::write(&log, "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT 1. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n").unwrap();
    let parser = LogParser::from_path(log.to_str().unwrap()).unwrap();
    let records: Vec<_> = parser.iter().flatten().collect();

    let mut e = ExporterKind::DryRun {
        stats: ExportStats::default(),
    };
    e.initialize().unwrap();
    for r in &records {
        let meta = r.parse_meta();
        let pm = r.parse_performance_metrics();
        e.export_one_preparsed(r, &meta, &pm, None).unwrap();
    }
    // ExporterKind has no finalize — use internal finalize
    let snap = e.stats_snapshot().unwrap();
    assert_eq!(snap.exported, records.len());
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
