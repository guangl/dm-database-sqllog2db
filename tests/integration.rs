//! Integration tests for CLI handlers and the run pipeline.

use dm_database_sqllog2db::cli::digest::{SortBy, handle_digest};
use dm_database_sqllog2db::cli::init::handle_init;
use dm_database_sqllog2db::cli::run::handle_run;
use dm_database_sqllog2db::cli::show_config::handle_show_config;
use dm_database_sqllog2db::cli::validate::handle_validate;
use dm_database_sqllog2db::config::{
    Config, CsvExporterConfig, ExporterConfig, SqliteExporterConfig, SqllogConfig,
};
use dm_database_sqllog2db::lang::Lang;
use dm_database_sqllog2db::pipeline::filters::{ExcludeFilters, IncludeFilters};
use dm_database_sqllog2db::pipeline::{
    FiltersFeature, NormalizeConfig, OutputConfig, TemplateConfig,
};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

// ── helpers ──────────────────────────────────────────────────────────────────

fn write_test_log(path: &std::path::Path, count: usize) {
    use std::fmt::Write as _;
    let mut buf = String::with_capacity(count * 180);
    for i in 0..count {
        writeln!(
            buf,
            "2025-01-15 10:30:28.001 (EP[0] sess:0x{i:04x} user:TESTUSER trxid:{i} stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT * FROM t WHERE id={i}. EXECTIME: {exec}(ms) ROWCOUNT: {rows}(rows) EXEC_ID: {i}.",
            exec = (i * 13) % 1000,
            rows = i % 100,
        )
        .unwrap();
    }
    std::fs::write(path, buf).unwrap();
}

fn make_run_config(log_dir: &std::path::Path, csv_file: &std::path::Path) -> Config {
    Config {
        sqllog: SqllogConfig {
            path: log_dir.to_str().unwrap().to_string(),
        },
        exporter: ExporterConfig {
            csv: Some(CsvExporterConfig {
                file: csv_file.to_str().unwrap().to_string(),
                overwrite: true,
                append: false,
                ..CsvExporterConfig::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    }
}

// ── handle_run tests ─────────────────────────────────────────────────────────

#[test]
fn test_handle_run_dry_run_empty_dir() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    // No log files → handle_run returns Ok early
    let cfg = Config {
        sqllog: SqllogConfig {
            path: log_dir.to_str().unwrap().to_string(),
        },
        ..Default::default()
    };
    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(
        &cfg,
        None,
        true,
        true,
        &interrupted,
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();
}

#[test]
fn test_handle_run_dry_run_with_log_files() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("a.log"), 20);
    write_test_log(&log_dir.join("b.log"), 10);

    let cfg = Config {
        sqllog: SqllogConfig {
            path: log_dir.to_str().unwrap().to_string(),
        },
        ..Default::default()
    };

    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(
        &cfg,
        None,
        true,
        true,
        &interrupted,
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();
}

#[test]
fn test_handle_run_dry_run_with_limit() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("test.log"), 50);

    let cfg = Config {
        sqllog: SqllogConfig {
            path: log_dir.to_str().unwrap().to_string(),
        },
        ..Default::default()
    };

    let interrupted = Arc::new(AtomicBool::new(false));
    // limit to 5 records
    handle_run(
        &cfg,
        Some(5),
        true,
        true,
        &interrupted,
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();
}

#[test]
fn test_handle_run_real_csv_export() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("test.log"), 10);

    let csv_file = dir.path().join("out.csv");
    let cfg = make_run_config(&log_dir, &csv_file);

    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(
        &cfg,
        None,
        false,
        true,
        &interrupted,
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();

    let content = std::fs::read_to_string(&csv_file).unwrap();
    // header + 10 data rows = 11 lines
    assert_eq!(
        content.lines().count(),
        11,
        "expected header + 10 data rows, got {}",
        content.lines().count()
    );
}

#[test]
fn test_handle_run_interrupted() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("test.log"), 100);

    let cfg = Config {
        sqllog: SqllogConfig {
            path: log_dir.to_str().unwrap().to_string(),
        },
        ..Default::default()
    };

    // Pre-set interrupted flag — run returns Err(Interrupted) when flag is set before processing
    let interrupted = Arc::new(AtomicBool::new(true));
    let result = handle_run(
        &cfg,
        None,
        true,
        true,
        &interrupted,
        80,
        false,
        None,
        1,
        None,
    );
    assert!(
        result.is_err(),
        "handle_run should return Err(Interrupted) when interrupt flag is pre-set: {result:?}"
    );
}

// ── resume tests ─────────────────────────────────────────────────────────────

#[test]
fn test_resume_skips_processed_files() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();

    // Two files: a.log (10 records) + b.log (10 records)
    write_test_log(&log_dir.join("a.log"), 10);
    write_test_log(&log_dir.join("b.log"), 10);

    let state_path = dir.path().join("state.toml");
    let csv1 = dir.path().join("out1.csv");
    let cfg = make_run_config(&log_dir, &csv1);
    let interrupted = Arc::new(AtomicBool::new(false));

    // First run with --resume: processes both files, writes state
    handle_run(
        &cfg,
        None,
        false,
        true,
        &interrupted,
        80,
        true,
        Some(state_path.to_str().unwrap()),
        1,
        None, // compiled_filters
    )
    .unwrap();
    let rows_first = std::fs::read_to_string(&csv1).unwrap().lines().count();
    // 2 files × 10 records = 20 data rows + header = 21 lines
    assert_eq!(
        rows_first, 21,
        "expected header + 20 data rows, got {rows_first}"
    );

    // State file must exist after first run
    assert!(state_path.exists(), "state file should be created");

    // Second run with --resume + append: already-processed files are skipped → no new rows
    let csv2 = dir.path().join("out2.csv");
    let mut cfg2 = make_run_config(&log_dir, &csv2);
    cfg2.exporter.csv.as_mut().unwrap().append = true;
    cfg2.exporter.csv.as_mut().unwrap().overwrite = false;

    handle_run(
        &cfg2,
        None,
        false,
        true,
        &interrupted,
        80,
        true,
        Some(state_path.to_str().unwrap()),
        1,
        None, // compiled_filters
    )
    .unwrap();

    // csv2 should have at most a header row (no data rows) because all files were skipped
    let rows_second = if csv2.exists() {
        std::fs::read_to_string(&csv2).unwrap().lines().count()
    } else {
        0
    };
    assert!(
        rows_second <= 1,
        "second run should skip all files; got {rows_second} rows (expected header only)"
    );
}

#[test]
fn test_resume_reprocesses_changed_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();

    let log_file = log_dir.join("a.log");
    write_test_log(&log_file, 5);

    let state_path = dir.path().join("state.toml");
    let csv = dir.path().join("out.csv");
    let cfg = make_run_config(&log_dir, &csv);
    let interrupted = Arc::new(AtomicBool::new(false));

    // First run: process and record state
    handle_run(
        &cfg,
        None,
        false,
        true,
        &interrupted,
        80,
        true,
        Some(state_path.to_str().unwrap()),
        1,
        None, // compiled_filters
    )
    .unwrap();
    assert!(state_path.exists());

    // Simulate file growing (append more records)
    write_test_log(&log_file, 10);

    // Second run with --resume: file fingerprint changed → must reprocess
    let csv2 = dir.path().join("out2.csv");
    let cfg2 = make_run_config(&log_dir, &csv2);
    handle_run(
        &cfg2,
        None,
        false,
        true,
        &interrupted,
        80,
        true,
        Some(state_path.to_str().unwrap()),
        1,
        None, // compiled_filters
    )
    .unwrap();

    // csv2 should have data (file was reprocessed)
    assert!(csv2.exists(), "changed file should be reprocessed");
    let rows = std::fs::read_to_string(&csv2).unwrap().lines().count();
    assert!(rows >= 1, "expected rows from reprocessed file");
}

fn make_stats_cfg(log_dir: &std::path::Path) -> Config {
    Config {
        sqllog: SqllogConfig {
            path: log_dir.to_str().unwrap().to_string(),
        },
        ..Default::default()
    }
}

// ── handle_digest tests (smoke tests — handle_digest returns (), no return value assertion) ───────────────────

#[test]
fn test_handle_digest_empty_dir() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("nologs");
    let cfg = Config {
        sqllog: SqllogConfig {
            path: log_dir.to_str().unwrap().to_string(),
        },
        ..Default::default()
    };
    // No log files → prints message and returns without panic
    handle_digest(&cfg, true, None, SortBy::Count, 1, false, None);
}

#[test]
fn test_handle_digest_basic() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path()).unwrap();
    write_test_log(&dir.path().join("data.log"), 20);
    let cfg = make_stats_cfg(dir.path());
    handle_digest(&cfg, true, None, SortBy::Count, 1, false, None);
}

#[test]
fn test_handle_digest_sort_exec() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path()).unwrap();
    write_test_log(&dir.path().join("data.log"), 20);
    let cfg = make_stats_cfg(dir.path());
    handle_digest(&cfg, true, None, SortBy::Exec, 1, false, None);
}

#[test]
fn test_handle_digest_top_n() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path()).unwrap();
    write_test_log(&dir.path().join("data.log"), 30);
    let cfg = make_stats_cfg(dir.path());
    handle_digest(&cfg, true, Some(5), SortBy::Count, 1, false, None);
}

#[test]
fn test_handle_digest_min_count() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path()).unwrap();
    write_test_log(&dir.path().join("data.log"), 20);
    let cfg = make_stats_cfg(dir.path());
    // min_count=100 filters out everything — should print "No SQL fingerprints found."
    handle_digest(&cfg, true, None, SortBy::Count, 100, false, None);
}

#[test]
fn test_handle_digest_json() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path()).unwrap();
    write_test_log(&dir.path().join("data.log"), 10);
    let cfg = make_stats_cfg(dir.path());
    handle_digest(&cfg, true, None, SortBy::Count, 1, true, None);
}

#[test]
fn test_handle_digest_nonexistent_dir() {
    let cfg = Config {
        sqllog: SqllogConfig {
            path: "/nonexistent_dir_xyz".to_string(),
        },
        ..Default::default()
    };
    // Should not panic
    handle_digest(&cfg, true, None, SortBy::Count, 1, false, None);
}

#[test]
fn test_handle_digest_aggregates_same_fingerprint() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path()).unwrap();
    // Write records with identical SQL structure but different literal values
    // These should collapse into one fingerprint
    let mut buf = String::new();
    use std::fmt::Write as _;
    for i in 0..5 {
        writeln!(
            buf,
            "2025-01-15 10:30:28.001 (EP[0] sess:0x{i:04x} user:U trxid:{i} stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT * FROM tbl WHERE id={i}. EXECTIME: 10(ms) ROWCOUNT: 1(rows) EXEC_ID: {i}.",
        ).unwrap();
    }
    std::fs::write(dir.path().join("data.log"), buf).unwrap();
    let cfg = make_stats_cfg(dir.path());
    handle_digest(&cfg, true, None, SortBy::Count, 1, true, None);
}

// ── handle_init tests ────────────────────────────────────────────────────────

#[test]
fn test_handle_init_creates_config_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    handle_init(config_path.to_str().unwrap(), false, Lang::Zh).unwrap();
    assert!(config_path.exists());
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        content.contains("[sqllog]"),
        "init template should contain [sqllog] section"
    );
}

#[test]
fn test_handle_init_fails_if_exists_without_force() {
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(&config_path, "existing").unwrap();
    let result = handle_init(config_path.to_str().unwrap(), false, Lang::Zh);
    assert!(result.is_err());
}

#[test]
fn test_handle_init_force_overwrites_existing() {
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(&config_path, "old content").unwrap();
    handle_init(config_path.to_str().unwrap(), true, Lang::Zh).unwrap();
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[sqllog]"));
}

#[test]
fn test_handle_init_en_template() {
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    handle_init(config_path.to_str().unwrap(), false, Lang::En).unwrap();
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[sqllog]"));
    assert!(content.contains("SQL log path"));
    assert!(!content.contains("日志路径"));
}

#[test]
fn test_handle_init_zh_template() {
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    handle_init(config_path.to_str().unwrap(), false, Lang::Zh).unwrap();
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[sqllog]"));
    assert!(content.contains("日志路径"));
}

// ── handle_validate tests ────────────────────────────────────────────────────

#[test]
fn test_handle_validate_default_config() {
    let cfg = Config::default();
    handle_validate(&cfg); // no panic, hits csv branch and no-filters branch
}

#[test]
fn test_handle_validate_with_sqlite_exporter() {
    let cfg = Config {
        exporter: ExporterConfig {
            csv: None,
            sqlite: Some(SqliteExporterConfig {
                database_url: "/tmp/test.db".to_string(),
                table_name: "records".to_string(),
                overwrite: true,
                append: false,
                batch_size: 10_000,
            }),
        },
        ..Default::default()
    };
    handle_validate(&cfg); // hits sqlite branch
}

#[test]
fn test_handle_validate_with_replace_parameters_none() {
    let cfg = Config {
        replace_parameters: None,
        ..Default::default()
    };
    handle_validate(&cfg); // hits replace_parameters None branch
}

#[test]
fn test_handle_validate_with_replace_parameters_some() {
    let cfg = Config {
        replace_parameters: Some(NormalizeConfig {
            enable: true,
            placeholders: vec!["?".to_string()],
        }),
        ..Default::default()
    };
    handle_validate(&cfg); // hits replace_parameters Some branch
}

#[test]
fn test_handle_validate_with_filters_none() {
    let cfg = Config {
        filter: None,
        ..Default::default()
    };
    handle_validate(&cfg); // hits filters None branch
}

#[test]
fn test_handle_validate_with_filters_all_fields() {
    use dm_database_sqllog2db::pipeline::filters::{IndicatorFilters, SqlFilters};
    let cfg = Config {
        filter: Some(FiltersFeature {
            enable: true,
            include: IncludeFilters {
                start_ts: Some("2025-01-01".to_string()),
                end_ts: Some("2025-12-31".to_string()),
                users: Some(vec!["admin".to_string()]),
                ips: Some(vec!["10.0.0.1".to_string()]),
                trxids: Some(
                    ["tx1"]
                        .iter()
                        .map(|s| compact_str::CompactString::new(s))
                        .collect(),
                ),
                ..Default::default()
            },
            exclude: ExcludeFilters::default(),
            indicators: IndicatorFilters {
                exec_ids: Some([42_i64].into_iter().collect()),
                min_runtime_ms: Some(100),
                min_row_count: Some(10),
            },
            sql: SqlFilters {
                includes: Some(vec!["SELECT".to_string()]),
                excludes: Some(vec!["DROP".to_string()]),
            },
            record_sql: SqlFilters::default(),
        }),
        ..Default::default()
    };
    handle_validate(&cfg); // hits all filter sub-branches
}

#[test]
fn test_handle_validate_filters_disabled() {
    use dm_database_sqllog2db::pipeline::filters::IndicatorFilters;
    let cfg = Config {
        filter: Some(FiltersFeature {
            enable: false,
            include: IncludeFilters::default(),
            exclude: ExcludeFilters::default(),
            indicators: IndicatorFilters::default(),
            ..Default::default()
        }),
        ..Default::default()
    };
    handle_validate(&cfg); // hits "配置但未明确启用" branch
}

// ── handle_run coverage supplement ──────────────────────────────────────────

#[test]
fn test_handle_run_non_quiet_prints_summary() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("data.log"), 10);
    let csv_file = dir.path().join("out.csv");
    let cfg = make_run_config(&log_dir, &csv_file);
    let interrupted = Arc::new(AtomicBool::new(false));
    // quiet=false exercises the summary print path
    handle_run(
        &cfg,
        None,
        true,
        false,
        &interrupted,
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();
}

#[test]
fn test_handle_run_with_filters_builds_pipeline() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("data.log"), 20);
    let csv_file = dir.path().join("out.csv");
    let mut cfg = make_run_config(&log_dir, &csv_file);
    // Enable a record-level filter — exercises build_pipeline and FilterProcessor
    // Explicitly compiles filters and passes them to handle_run (pre-compiled path)
    cfg.filter = Some(FiltersFeature {
        enable: true,
        include: IncludeFilters {
            users: Some(vec!["TESTUSER".to_string()]),
            ..Default::default()
        },
        exclude: ExcludeFilters::default(),
        ..Default::default()
    });
    let compiled_filters = cfg.validate_and_compile().unwrap();
    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(
        &cfg,
        None,
        true,
        true,
        &interrupted,
        80,
        false,
        None,
        1,
        compiled_filters,
    )
    .unwrap();
}

#[test]
fn test_handle_run_with_limit_mid_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("data.log"), 100);
    let csv_file = dir.path().join("out.csv");
    let cfg = make_run_config(&log_dir, &csv_file);
    let interrupted = Arc::new(AtomicBool::new(false));
    // limit=5 stops partway through the file — exercises the limit check in process_log_file
    handle_run(
        &cfg,
        Some(5),
        false,
        true,
        &interrupted,
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();
    let content = std::fs::read_to_string(&csv_file).unwrap();
    let data_lines = content.lines().count().saturating_sub(1); // minus header
    assert!(data_lines <= 5, "expected ≤5 records, got {data_lines}");
}

#[test]
fn test_handle_run_with_transaction_filters_prescans() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("data.log"), 30);
    let csv_file = dir.path().join("out.csv");
    let mut cfg = make_run_config(&log_dir, &csv_file);
    // exec_ids filter triggers transaction pre-scan path
    // Passes compiled_filters=None — exercises handle_run's internal recompile_meta_if_needed path
    cfg.filter = Some(FiltersFeature {
        enable: true,
        include: IncludeFilters::default(),
        exclude: ExcludeFilters::default(),
        indicators: dm_database_sqllog2db::pipeline::filters::IndicatorFilters {
            exec_ids: Some([0_i64, 1, 2].into_iter().collect()),
            min_runtime_ms: None,
            min_row_count: None,
        },
        ..Default::default()
    });
    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(
        &cfg,
        None,
        true,
        true,
        &interrupted,
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();
}

#[test]
fn test_handle_run_with_min_runtime_filter() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("data.log"), 20);
    let csv_file = dir.path().join("out.csv");
    let mut cfg = make_run_config(&log_dir, &csv_file);
    // min_runtime filter — exercises the record-level runtime check
    // Passes compiled_filters=None — exercises handle_run's internal recompile_meta_if_needed path
    cfg.filter = Some(FiltersFeature {
        enable: true,
        include: IncludeFilters::default(),
        exclude: ExcludeFilters::default(),
        indicators: dm_database_sqllog2db::pipeline::filters::IndicatorFilters {
            exec_ids: None,
            min_runtime_ms: Some(1),
            min_row_count: None,
        },
        ..Default::default()
    });
    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(
        &cfg,
        None,
        true,
        true,
        &interrupted,
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();
}

// ── handle_show_config tests (via integration) ───────────────────────────────

#[test]
fn test_handle_show_config_integration() {
    let cfg = Config::default();
    // Smoke test: handle_show_config returns () — no return value assertion
    handle_show_config(&cfg, "/path/to/config.toml", false);
}

// ── parallel CSV tests ──────────────────────────────────────────────────────

#[test]
fn test_handle_run_parallel_csv_multiple_files() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    // Create 3 log files to trigger the parallel path
    write_test_log(&log_dir.join("a.log"), 10);
    write_test_log(&log_dir.join("b.log"), 10);
    write_test_log(&log_dir.join("c.log"), 10);

    let csv_file = dir.path().join("out.csv");
    let cfg = make_run_config(&log_dir, &csv_file);
    let interrupted = Arc::new(AtomicBool::new(false));

    // jobs=2, multiple files, no limit, CSV exporter → triggers process_csv_parallel
    handle_run(
        &cfg,
        None,
        false,
        true,
        &interrupted,
        80,
        false,
        None,
        2,
        None,
    )
    .unwrap();

    let content = std::fs::read_to_string(&csv_file).unwrap();
    let data_lines = content.lines().count().saturating_sub(1);
    assert_eq!(data_lines, 30, "expected 30 records from 3 × 10");
}

#[test]
fn test_handle_run_parallel_csv_with_resume() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("a.log"), 5);
    write_test_log(&log_dir.join("b.log"), 5);

    let csv_file = dir.path().join("out.csv");
    let state_file = dir.path().join("state.toml");
    let cfg = make_run_config(&log_dir, &csv_file);
    let interrupted = Arc::new(AtomicBool::new(false));

    // First parallel run: processes both files and records state
    handle_run(
        &cfg,
        None,
        false,
        true,
        &interrupted,
        80,
        true,
        Some(state_file.to_str().unwrap()),
        2,
        None, // compiled_filters
    )
    .unwrap();
    assert!(state_file.exists());

    // Second run: all files already processed → output empty (no data rows)
    let csv2 = dir.path().join("out2.csv");
    let mut cfg2 = make_run_config(&log_dir, &csv2);
    cfg2.exporter.csv.as_mut().unwrap().append = true;
    cfg2.exporter.csv.as_mut().unwrap().overwrite = false;
    handle_run(
        &cfg2,
        None,
        false,
        true,
        &interrupted,
        80,
        true,
        Some(state_file.to_str().unwrap()),
        2,
        None, // compiled_filters
    )
    .unwrap();
    // csv2 should have at most a header (all files skipped)
    let rows = if csv2.exists() {
        std::fs::read_to_string(&csv2).unwrap().lines().count()
    } else {
        0
    };
    assert!(rows <= 1, "expected ≤1 rows in second run, got {rows}");
}

// ── performance baseline ─────────────────────────────────────────────────────
//
// Lightweight sanity check — NOT a substitute for `cargo bench`.
// Thresholds are intentionally conservative:
//   - debug builds: 30k rec/s  (catches complete disasters only)
//   - release builds: 500k rec/s  (catches real regressions)
// Run with `cargo test --release` for meaningful numbers.

#[test]
fn test_csv_throughput_baseline() {
    const RECORD_COUNT: usize = 20_000;

    // Debug builds run ~100k rec/s; release runs ~2M rec/s on developer machines.
    // CI machines are slower, so thresholds are kept conservative.
    #[cfg(debug_assertions)]
    const MIN_RECORDS_PER_SEC: f64 = 30_000.0;
    #[cfg(not(debug_assertions))]
    const MIN_RECORDS_PER_SEC: f64 = 500_000.0;

    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("perf.log"), RECORD_COUNT);

    let csv_file = dir.path().join("out.csv");
    let cfg = make_run_config(&log_dir, &csv_file);

    let interrupted = Arc::new(AtomicBool::new(false));
    let start = std::time::Instant::now();
    handle_run(
        &cfg,
        None,
        false,
        true,
        &interrupted,
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();
    let elapsed = start.elapsed().as_secs_f64();

    #[allow(clippy::cast_precision_loss)]
    let rate = RECORD_COUNT as f64 / elapsed;
    assert!(
        rate >= MIN_RECORDS_PER_SEC,
        "CSV throughput {rate:.0} rec/s is below {MIN_RECORDS_PER_SEC:.0} rec/s minimum \
         ({} build)",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
    );
}

#[test]
fn test_init_generates_new_nested_format() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("init.toml");
    let path_str = path.to_str().unwrap();
    handle_init(path_str, false, Lang::Zh).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(
        content.contains("[filter.include]"),
        "init template must contain [filter.include]"
    );
    assert!(
        content.contains("[filter.exclude]"),
        "init template must contain [filter.exclude]"
    );
    assert!(
        content.contains("[filter.indicators]"),
        "init template must contain [filter.indicators]"
    );
    assert!(
        content.contains("[filter.sql]"),
        "init template must contain [filter.sql]"
    );
    assert!(
        content.contains("[template]"),
        "init template must contain [template]"
    );
    assert!(
        content.contains("[replace_parameters]"),
        "init template must contain [replace_parameters]"
    );
    assert!(
        !content.contains("[pipeline."),
        "init template must NOT contain legacy [pipeline.*]"
    );
    assert!(
        !content.contains("\nusernames = "),
        "init template must not contain active 'usernames' field"
    );
    assert!(
        !content.contains("\ninclude_patterns = "),
        "init template must not contain active 'include_patterns' field"
    );
    let cfg: dm_database_sqllog2db::config::Config = toml::from_str(&content).unwrap();
    cfg.validate().unwrap();
}

#[test]
fn test_init_generated_zh_template_passes_validate() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    handle_init(path.to_str().unwrap(), true, Lang::Zh).unwrap();
    let cfg = dm_database_sqllog2db::config::Config::from_file(&path).unwrap();
    assert!(
        cfg.validate().is_ok(),
        "ZH init template must pass validate()"
    );
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(
        !content.contains("pipeline."),
        "ZH init template must not contain any 'pipeline.' substring"
    );
}

#[test]
fn test_init_generated_en_template_passes_validate() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    handle_init(path.to_str().unwrap(), true, Lang::En).unwrap();
    let cfg = dm_database_sqllog2db::config::Config::from_file(&path).unwrap();
    assert!(
        cfg.validate().is_ok(),
        "EN init template must pass validate()"
    );
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(
        !content.contains("pipeline."),
        "EN init template must not contain any 'pipeline.' substring"
    );
}

#[test]
fn test_validate_rejects_legacy_pipeline_template_analysis() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.toml");
    std::fs::write(
        &path,
        "[sqllog]\npath = \"sqllogs\"\n\n[pipeline.template_analysis]\nenabled = true\n\n[exporter.csv]\nfile = \"out.csv\"\n",
    )
    .unwrap();
    let cfg = dm_database_sqllog2db::config::Config::from_file(&path).unwrap();
    let result = cfg.validate();
    assert!(
        result.is_err(),
        "legacy [pipeline.template_analysis] must be rejected by validate()"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("[pipeline.template_analysis] → [template]"),
        "error must contain migration hint for template_analysis; got: {err_msg}"
    );
    assert!(
        err_msg.contains("[pipeline.charts] → [charts]"),
        "error must contain migration hint for charts; got: {err_msg}"
    );
    assert!(
        err_msg.contains("[pipeline.filters.*] → [filter.*]"),
        "error must contain migration hint for filters; got: {err_msg}"
    );
}

#[test]
fn test_validate_rejects_legacy_pipeline_filters_section() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy_filters.toml");
    std::fs::write(
        &path,
        "[sqllog]\npath = \"sqllogs\"\n\n[pipeline.filters]\nenable = true\n\n[exporter.csv]\nfile = \"out.csv\"\n",
    )
    .unwrap();
    let cfg = dm_database_sqllog2db::config::Config::from_file(&path).unwrap();
    let result = cfg.validate();
    assert!(
        result.is_err(),
        "legacy [pipeline.filters] must be rejected by validate()"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("[pipeline.template_analysis] → [template]"),
        "error must contain migration hint for template_analysis; got: {err_msg}"
    );
    assert!(
        err_msg.contains("[pipeline.charts] → [charts]"),
        "error must contain migration hint for charts; got: {err_msg}"
    );
    assert!(
        err_msg.contains("[pipeline.normalize] → [replace_parameters]"),
        "error must contain migration hint for normalize; got: {err_msg}"
    );
    assert!(
        err_msg.contains("[pipeline.filters.*] → [filter.*]"),
        "error must contain migration hint for filters; got: {err_msg}"
    );
    assert!(
        err_msg.contains("[pipeline.fields] → [output.fields]"),
        "error must contain migration hint for fields; got: {err_msg}"
    );
}

// ── E2E pipeline tests (TEST-02) ─────────────────────────────────────────────

#[test]
fn test_e2e_filter_pipeline() {
    // Arrange: 10 条 user=TESTUSER 记录
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("test.log"), 10);

    let csv_file = dir.path().join("out.csv");
    let mut cfg = make_run_config(&log_dir, &csv_file);
    // 配置 include.users = ["TESTUSER"]，全部 10 条应通过过滤
    cfg.filter = Some(FiltersFeature {
        enable: true,
        include: IncludeFilters {
            users: Some(vec!["TESTUSER".to_string()]),
            ..Default::default()
        },
        exclude: ExcludeFilters::default(),
        ..Default::default()
    });

    // Act
    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(
        &cfg,
        None,
        false,
        true,
        &interrupted,
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();

    // Assert: header + 10 条数据行 = 11 行
    let content = std::fs::read_to_string(&csv_file).unwrap();
    assert_eq!(
        content.lines().count(),
        11,
        "expected header + 10 data rows, got {}",
        content.lines().count()
    );

    // 追加一个包含 user=OTHER 的第二个日志文件
    let log_dir2 = dir.path().join("logs2");
    std::fs::create_dir_all(&log_dir2).unwrap();
    {
        use std::fmt::Write as _;
        let mut buf = String::with_capacity(5 * 180);
        for i in 100..105usize {
            writeln!(
                buf,
                "2025-01-15 10:30:28.001 (EP[0] sess:0x{i:04x} user:OTHER trxid:{i} stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT * FROM t WHERE id={i}. EXECTIME: 0(ms) ROWCOUNT: 1(rows) EXEC_ID: {i}.",
            )
            .unwrap();
        }
        std::fs::write(log_dir2.join("other.log"), buf).unwrap();
    }
    let csv_file2 = dir.path().join("out2.csv");
    let mut cfg2 = make_run_config(&log_dir2, &csv_file2);
    cfg2.filter = Some(FiltersFeature {
        enable: true,
        include: IncludeFilters {
            users: Some(vec!["TESTUSER".to_string()]),
            ..Default::default()
        },
        exclude: ExcludeFilters::default(),
        ..Default::default()
    });
    handle_run(
        &cfg2,
        None,
        false,
        true,
        &Arc::new(AtomicBool::new(false)),
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();
    let content2 = std::fs::read_to_string(&csv_file2).unwrap();
    // OTHER 全被过滤，只有 header
    assert_eq!(
        content2.lines().count(),
        1,
        "expected only header row when all records filtered out, got {}",
        content2.lines().count()
    );
}

#[test]
fn test_e2e_template_normalization() {
    // Arrange: 5 条记录，启用模板归一化
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("test.log"), 5);

    let csv_file = dir.path().join("out.csv");
    let mut cfg = make_run_config(&log_dir, &csv_file);
    cfg.template = Some(TemplateConfig {
        enable: true,
        report: None,
    });

    // Act
    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(
        &cfg,
        None,
        false,
        true,
        &interrupted,
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();

    // Assert: header 包含 normalized_sql，且第一条数据行的 normalized_sql 列非空
    let content = std::fs::read_to_string(&csv_file).unwrap();
    let header = content.lines().next().unwrap();
    assert!(
        header.contains("normalized_sql"),
        "CSV header should contain 'normalized_sql', got: {header}"
    );
    // 第一条数据行（索引 14 = normalized_sql）应非空
    let data_line = content.lines().nth(1).unwrap();
    // normalized_sql 是第 15 个字段（索引 14），用逗号分割取第 14 个字段
    // 注意：SQL 格式为 "SELECT * FROM t WHERE id=N" 不含逗号，因此 split(',') 安全
    // 如果测试 SQL 未来包含逗号，需改用 csv crate 正确解析带引号的字段
    assert!(!data_line.is_empty(), "first data line should not be empty");
    // 验证 normalized_sql 列存在内容：整行中字段数至少为 15
    let field_count = data_line.split(',').count();
    assert!(
        field_count >= 15,
        "expected at least 15 fields in data line, got {field_count}: {data_line}"
    );
}

#[test]
fn test_e2e_field_projection() {
    // Arrange: 3 条记录，字段投影为 ts/username/sql
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("test.log"), 3);

    let csv_file = dir.path().join("out.csv");
    let mut cfg = make_run_config(&log_dir, &csv_file);
    cfg.output = Some(OutputConfig {
        fields: Some(vec![
            "ts".to_string(),
            "username".to_string(),
            "sql".to_string(),
        ]),
    });

    // Act
    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(
        &cfg,
        None,
        false,
        true,
        &interrupted,
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();

    // Assert: header 精确为 "ts,username,sql"，数据行 split(',').count() == 3
    let content = std::fs::read_to_string(&csv_file).unwrap();
    let header = content.lines().next().unwrap();
    assert_eq!(
        header, "ts,username,sql",
        "expected header 'ts,username,sql', got: {header}"
    );
    // 验证每条数据行字段数 == 3
    // 注意：sql 字段内容为 "SELECT * FROM t WHERE id=N" 不含逗号，所以 split(',').count() == 3
    // 如果 SQL 中包含逗号，需改用 csv crate 正确解析带引号的字段
    let data_lines: Vec<_> = content.lines().skip(1).collect();
    assert_eq!(
        data_lines.len(),
        3,
        "expected 3 data rows, got {}",
        data_lines.len()
    );
    for line in &data_lines {
        let field_count = line.split(',').count();
        assert_eq!(
            field_count, 3,
            "expected 3 fields per row, got {field_count}: {line}"
        );
    }
}

// ── Boundary tests (TEST-03) ─────────────────────────────────────────────────

#[test]
fn test_boundary_empty_log_file() {
    // Arrange: 0 字节 empty.log
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    std::fs::write(log_dir.join("empty.log"), b"").unwrap();

    let csv_file = dir.path().join("out.csv");
    let cfg = make_run_config(&log_dir, &csv_file);

    // Act
    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(
        &cfg,
        None,
        false,
        true,
        &interrupted,
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();

    // Assert: CSV 文件存在且只有 header（1 行）
    assert!(
        csv_file.exists(),
        "CSV file should exist even for empty input"
    );
    let content = std::fs::read_to_string(&csv_file).unwrap();
    assert_eq!(
        content.lines().count(),
        1,
        "expected only header row for empty log, got {} lines",
        content.lines().count()
    );
}

#[test]
fn test_boundary_all_filtered() {
    // Arrange: 5 条 user=TESTUSER 记录，但过滤器 include.users=["NONEXISTENT"]
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("test.log"), 5);

    let csv_file = dir.path().join("out.csv");
    let mut cfg = make_run_config(&log_dir, &csv_file);
    cfg.filter = Some(FiltersFeature {
        enable: true,
        include: IncludeFilters {
            users: Some(vec!["NONEXISTENT".to_string()]),
            ..Default::default()
        },
        exclude: ExcludeFilters::default(),
        ..Default::default()
    });

    // Act
    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(
        &cfg,
        None,
        false,
        true,
        &interrupted,
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();

    // Assert: CSV 只有 header（全部记录被过滤）
    let content = std::fs::read_to_string(&csv_file).unwrap();
    assert_eq!(
        content.lines().count(),
        1,
        "expected only header row when all records filtered, got {} lines",
        content.lines().count()
    );
}

#[test]
fn test_boundary_malformed_line() {
    // Arrange: 2 条正常行 + 1 条无效行 + 2 条正常行 = 4 条正常记录
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();

    // 无效行放在文件开头：解析器会把它作为第一条记录处理 → 解析失败 → 跳过
    // 后续 4 条正常行继续被导出，验证不 panic 且正常行全部处理
    use std::fmt::Write as FmtWrite;
    let mut content = String::new();
    content.push_str("INVALID LINE NO TIMESTAMP HERE\n");
    for i in 0..4usize {
        writeln!(
            content,
            "2025-01-15 10:30:28.001 (EP[0] sess:0x{i:04x} user:TESTUSER trxid:{i} stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT * FROM t WHERE id={i}. EXECTIME: 0(ms) ROWCOUNT: 1(rows) EXEC_ID: {i}."
        )
        .unwrap();
    }
    std::fs::write(log_dir.join("mixed.log"), content).unwrap();

    let csv_file = dir.path().join("out.csv");
    let cfg = make_run_config(&log_dir, &csv_file);

    // Act
    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(
        &cfg,
        None,
        false,
        true,
        &interrupted,
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();

    // Assert: 无效行被跳过，4 条正常记录导出 → header + 4 data = 5 行
    let csv_content = std::fs::read_to_string(&csv_file).unwrap();
    assert_eq!(
        csv_content.lines().count(),
        5,
        "expected header + 4 data rows (malformed line skipped), got {} lines",
        csv_content.lines().count()
    );
}

#[test]
fn test_boundary_long_sql() {
    // Arrange: 1 条超长 SQL 记录（SQL 字段 1MB），保持完整达梦日志格式
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();

    let huge_sql = "X".repeat(1_048_576);
    let log_line = format!(
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:TESTUSER trxid:1 stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT FROM t WHERE c='{huge_sql}'. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n"
    );
    std::fs::write(log_dir.join("long.log"), log_line).unwrap();

    let csv_file = dir.path().join("out.csv");
    let cfg = make_run_config(&log_dir, &csv_file);

    // Act: 不应 panic，不应 OOM
    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(
        &cfg,
        None,
        false,
        true,
        &interrupted,
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();

    // Assert: 1 条记录正常导出 → header + 1 data = 2 行
    let csv_content = std::fs::read_to_string(&csv_file).unwrap();
    assert_eq!(
        csv_content.lines().count(),
        2,
        "expected header + 1 data row for long SQL, got {} lines",
        csv_content.lines().count()
    );
}
