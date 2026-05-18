use super::*;
use crate::error::Result;
use dm_database_parser_sqllog::Sqllog;

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

// ── write_template_stats ───────────────────────────────────

/// 辅助：构造一个最小 `TemplateStats` 实例
fn make_template_stats(key: &str) -> crate::pipeline::TemplateStats {
    crate::pipeline::TemplateStats {
        template_key: key.to_string(),
        count: 1,
        avg_us: 100,
        min_us: 10,
        max_us: 200,
        p50_us: 90,
        p95_us: 180,
        p99_us: 195,
        first_seen: "2025-01-01 00:00:00".to_string(),
        last_seen: "2025-01-01 01:00:00".to_string(),
    }
}

/// Test 1: 自定义 mock exporter 不覆盖 `write_template_stats`，默认 no-op 返回 `Ok(())`
#[test]
fn test_default_write_template_stats_noop() {
    #[derive(Debug, Default)]
    struct MockExporter;

    impl Exporter for MockExporter {
        fn initialize(&mut self) -> Result<()> {
            Ok(())
        }
        fn export(&mut self, _: &Sqllog<'_>) -> Result<()> {
            Ok(())
        }
        fn finalize(&mut self) -> Result<()> {
            Ok(())
        }
        // write_template_stats 未覆盖 → 使用 trait 默认 no-op
    }

    let mut mock = MockExporter;
    let stats = vec![make_template_stats("SELECT ?")];
    let result = mock.write_template_stats(&stats, None, None);
    assert!(result.is_ok());
}

/// Test 2: `ExporterKind::DryRun` no-op，不创建任何文件
#[test]
fn test_dry_run_write_template_stats_noop() {
    let mut e = ExporterKind::DryRun {
        stats: ExportStats::default(),
    };
    e.initialize().unwrap();
    let before = e.stats_snapshot().unwrap().exported;

    let stats = vec![
        make_template_stats("SELECT ?"),
        make_template_stats("INSERT ?"),
    ];
    let result = e.write_template_stats(&stats, None, None);
    assert!(result.is_ok());

    // write_template_stats 不影响 exported 计数
    let after = e.stats_snapshot().unwrap().exported;
    assert_eq!(before, after);
}

/// Test 3: `ExporterManager::dry_run()` 委托调用 `write_template_stats` 返回 `Ok(())`
#[test]
fn test_exporter_manager_write_template_stats_dry_run() {
    let mut manager = ExporterManager::dry_run();
    let stats = vec![make_template_stats("SELECT ?")];
    let result = manager.write_template_stats(&stats, None, None);
    assert!(result.is_ok());
}

/// Test 4: `ExporterKind` 三个 variant 透传 `write_template_stats` 均不 panic
#[test]
fn test_exporter_kind_dispatch_write_template_stats() {
    let stats: Vec<crate::pipeline::TemplateStats> = vec![];

    // DryRun variant
    let mut dry_run = ExporterKind::DryRun {
        stats: ExportStats::default(),
    };
    assert!(dry_run.write_template_stats(&stats, None, None).is_ok());

    // CSV variant — 空 stats，路径为 None → 跳过写入
    let dir = tempfile::TempDir::new().unwrap();
    let csv_path = dir.path().join("test_dispatch.csv");
    let mut csv = CsvExporter::new(&csv_path);
    csv.initialize().unwrap();
    csv.finalize().unwrap();
    let mut csv_kind = ExporterKind::Csv(csv);
    assert!(csv_kind.write_template_stats(&stats, None, None).is_ok());

    // SQLite variant — 需要先 initialize 建立数据库连接
    use crate::config::SqliteExporterConfig as SqliteExporterCfg;
    let db_path = dir.path().join("test_dispatch.db");
    let sqlite_cfg = SqliteExporterCfg {
        database_url: db_path.to_string_lossy().into(),
        table_name: "records".to_string(),
        overwrite: true,
        append: false,
        batch_size: 10_000,
    };
    let mut sqlite = SqliteExporter::from_config(&sqlite_cfg);
    sqlite.initialize().unwrap();
    // finalize() commits the main transaction so write_template_stats can open its own
    sqlite.finalize().unwrap();
    let mut sqlite_kind = ExporterKind::Sqlite(sqlite);
    let result = sqlite_kind.write_template_stats(&stats, None, None);
    assert!(
        result.is_ok(),
        "sqlite write_template_stats failed: {result:?}"
    );
}
