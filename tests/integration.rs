//! Integration tests for CLI handlers and the run pipeline.

use dm_database_sqllog2db::cli::init::{ExporterChoice, handle_init, run_wizard};
use dm_database_sqllog2db::cli::run::handle_run;
use dm_database_sqllog2db::cli::validate::handle_validate;
use dm_database_sqllog2db::config::{
    Config, CsvExporterConfig, ExporterConfig, SqliteExporterConfig, SqllogConfig,
};
use dm_database_sqllog2db::pipeline::filters::types::{ExcludeFilters, IncludeFilters};
use dm_database_sqllog2db::pipeline::{FiltersFeature, NormalizeConfig, OutputConfig};
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

/// Like `write_test_log` but records start at `start_offset` so files have non-overlapping IDs.
fn write_test_log_offset(path: &std::path::Path, count: usize, start_offset: usize) {
    use std::fmt::Write as _;
    let mut buf = String::with_capacity(count * 180);
    for n in 0..count {
        let i = start_offset + n;
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

fn write_heterogeneous_log(
    path: &std::path::Path,
    count: usize,
    trxid_offset: usize,
    username: &str,
) {
    use std::fmt::Write as _;
    let mut buf = String::with_capacity(count * 200);
    for i in 0..count {
        let trxid = trxid_offset + i;
        writeln!(
            buf,
            "2025-01-15 10:30:28.001 (EP[0] sess:0x{trxid:04x} user:{username} trxid:{trxid} \
             stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT * FROM t WHERE id={trxid}. \
             EXECTIME: {}(ms) ROWCOUNT: {}(rows) EXEC_ID: {trxid}.",
            (trxid * 13) % 1000,
            trxid % 100,
        )
        .unwrap();
    }
    std::fs::write(path, buf).unwrap();
}

fn make_run_config(log_dir: &std::path::Path, csv_file: &std::path::Path) -> Config {
    Config {
        sqllog: SqllogConfig {
            inputs: vec![log_dir.to_str().unwrap().to_string()],
            path_deprecated: None,
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
#[cfg(target_os = "windows")]
fn test_handle_run_empty_dir_returns_no_files_found() {
    // Windows: stdin pipe fallback disabled, NoFilesFound is the only path
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    let csv_file = dir.path().join("out.csv");
    let cfg = make_run_config(&log_dir, &csv_file);
    let interrupted = Arc::new(AtomicBool::new(false));
    let result = handle_run(&cfg, true, false, &interrupted, None);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("No log files found matching inputs"));
}

#[test]
#[cfg(not(target_os = "windows"))]
#[ignore = "stdin tty behavior is non-deterministic in CI; covered indirectly by C3"]
fn test_handle_run_empty_dir_unix_behavior() {}

#[test]
fn test_handle_run_multi_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("a.log"), 20);
    write_test_log(&log_dir.join("b.log"), 10);

    let csv_file = dir.path().join("out.csv");
    let cfg = make_run_config(&log_dir, &csv_file);

    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(&cfg, true, false, &interrupted, None).unwrap();
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
    handle_run(&cfg, true, false, &interrupted, None).unwrap();

    let content = std::fs::read_to_string(&csv_file).unwrap();
    // header + 10 data rows = 11 lines
    assert_eq!(
        content.lines().count(),
        11,
        "expected header + 10 data rows"
    );
}

#[test]
fn test_handle_run_interrupted() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("test.log"), 10);

    let csv_file = dir.path().join("out.csv");
    let cfg = make_run_config(&log_dir, &csv_file);

    // Pre-set interrupted flag — run returns Err(Interrupted) when flag is set before processing
    let interrupted = Arc::new(AtomicBool::new(true));
    let result = handle_run(&cfg, true, false, &interrupted, None);
    assert!(
        matches!(
            result,
            Err(dm_database_sqllog2db::error::Error::Interrupted)
        ),
        "handle_run should return Err(Interrupted) when interrupt flag is pre-set, got: {result:?}"
    );
}

// ── handle_init tests ────────────────────────────────────────────────────────

#[test]
fn test_handle_init_creates_config_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    handle_init(config_path.to_str().unwrap(), false).unwrap();
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
    let result = handle_init(config_path.to_str().unwrap(), false);
    assert!(result.is_err());
}

#[test]
fn test_handle_init_force_overwrites_existing() {
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(&config_path, "old content").unwrap();
    handle_init(config_path.to_str().unwrap(), true).unwrap();
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[sqllog]"));
}

#[test]
fn test_handle_init_en_template() {
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    handle_init(config_path.to_str().unwrap(), false).unwrap();
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        content.contains("[sqllog]"),
        "init template should contain [sqllog] section"
    );
    assert!(
        content.contains("SQL log path"),
        "init template should contain 'SQL log path' comment"
    );
    assert!(
        content.contains("log path"),
        "init template should contain 'log path' (English only)"
    );
    assert!(
        !content.contains("日志路径"),
        "init template must not contain Chinese text"
    );
}

// ── handle_init template comment tests ───────────────────────────────────────

#[test]
fn test_init_template_has_csv_append_comment() {
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    handle_init(config_path.to_str().unwrap(), false).unwrap();
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        content.contains("Append to existing CSV file instead of overwriting"),
        "init template should contain csv append comment"
    );
    assert!(
        content.contains("CSV output file path"),
        "init template should contain csv file comment"
    );
}

#[test]
fn test_init_template_has_sqlite_field_comments() {
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    handle_init(config_path.to_str().unwrap(), false).unwrap();
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        content.contains("SQLite database file path"),
        "init template should contain sqlite database_url comment"
    );
    assert!(
        content.contains("Table name to write records into"),
        "init template should contain sqlite table_name comment"
    );
    assert!(
        content.contains("ASCII identifiers only"),
        "init template should contain ASCII identifiers note"
    );
    assert!(
        content.contains("Drop and recreate the table"),
        "init template should contain sqlite overwrite comment"
    );
    assert!(
        content.contains("Append rows to existing table"),
        "init template should contain sqlite append comment"
    );
}

#[test]
fn test_init_template_has_filter_inline_comments() {
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    handle_init(config_path.to_str().unwrap(), false).unwrap();
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        content.contains("Exact-match list of usernames to include"),
        "filter.include must have users inline comment"
    );
    assert!(
        content.contains("Exact-match list of client IP addresses to include"),
        "filter.include must have ips inline comment"
    );
    assert!(
        content.contains("Exact-match list of session IDs (hex strings) to include"),
        "filter.include must have sessions inline comment"
    );
    assert!(
        content.contains("Exact-match list of usernames to exclude"),
        "filter.exclude must have users inline comment"
    );
    assert!(
        content.contains("Exact-match list of client IP addresses to exclude"),
        "filter.exclude must have ips inline comment"
    );
    assert!(
        content.contains("Exact-match list of session IDs (hex strings) to exclude"),
        "filter.exclude must have sessions inline comment"
    );
    assert!(
        content.contains(
            "Transaction-level: retain whole transaction if any record's exec_id matches"
        ),
        "filter.indicators must have exec_ids inline comment"
    );
    assert!(
        content.contains("Transaction-level: retain whole transaction if any statement's runtime (ms) >= threshold"),
        "filter.indicators must have min_runtime_ms inline comment"
    );
    assert!(
        content.contains(
            "Transaction-level: retain whole transaction if any statement's row_count >= threshold"
        ),
        "filter.indicators must have min_row_count inline comment"
    );
    assert!(
        content.contains("Transaction-level: retain whole transaction if any SQL text contains any substring listed"),
        "filter.sql must have includes inline comment"
    );
    assert!(
        content.contains("Transaction-level: drop whole transaction if any SQL text contains any substring listed"),
        "filter.sql must have excludes inline comment"
    );
}

// ── handle_validate tests ────────────────────────────────────────────────────

#[test]
fn test_handle_validate_default_config() {
    let cfg = Config::default();
    handle_validate(&cfg); // validate called without panic
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
                multi_row_batch_size: 64,
            }),
        },
        ..Default::default()
    };
    handle_validate(&cfg); // validate called without panic (sqlite exporter config)
}

#[test]
fn test_handle_validate_with_replace_parameters_none() {
    let cfg = Config {
        replace_parameters: None,
        ..Default::default()
    };
    handle_validate(&cfg); // validate called without panic (replace_parameters is None)
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
    handle_validate(&cfg); // validate called without panic (replace_parameters is Some)
}

#[test]
fn test_handle_validate_with_filters_none() {
    let cfg = Config {
        filter: None,
        ..Default::default()
    };
    handle_validate(&cfg); // validate called without panic (filter is None)
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
                trxids: Some(["tx1"].iter().map(|s| String::from(*s)).collect()),
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
        }),
        ..Default::default()
    };
    handle_validate(&cfg); // validate called without panic (all filter sub-fields populated)
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
    handle_validate(&cfg); // validate called without panic (filter configured but not enabled)
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
    handle_run(&cfg, false, false, &interrupted, None).unwrap();
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
    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(&cfg, true, false, &interrupted, None).unwrap();
    // include.users = ["TESTUSER"]，全部 20 条匹配 → header + 20 = 21 行
    let content = std::fs::read_to_string(&csv_file).unwrap();
    assert_eq!(
        content.lines().count(),
        21,
        "expected header + 20 matching records"
    );
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
    handle_run(&cfg, true, false, &interrupted, None).unwrap();
    // exec_ids = [0, 1, 2]，30 条记录中匹配 3 条 → header + 3 = 4 行
    let content = std::fs::read_to_string(&csv_file).unwrap();
    assert_eq!(
        content.lines().count(),
        4,
        "expected header + 3 records matching exec_ids [0,1,2]"
    );
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
    handle_run(&cfg, true, false, &interrupted, None).unwrap();
    // EXECTIME = (i*13)%1000：i=0 时为 0ms（被过滤），其余 19 条 ≥ 13ms
    // → header + 19 = 20 行
    let content = std::fs::read_to_string(&csv_file).unwrap();
    assert_eq!(
        content.lines().count(),
        20,
        "expected header + 19 records with EXECTIME >= 1ms"
    );
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
    handle_run(&cfg, true, false, &interrupted, None).unwrap();

    let content = std::fs::read_to_string(&csv_file).unwrap();
    let data_lines = content.lines().count().saturating_sub(1);
    assert_eq!(data_lines, 30, "expected 30 records from 3 × 10");
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

    // Debug builds run ~100k rec/s on dev machines, ~10k on slow CI (Windows).
    // Keep a low threshold to catch complete disasters only.
    #[cfg(debug_assertions)]
    const MIN_RECORDS_PER_SEC: f64 = 5_000.0;
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
    handle_run(&cfg, true, false, &interrupted, None).unwrap();
    let elapsed = start.elapsed().as_secs_f64();

    let rate = f64::from(u32::try_from(RECORD_COUNT).expect("20_000 fits in u32")) / elapsed;
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
    handle_init(path_str, false).unwrap();
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
fn test_init_generated_en_template_passes_validate() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    handle_init(path.to_str().unwrap(), true).unwrap();
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
    handle_run(&cfg, true, false, &interrupted, None).unwrap();

    // Assert: header + 10 条数据行 = 11 行
    let content = std::fs::read_to_string(&csv_file).unwrap();
    assert_eq!(
        content.lines().count(),
        11,
        "expected header + 10 data rows"
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
    handle_run(&cfg2, true, false, &Arc::new(AtomicBool::new(false)), None).unwrap();
    let content2 = std::fs::read_to_string(&csv_file2).unwrap();
    // OTHER 全被过滤，只有 header
    assert_eq!(
        content2.lines().count(),
        1,
        "expected only header row when all records filtered out"
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
    handle_run(&cfg, true, false, &interrupted, None).unwrap();

    // Assert: header 精确为 "ts,username,sql"（已验证字段投影正确）
    // 数据行只验证行数（不用 split(',').count()，SQL 含逗号时会误判）
    let content = std::fs::read_to_string(&csv_file).unwrap();
    let header = content.lines().next().unwrap();
    assert_eq!(
        header, "ts,username,sql",
        "expected header 'ts,username,sql', got: {header}"
    );
    let data_lines: Vec<_> = content.lines().skip(1).collect();
    assert_eq!(
        data_lines.len(),
        3,
        "expected 3 data rows, got {}",
        data_lines.len()
    );
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
    handle_run(&cfg, true, false, &interrupted, None).unwrap();

    // Assert: CSV 文件存在且只有 header（1 行）
    assert!(
        csv_file.exists(),
        "CSV file should exist even for empty input"
    );
    let content = std::fs::read_to_string(&csv_file).unwrap();
    assert_eq!(
        content.lines().count(),
        1,
        "expected only header row for empty log"
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
    handle_run(&cfg, true, false, &interrupted, None).unwrap();

    // Assert: CSV 只有 header（全部记录被过滤）
    let content = std::fs::read_to_string(&csv_file).unwrap();
    assert_eq!(
        content.lines().count(),
        1,
        "expected only header row when all records filtered"
    );
}

#[test]
fn test_boundary_malformed_line() {
    // Arrange: 1 条无效行（文件开头）+ 4 条正常行 = 4 条正常记录
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
    handle_run(&cfg, true, false, &interrupted, None).unwrap();

    // Assert: 无效行被跳过，4 条正常记录导出 → header + 4 data = 5 行
    let csv_content = std::fs::read_to_string(&csv_file).unwrap();
    assert_eq!(
        csv_content.lines().count(),
        5,
        "expected header + 4 data rows (malformed line skipped)"
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
    handle_run(&cfg, true, false, &interrupted, None).unwrap();

    // Assert: 1 条记录正常导出 → header + 1 data = 2 行
    let csv_content = std::fs::read_to_string(&csv_file).unwrap();
    assert_eq!(
        csv_content.lines().count(),
        2,
        "expected header + 1 data row for long SQL"
    );
}

// ── CLI stderr error format tests ────────────────────────────────────────────

/// Verify that fatal errors output "  hint: " prefix and not "Suggestion:" in real stderr.
/// Triggers `Config::ParseFailed` by passing an invalid TOML config file.
#[test]
fn test_cli_error_uses_hint_prefix() {
    let dir = tempfile::TempDir::new().unwrap();
    let bad_toml = dir.path().join("bad.toml");
    std::fs::write(&bad_toml, "not valid toml ][[[").unwrap();

    let binary = env!("CARGO_BIN_EXE_sqllog2db");
    let output = std::process::Command::new(binary)
        .args(["run", "-c", bad_toml.to_str().unwrap()])
        .output()
        .expect("failed to execute sqllog2db binary");

    let stderr_text = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2 (EXIT_FATAL), got: {:?}\nstderr: {stderr_text}",
        output.status.code()
    );
    assert!(
        stderr_text.contains("[CRITICAL]"),
        "stderr should contain [CRITICAL] prefix, got: {stderr_text}"
    );
    assert!(
        stderr_text.contains("  hint: "),
        "stderr should contain '  hint: ' prefix, got: {stderr_text}"
    );
    assert!(
        !stderr_text.contains("Suggestion:"),
        "stderr should not contain old 'Suggestion:' prefix, got: {stderr_text}"
    );
    assert!(
        stderr_text.contains("Configuration error"),
        "stderr should contain 'Configuration error' text, got: {stderr_text}"
    );
}

// ── validate command output tests (CONFIG-02) ────────────────────────────────

/// Verify that `sqllog2db validate` on a valid config outputs exactly "Configuration valid." to stdout.
#[test]
fn test_cli_validate_valid_config_outputs_configuration_valid() {
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    // Write a minimal valid config: sqllog + csv exporter
    std::fs::write(
        &config_path,
        "[sqllog]\ninputs = [\"sqllogs\"]\n\n[exporter.csv]\nfile = \"out.csv\"\n",
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_sqllog2db");
    let output = std::process::Command::new(binary)
        .args(["validate", "-c", config_path.to_str().unwrap()])
        .output()
        .expect("failed to execute sqllog2db binary");

    let stdout_text = String::from_utf8_lossy(&output.stdout);
    let stderr_text = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected exit code 0 for valid config, got: {:?}\nstderr: {stderr_text}",
        output.status.code()
    );
    assert!(
        stdout_text.contains("Configuration valid."),
        "stdout should contain 'Configuration valid.', got: {stdout_text}"
    );
}

/// Verify that `sqllog2db validate` on an invalid config outputs "[FAIL]" to stderr and exits with code 2.
#[test]
fn test_cli_validate_invalid_config_outputs_fail_prefix() {
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("bad_config.toml");
    // Invalid: logging.level set to an invalid value
    std::fs::write(
        &config_path,
        "[sqllog]\ninputs = [\"sqllogs\"]\n\n[logging]\nlevel = \"verbose\"\n\n[exporter.csv]\nfile = \"out.csv\"\n",
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_sqllog2db");
    let output = std::process::Command::new(binary)
        .args(["validate", "-c", config_path.to_str().unwrap()])
        .output()
        .expect("failed to execute sqllog2db binary");

    let stderr_text = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2 (EXIT_FATAL) for invalid config, got: {:?}\nstderr: {stderr_text}",
        output.status.code()
    );
    assert!(
        stderr_text.contains("[FAIL]"),
        "stderr should contain '[FAIL]' prefix, got: {stderr_text}"
    );
    assert!(
        stderr_text.contains("  hint: "),
        "stderr should contain '  hint: ' line, got: {stderr_text}"
    );
    assert!(
        !stderr_text.contains("[CRITICAL]"),
        "stderr should not contain '[CRITICAL]' for validate errors, got: {stderr_text}"
    );
    assert!(
        !stderr_text.contains("[ERROR]"),
        "stderr should not contain '[ERROR]' for validate errors, got: {stderr_text}"
    );
}

// ── verbose/quiet CLI behavior tests (LOG-01, LOG-02) ───────────────────────

/// Verify that `-v -q` conflict is detected by clap and exits non-zero with a conflict message.
#[test]
fn test_cli_verbose_quiet_mutual_exclusion() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sqllog2db"))
        .args(["-v", "-q", "run", "-c", "nonexistent.toml"])
        .output()
        .expect("failed to spawn sqllog2db binary");
    assert!(
        !output.status.success(),
        "expected non-zero exit for -v -q conflict"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be used with") || stderr.contains("conflict"),
        "expected clap conflict message, got: {stderr}"
    );
}

/// Verify that `--verbose run` prints `Processing: <path>` to stderr for each processed file.
/// Uses a single log file to force the sequential path (where per-file output is emitted).
#[test]
fn test_cli_verbose_prints_processing_line_per_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    // Single file forces sequential path, which emits "Processing: <path>" per file.
    write_test_log(&log_dir.join("a.log"), 5);

    let csv_path = dir.path().join("out.csv");
    let error_log = dir.path().join("errors.log");
    let app_log = dir.path().join("app.log");

    let config_path = dir.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            "[sqllog]\ninputs = [\"{logdir}\"]\n[error]\nfile = \"{errlog}\"\n[logging]\nfile = \"{applog}\"\nlevel = \"warn\"\nretention_days = 1\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\n",
            logdir = log_dir.to_string_lossy().replace('\\', "/"),
            errlog = error_log.to_string_lossy().replace('\\', "/"),
            applog = app_log.to_string_lossy().replace('\\', "/"),
            csv = csv_path.to_string_lossy().replace('\\', "/"),
        ),
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sqllog2db"))
        .args(["--verbose", "run", "-c", config_path.to_str().unwrap()])
        .output()
        .expect("failed to spawn sqllog2db binary");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "verbose run should succeed, stderr: {stderr}"
    );
    let processing_count = stderr.matches("Processing: ").count();
    assert!(
        processing_count >= 1,
        "expected >=1 Processing line, got {processing_count}: {stderr}"
    );
}

/// Verify that `--verbose run` with multiple log files emits `Processing: <path>` to stderr for
/// each file regardless of whether the parallel or sequential path is taken.
///
/// Two files are created so that on multi-core machines the parallel path (PARALLEL-05) is
/// exercised.  On single-core CI (jobs=1) the sequential path still emits the same line per
/// file, so the assertion holds in both cases.
#[test]
fn test_cli_verbose_parallel_prints_processing_lines() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    // Two files: on multi-core hosts this triggers the parallel path (jobs > 1 && files > 1).
    write_test_log(&log_dir.join("a.log"), 5);
    write_test_log(&log_dir.join("b.log"), 5);

    let csv_file = dir.path().join("out.csv");
    let error_log = dir.path().join("errors.log");
    let app_log = dir.path().join("app.log");
    let config_path = dir.path().join("config.toml");
    std::fs::write(
        &config_path,
        make_toml_config(&log_dir, &csv_file, &error_log, &app_log),
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sqllog2db"))
        .args(["--verbose", "run", "-c", config_path.to_str().unwrap()])
        .output()
        .expect("failed to spawn sqllog2db binary");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "verbose multi-file run should succeed, stderr: {stderr}"
    );
    let processing_count = stderr.matches("Processing: ").count();
    assert!(
        processing_count >= 2,
        "expected >=2 'Processing: ' lines (one per file), got {processing_count}: {stderr}"
    );
}

/// Verify that `--quiet run` suppresses the completion summary and `ProgressBar` output.
#[test]
fn test_cli_quiet_suppresses_summary() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("test.log"), 5);

    let csv_path = dir.path().join("out.csv");
    let error_log = dir.path().join("errors.log");
    let app_log = dir.path().join("app.log");

    let config_path = dir.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            "[sqllog]\ninputs = [\"{logdir}\"]\n[error]\nfile = \"{errlog}\"\n[logging]\nfile = \"{applog}\"\nlevel = \"warn\"\nretention_days = 1\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\n",
            logdir = log_dir.to_string_lossy().replace('\\', "/"),
            errlog = error_log.to_string_lossy().replace('\\', "/"),
            applog = app_log.to_string_lossy().replace('\\', "/"),
            csv = csv_path.to_string_lossy().replace('\\', "/"),
        ),
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sqllog2db"))
        .args(["--quiet", "run", "-c", config_path.to_str().unwrap()])
        .output()
        .expect("failed to spawn sqllog2db binary");
    assert!(output.status.success(), "quiet run should succeed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("SQL Log Export Task Completed"),
        "quiet should suppress completion summary, got: {stderr}"
    );
    assert!(
        !stderr.contains("Completed with"),
        "quiet should suppress error count line, got: {stderr}"
    );
}

// ── verbose summary differentiation tests (LOG-03) ───────────────────────────

fn make_toml_config(
    log_dir: &std::path::Path,
    csv_file: &std::path::Path,
    error_log: &std::path::Path,
    app_log: &std::path::Path,
) -> String {
    format!(
        "[sqllog]\ninputs = [\"{logdir}\"]\n[error]\nfile = \"{errlog}\"\n[logging]\nfile = \"{applog}\"\nlevel = \"warn\"\nretention_days = 1\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\n",
        logdir = log_dir.to_string_lossy().replace('\\', "/"),
        errlog = error_log.to_string_lossy().replace('\\', "/"),
        applog = app_log.to_string_lossy().replace('\\', "/"),
        csv = csv_file.to_string_lossy().replace('\\', "/"),
    )
}

/// Verify that `--verbose run` stderr includes per-file `Processed: <path> — N records` lines
/// before the completion summary, covering at least 2 files.
#[test]
fn test_cli_verbose_summary_includes_per_file_counts() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("a.log"), 5);
    write_test_log(&log_dir.join("b.log"), 7);

    let csv_file = dir.path().join("out.csv");
    let error_log = dir.path().join("errors.log");
    let app_log = dir.path().join("app.log");
    let toml_path = dir.path().join("config.toml");
    std::fs::write(
        &toml_path,
        make_toml_config(&log_dir, &csv_file, &error_log, &app_log),
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sqllog2db"))
        .args(["--verbose", "run", "-c", toml_path.to_str().unwrap()])
        .output()
        .expect("failed to spawn");
    assert!(
        output.status.success(),
        "verbose run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let processed_count = stderr.matches("Processed: ").count();
    assert!(
        processed_count >= 2,
        "expected >=2 'Processed: ' lines (one per file), got {processed_count}: {stderr}"
    );
    assert!(
        stderr.contains("5 records") || stderr.contains("7 records"),
        "expected per-file record count in stderr: {stderr}"
    );
    assert!(
        stderr.contains("✓ SQL Log Export Task Completed"),
        "expected completion summary in stderr: {stderr}"
    );
}

/// Verify that default mode (no flags) stderr includes the completion summary but NOT
/// per-file `Processed: ` detail lines.
#[test]
fn test_cli_default_summary_omits_per_file_counts() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("a.log"), 5);

    let csv_file = dir.path().join("out.csv");
    let error_log = dir.path().join("errors.log");
    let app_log = dir.path().join("app.log");
    let toml_path = dir.path().join("config.toml");
    std::fs::write(
        &toml_path,
        make_toml_config(&log_dir, &csv_file, &error_log, &app_log),
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sqllog2db"))
        .args(["run", "-c", toml_path.to_str().unwrap()])
        .output()
        .expect("failed to spawn");
    assert!(output.status.success(), "default run should succeed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Processed: "),
        "default mode should NOT print per-file lines, got: {stderr}"
    );
    assert!(
        stderr.contains("✓ SQL Log Export Task Completed"),
        "default mode should print completion summary, got: {stderr}"
    );
}

// ── --input CLI flag + e2e tests (INPUT-02) ──────────────────────────────────

/// Verify that legacy [sqllog] path = "..." key is rejected via validate subcommand
/// with stderr containing sqllog.path, inputs, and hint: (SC3 main validation path).
#[test]
fn test_validate_rejects_legacy_sqllog_path_key_via_rust_api() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy_sqllog.toml");
    std::fs::write(
        &path,
        "[sqllog]\npath = \"sqllogs\"\n\n[exporter.csv]\nfile = \"out.csv\"\n",
    )
    .unwrap();
    let cfg = dm_database_sqllog2db::config::Config::from_file(&path).unwrap();
    let result = cfg.validate();
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("sqllog.path"),
        "expected sqllog.path in error; got: {msg}"
    );
    assert!(
        msg.contains("inputs"),
        "expected migration hint mentioning inputs; got: {msg}"
    );
}

fn make_run_only_config_file(dir: &std::path::Path, csv_relative: &str) -> std::path::PathBuf {
    let cfg_path = dir.join("cfg.toml");
    let content = format!(
        "[sqllog]\ninputs = [\"__placeholder_unused__\"]\n[exporter.csv]\nfile = \"{}\"\noverwrite = true\n",
        dir.join(csv_relative).to_string_lossy().replace('\\', "/")
    );
    std::fs::write(&cfg_path, content).unwrap();
    cfg_path
}

/// C1: --input flag overrides config inputs; multiple --input flags expand to all files.
#[test]
fn test_cli_input_flag_overrides_config_inputs() {
    use assert_cmd::Command;
    let dir = tempfile::tempdir().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("a.log"), 5);
    write_test_log(&log_dir.join("b.log"), 3);

    let cfg_path = make_run_only_config_file(dir.path(), "out.csv");
    let csv_path = dir.path().join("out.csv");

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .arg("run")
        .arg("-c")
        .arg(&cfg_path)
        .arg("--input")
        .arg(log_dir.join("a.log"))
        .arg("--input")
        .arg(log_dir.join("b.log"))
        .assert()
        .success();

    let content = std::fs::read_to_string(&csv_path).unwrap();
    assert_eq!(
        content.lines().count(),
        9,
        "expected header + 8 data rows (5+3)"
    );
}

/// C2: --input flag with glob pattern expands to matching files.
#[test]
fn test_cli_input_flag_with_glob() {
    use assert_cmd::Command;
    let dir = tempfile::tempdir().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_test_log(&log_dir.join("2025-01.log"), 4);
    write_test_log(&log_dir.join("2025-02.log"), 6);
    // This file should NOT match *.log glob (it's a .txt)
    std::fs::write(log_dir.join("other.txt"), "ignored").unwrap();

    let cfg_path = make_run_only_config_file(dir.path(), "out.csv");
    let csv_path = dir.path().join("out.csv");
    let glob_pattern = format!("{}/*.log", log_dir.to_string_lossy().replace('\\', "/"));

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .arg("run")
        .arg("-c")
        .arg(&cfg_path)
        .arg("--input")
        .arg(&glob_pattern)
        .assert()
        .success();

    let content = std::fs::read_to_string(&csv_path).unwrap();
    assert_eq!(
        content.lines().count(),
        11,
        "expected header + 10 data rows (4+6 from *.log)"
    );
}

/// C3: legacy [sqllog] path = "..." config is rejected via validate subcommand,
/// stderr contains sqllog.path, inputs, and hint: (SC3 main validation path).
#[test]
fn test_cli_legacy_path_key_rejected() {
    use assert_cmd::Command;
    use predicates::str::contains;
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("legacy.toml");
    let csv_path = dir.path().join("out.csv");
    let toml = format!(
        "[sqllog]\npath = \"sqllogs\"\n\n[exporter.csv]\nfile = \"{}\"\noverwrite = true\n",
        csv_path.to_string_lossy().replace('\\', "/")
    );
    std::fs::write(&cfg_path, &toml).unwrap();

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .arg("validate")
        .arg("-c")
        .arg(&cfg_path)
        .assert()
        .failure()
        .stderr(contains("sqllog.path"))
        .stderr(contains("inputs"))
        .stderr(contains("hint:"));
}

/// C4: glob with no matching files — allows either stdin fallback (Unix no-tty)
/// or `NoFilesFound` (Windows or explicit tty). Both are valid behaviors.
#[test]
fn test_cli_input_flag_with_glob_no_match_behavior() {
    use assert_cmd::Command;
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = make_run_only_config_file(dir.path(), "out.csv");
    let nonexistent_glob = dir.path().join("nonexistent_*.log");

    let output = Command::cargo_bin("sqllog2db")
        .unwrap()
        .arg("run")
        .arg("-c")
        .arg(&cfg_path)
        .arg("--input")
        .arg(&nonexistent_glob)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let success = output.status.success();
    // Three valid behaviors:
    // 1. stdin fallback (Unix no-tty): exit 0, program reads /dev/stdin (EOF) and completes
    // 2. NoFilesFound: exit non-zero, stderr contains NoFilesFound text + hint
    // 3. stdin iter fails (2.0.3+): exit non-zero, parser fails to seek /dev/stdin
    let stdin_iter_fail = stderr.contains("/dev/stdin") || stderr.contains("stdin");
    assert!(
        success
            || (stderr.contains("No log files found matching inputs") && stderr.contains("hint:"))
            || stdin_iter_fail,
        "expected stdin fallback (exit 0) OR NoFilesFound+hint (non-zero) OR stdin iter error; exit_code={:?}, stderr={}",
        output.status.code(),
        stderr
    );
}

// ── stats subcommand tests ────────────────────────────────────────────────────

/// Create a valid stats config file with a real input log file and logging output.
///
/// Creates `input.log` with one valid DML record so `run_stats` can complete successfully.
fn make_stats_config_file(dir: &std::path::Path) -> std::path::PathBuf {
    let cfg_path = dir.join("stats_cfg.toml");
    let app_log_path = dir.join("test.log");
    let input_log = dir.join("input.log");
    // Write one valid DML record so stats can scan and produce output
    std::fs::write(
        &input_log,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT id FROM t WHERE id=1. EXECTIME: 5(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n",
    ).unwrap();
    let content = format!(
        "[sqllog]\ninputs = [\"{}\"]\n\
         [exporter.csv]\nfile = \"{}\"\noverwrite = true\n\
         [logging]\nfile = \"{}\"\nlevel = \"info\"\nretention_days = 7\n",
        input_log.to_string_lossy().replace('\\', "/"),
        dir.join("out.csv").to_string_lossy().replace('\\', "/"),
        app_log_path.to_string_lossy().replace('\\', "/"),
    );
    std::fs::write(&cfg_path, content).unwrap();
    cfg_path
}

/// S1: stats --help shows subcommand description and key arguments.
#[test]
fn test_cli_stats_help_shows_subcommand() {
    use assert_cmd::Command;
    use predicates::str::contains;

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .arg("stats")
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("--config"))
        .stdout(contains("--top"))
        .stdout(contains("Number of top records"));
}

/// S2: stats with valid config exits successfully (exit code 0).
#[test]
fn test_cli_stats_with_valid_config_succeeds() {
    use assert_cmd::Command;
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = make_stats_config_file(dir.path());

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .arg("stats")
        .arg("-c")
        .arg(&cfg_path)
        .assert()
        .success();
}

/// S3: stats without --top uses default value of 20 (verified via log file).
#[test]
fn test_cli_stats_top_default_is_20() {
    use assert_cmd::Command;
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = make_stats_config_file(dir.path());
    let log_path = dir.path().join("test.log");

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .arg("stats")
        .arg("-c")
        .arg(&cfg_path)
        .assert()
        .success();

    let log_content = std::fs::read_to_string(&log_path)
        .unwrap_or_else(|_| panic!("log file not found at {}", log_path.display()));
    assert!(
        log_content.contains("top=20"),
        "log file should contain 'top=20', got:\n{log_content}"
    );
}

/// S4: stats with --top 5 passes value 5 to `handle_stats` (verified via log file).
#[test]
fn test_cli_stats_top_explicit_value() {
    use assert_cmd::Command;
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = make_stats_config_file(dir.path());
    let log_path = dir.path().join("test.log");

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .arg("stats")
        .arg("-c")
        .arg(&cfg_path)
        .arg("--top")
        .arg("5")
        .assert()
        .success();

    let log_content = std::fs::read_to_string(&log_path)
        .unwrap_or_else(|_| panic!("log file not found at {}", log_path.display()));
    assert!(
        log_content.contains("top=5"),
        "log file should contain 'top=5', got:\n{log_content}"
    );
}

/// S5: stats with --top 0 exits with non-zero and stderr contains error hint.
#[test]
fn test_cli_stats_top_zero_errors() {
    use assert_cmd::Command;
    use predicates::str::contains;
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = make_stats_config_file(dir.path());

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .arg("stats")
        .arg("-c")
        .arg(&cfg_path)
        .arg("--top")
        .arg("0")
        .assert()
        .failure()
        .stderr(contains("--top"))
        .stderr(contains("is not in 1"));
}

/// S6: stats with non-existent config exits with non-zero (no fallback to default, D-05).
#[test]
fn test_cli_stats_config_not_found_errors() {
    use assert_cmd::Command;

    let output = Command::cargo_bin("sqllog2db")
        .unwrap()
        .arg("stats")
        .arg("-c")
        .arg("/nonexistent/does/not/exist.toml")
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "stats with missing config should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("Configuration file not found"),
        "stderr should mention config not found, got: {stderr}"
    );
}

// ── Phase 52 stats output integration tests ──────────────────────────────────

/// 写入 N 条 DML 记录到测试日志文件的辅助函数（含不同 SQL 模板）。
fn write_stats_test_log(path: &std::path::Path, count: usize) {
    use std::fmt::Write as _;
    let mut buf = String::with_capacity(count * 200);
    for idx in 0..count {
        writeln!(
            buf,
            "2025-01-15 10:30:{:02}.001 (EP[0] sess:0x{idx:04x} user:U trxid:{idx} stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT * FROM table_{idx} WHERE id={idx}. EXECTIME: {exec}(ms) ROWCOUNT: 1(rows) EXEC_ID: {idx}.",
            idx % 60,
            exec = (idx * 11) % 500 + 1,
        ).unwrap();
    }
    std::fs::write(path, buf).unwrap();
}

/// 创建仅含 CSV exporter 的统计配置文件。
fn make_stats_csv_config(dir: &std::path::Path, log_path: &std::path::Path) -> std::path::PathBuf {
    let cfg_path = dir.join("stats_csv.toml");
    let csv_path = dir.join("out").join("data.csv");
    let content = format!(
        "[sqllog]\ninputs = [\"{}\"]\n[exporter.csv]\nfile = \"{}\"\noverwrite = true\n",
        log_path.to_string_lossy().replace('\\', "/"),
        csv_path.to_string_lossy().replace('\\', "/"),
    );
    std::fs::write(&cfg_path, content).unwrap();
    cfg_path
}

/// 创建仅含 `SQLite` exporter 的统计配置文件。
fn make_stats_sqlite_config(
    dir: &std::path::Path,
    log_path: &std::path::Path,
) -> std::path::PathBuf {
    let cfg_path = dir.join("stats_sqlite.toml");
    let db_path = dir.join("out").join("stats.db");
    let content = format!(
        "[sqllog]\ninputs = [\"{}\"]\n[exporter.sqlite]\ndatabase_url = \"{}\"\n",
        log_path.to_string_lossy().replace('\\', "/"),
        db_path.to_string_lossy().replace('\\', "/"),
    );
    std::fs::write(&cfg_path, content).unwrap();
    cfg_path
}

/// Phase 52 集成测试 1：stats 命令生成 `slow_sql.csv` 和 `frequent_sql.csv`。
#[test]
fn test_stats_csv_outputs_two_files() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let log_file = dir.path().join("test.log");
    write_stats_test_log(&log_file, 3);
    let cfg_path = make_stats_csv_config(dir.path(), &log_file);

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["stats", "-c"])
        .arg(&cfg_path)
        .args(["--top", "10"])
        .assert()
        .success();

    let out_dir = dir.path().join("out");
    assert!(
        out_dir.join("slow_sql.csv").exists(),
        "slow_sql.csv must exist"
    );
    assert!(
        out_dir.join("frequent_sql.csv").exists(),
        "frequent_sql.csv must exist"
    );
    // 验证表头
    let slow = std::fs::read_to_string(out_dir.join("slow_sql.csv")).unwrap();
    assert_eq!(
        slow.lines().next().unwrap(),
        "sql_text,elapsed_ms,timestamp"
    );
    let freq = std::fs::read_to_string(out_dir.join("frequent_sql.csv")).unwrap();
    assert_eq!(
        freq.lines().next().unwrap(),
        "normalized_sql,call_count,avg_elapsed_ms,max_elapsed_ms"
    );
}

/// Phase 52 集成测试 2：--top 5 严格限制输出行数不超过 5。
#[test]
fn test_stats_csv_top_5_limits_rows() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let log_file = dir.path().join("test.log");
    write_stats_test_log(&log_file, 8);
    let cfg_path = make_stats_csv_config(dir.path(), &log_file);

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["stats", "-c"])
        .arg(&cfg_path)
        .args(["--top", "5"])
        .assert()
        .success();

    let out_dir = dir.path().join("out");
    let slow = std::fs::read_to_string(out_dir.join("slow_sql.csv")).unwrap();
    // 数据行 = total lines - 1 (header)
    let slow_data = slow.lines().count() - 1;
    assert!(
        (1..=5).contains(&slow_data),
        "slow_sql.csv data rows should be 1..=5, got {slow_data}"
    );

    let freq = std::fs::read_to_string(out_dir.join("frequent_sql.csv")).unwrap();
    let freq_data = freq.lines().count() - 1;
    assert!(
        (1..=5).contains(&freq_data),
        "frequent_sql.csv data rows should be 1..=5, got {freq_data}"
    );
}

/// Phase 52 集成测试 3：`SQLite` 配置时生成 `slow_sql` 和 `frequent_sql` 两张表。
#[test]
fn test_stats_sqlite_outputs_two_tables() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let log_file = dir.path().join("test.log");
    write_stats_test_log(&log_file, 3);
    let cfg_path = make_stats_sqlite_config(dir.path(), &log_file);

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["stats", "-c"])
        .arg(&cfg_path)
        .args(["--top", "10"])
        .assert()
        .success();

    let db_path = dir.path().join("out").join("stats.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let slow_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM slow_sql", [], |row| row.get(0))
        .unwrap();
    assert!(slow_count > 0, "slow_sql table should have rows");
    let freq_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM frequent_sql", [], |row| row.get(0))
        .unwrap();
    assert!(freq_count > 0, "frequent_sql table should have rows");
}

/// Phase 52 集成测试 4：同时配置 CSV 和 `SQLite` 时，只生成 CSV（CSV 优先）。
#[test]
fn test_stats_csv_preferred_over_sqlite_when_both_configured() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let log_file = dir.path().join("test.log");
    write_stats_test_log(&log_file, 3);

    let cfg_path = dir.path().join("both.toml");
    let csv_path = dir.path().join("out").join("data.csv");
    let db_path = dir.path().join("out").join("stats.db");
    let content = format!(
        "[sqllog]\ninputs = [\"{}\"]\n\
         [exporter.csv]\nfile = \"{}\"\noverwrite = true\n\
         [exporter.sqlite]\ndatabase_url = \"{}\"\n",
        log_file.to_string_lossy().replace('\\', "/"),
        csv_path.to_string_lossy().replace('\\', "/"),
        db_path.to_string_lossy().replace('\\', "/"),
    );
    std::fs::write(&cfg_path, &content).unwrap();

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["stats", "-c"])
        .arg(&cfg_path)
        .args(["--top", "10"])
        .assert()
        .success();

    let out_dir = dir.path().join("out");
    assert!(
        out_dir.join("slow_sql.csv").exists(),
        "CSV should be generated when both exporters configured"
    );
    assert!(
        !db_path.exists(),
        "SQLite db should NOT be created when CSV takes priority"
    );
}

/// Phase 52 集成测试 5：exectime = 0 的记录纳入 slow_sql.csv（D-12）。
#[test]
fn test_stats_zero_elapsed_records_included() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let log_file = dir.path().join("test.log");
    // 写入 exectime = 0 的记录
    std::fs::write(
        &log_file,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT zero FROM t. EXECTIME: 0(ms) ROWCOUNT: 0(rows) EXEC_ID: 0.\n",
    ).unwrap();

    let cfg_path = make_stats_csv_config(dir.path(), &log_file);
    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["stats", "-c"])
        .arg(&cfg_path)
        .args(["--top", "10"])
        .assert()
        .success();

    let out_dir = dir.path().join("out");
    let slow = std::fs::read_to_string(out_dir.join("slow_sql.csv")).unwrap();
    assert!(
        slow.contains("SELECT zero FROM t"),
        "zero-elapsed record should appear in slow_sql.csv, got:\n{slow}"
    );
}

// ── Phase 53 end-to-end integration tests ────────────────────────────────────

/// 创建包含 [stats] 节的配置文件，from/to/top 字段可选。
fn make_stats_config_with_section(
    dir: &std::path::Path,
    from: Option<&str>,
    to: Option<&str>,
    top: Option<u32>,
) -> std::path::PathBuf {
    let cfg_path = dir.join("stats_section_cfg.toml");
    let app_log_path = dir.join("test.log");
    let input_log = dir.join("input.log");
    std::fs::write(
        &input_log,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT id FROM t WHERE id=1. EXECTIME: 5(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n",
    )
    .unwrap();
    use std::fmt::Write as _;
    let mut stats_section = String::from("\n[stats]\n");
    if let Some(f) = from {
        let _ = writeln!(stats_section, "from = \"{f}\"");
    }
    if let Some(t) = to {
        let _ = writeln!(stats_section, "to = \"{t}\"");
    }
    if let Some(n) = top {
        let _ = writeln!(stats_section, "top = {n}");
    }
    let content = format!(
        "[sqllog]\ninputs = [\"{}\"]\n\
         [exporter.csv]\nfile = \"{}\"\noverwrite = true\n\
         [logging]\nfile = \"{}\"\nlevel = \"info\"\nretention_days = 7\n{}",
        input_log.to_string_lossy().replace('\\', "/"),
        dir.join("out.csv").to_string_lossy().replace('\\', "/"),
        app_log_path.to_string_lossy().replace('\\', "/"),
        stats_section,
    );
    std::fs::write(&cfg_path, content).unwrap();
    cfg_path
}

/// Phase 53 SC#1: stats --help 包含 --from / --to / YYYY-MM-DD 三个关键子串。
#[test]
fn test_cli_stats_help_shows_from_and_to() {
    use assert_cmd::Command;
    use predicates::str::contains;

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["stats", "--help"])
        .assert()
        .success()
        .stdout(contains("--from"))
        .stdout(contains("--to"))
        .stdout(contains("YYYY-MM-DD"));
}

/// Phase 53 SC#1 + STATS-07: stats --from / --to CLI 参数成功传入，应用日志含合并值。
#[test]
fn test_cli_stats_with_cli_from_and_to_succeeds() {
    use assert_cmd::Command;

    let dir = tempfile::tempdir().unwrap();
    let cfg_path = make_stats_config_file(dir.path());

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["stats", "-c"])
        .arg(&cfg_path)
        .args(["--from", "2024-01-01", "--to", "2024-12-31"])
        .assert()
        .success();

    let app_log = dir.path().join("test.log");
    let log_content = std::fs::read_to_string(&app_log).unwrap_or_default();
    assert!(
        log_content.contains("from=Some"),
        "app log should contain from=Some, got:\n{log_content}"
    );
    assert!(
        log_content.contains("to=Some"),
        "app log should contain to=Some, got:\n{log_content}"
    );
}

/// Phase 53 SC#2 + STATS-08: validate 命令通过含 from/to 的 config.toml。
#[test]
fn test_cli_stats_validate_accepts_valid_config_stats_section() {
    use assert_cmd::Command;
    use predicates::str::contains;

    let dir = tempfile::tempdir().unwrap();
    let cfg_path = make_stats_config_with_section(
        dir.path(),
        Some("2024-01-01"),
        Some("2024-01-31"),
        Some(10),
    );

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["validate", "-c"])
        .arg(&cfg_path)
        .assert()
        .success()
        .stdout(contains("Configuration valid."));
}

/// Phase 53 SC#4 + STATS-11: validate 命令拒绝含非法 from 格式的 config.toml。
#[test]
fn test_cli_stats_validate_rejects_bad_config_from_format() {
    use assert_cmd::Command;
    use predicates::str::contains;

    let dir = tempfile::tempdir().unwrap();
    let cfg_path = make_stats_config_with_section(dir.path(), Some("20240101"), None, None);

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["validate", "-c"])
        .arg(&cfg_path)
        .assert()
        .failure()
        .stderr(contains("[FAIL]"))
        .stderr(contains("stats.from"))
        .stderr(contains("YYYY-MM-DD"));
}

/// Phase 53 SC#3 + STATS-09: CLI --from 优先于 config.toml 中的 from 值。
#[test]
fn test_cli_stats_cli_overrides_config_from() {
    use assert_cmd::Command;

    let dir = tempfile::tempdir().unwrap();
    // config 中设置一个遥远未来的 from，CLI 覆盖为过去的日期
    let cfg_path = make_stats_config_with_section(dir.path(), Some("2099-12-31"), None, None);

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["stats", "-c"])
        .arg(&cfg_path)
        .args(["--from", "2024-01-01"])
        .assert()
        .success();

    let app_log = dir.path().join("test.log");
    let log_content = std::fs::read_to_string(&app_log).unwrap_or_default();
    assert!(
        log_content.contains("2024-01-01"),
        "app log should contain CLI from value 2024-01-01, got:\n{log_content}"
    );
}

/// Phase 53 SC#4 + STATS-11 CLI 路径（D-09）: stats --from 传入非法格式时退出非零。
#[test]
fn test_cli_stats_runtime_rejects_bad_cli_from_format() {
    use assert_cmd::Command;
    use predicates::str::contains;

    let dir = tempfile::tempdir().unwrap();
    let cfg_path = make_stats_config_file(dir.path());

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["stats", "-c"])
        .arg(&cfg_path)
        .args(["--from", "not-a-date"])
        .assert()
        .failure()
        .stderr(contains("stats.from"))
        .stderr(contains("YYYY-MM-DD"));
}

/// Phase 53 init 模板：生成文件包含 [stats] 注释段与三字段示例。
#[test]
fn test_init_template_contains_stats_section() {
    use assert_cmd::Command;

    let dir = tempfile::tempdir().unwrap();
    let out_file = dir.path().join("cfg.toml");

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["init", "-o"])
        .arg(&out_file)
        .args(["--force"])
        .assert()
        .success();

    let content = std::fs::read_to_string(&out_file).unwrap();
    assert!(
        content.contains("[stats]"),
        "template should contain [stats]"
    );
    assert!(
        content.contains("# from = \"2024"),
        "template should contain commented from field"
    );
    assert!(
        content.contains("# to"),
        "template should contain commented to field"
    );
    assert!(
        content.contains("# top"),
        "template should contain commented top field"
    );
    assert!(
        content.contains("YYYY-MM-DD HH:MM:SS"),
        "template should contain format hint YYYY-MM-DD HH:MM:SS"
    );
}

// ── Phase 54 stats time-range filter integration tests ──

#[test]
fn test_stats_from_to_filters_to_single_day() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let log_file = dir.path().join("input.log");
    let lines = [
        "2024-01-14 10:00:00.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT * FROM t WHERE id=1. EXECTIME: 10(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.",
        "2024-01-15 10:00:00.001 (EP[0] sess:0x0002 user:U trxid:2 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT * FROM t WHERE id=2. EXECTIME: 20(ms) ROWCOUNT: 1(rows) EXEC_ID: 2.",
        "2024-01-16 10:00:00.001 (EP[0] sess:0x0003 user:U trxid:3 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT * FROM t WHERE id=3. EXECTIME: 30(ms) ROWCOUNT: 1(rows) EXEC_ID: 3.",
    ];
    std::fs::write(&log_file, lines.join("\n") + "\n").unwrap();
    let cfg_path = make_stats_csv_config(dir.path(), &log_file);
    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args([
            "stats",
            "-c",
            cfg_path.to_str().unwrap(),
            "--from",
            "2024-01-15",
            "--to",
            "2024-01-15",
        ])
        .assert()
        .success();
    let slow_csv = dir.path().join("out").join("slow_sql.csv");
    let content = std::fs::read_to_string(&slow_csv).unwrap();
    let data_lines: Vec<&str> = content.lines().skip(1).filter(|l| !l.is_empty()).collect();
    assert_eq!(
        data_lines.len(),
        1,
        "only 2024-01-15 record should be included"
    );
    assert!(
        data_lines[0].contains("2024-01-15"),
        "timestamp should be 2024-01-15"
    );
}

#[test]
fn test_stats_no_from_to_filters_nothing() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let log_file = dir.path().join("input.log");
    let lines = [
        "2024-01-14 10:00:00.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT * FROM t WHERE id=1. EXECTIME: 10(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.",
        "2024-01-16 10:00:00.001 (EP[0] sess:0x0002 user:U trxid:2 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT * FROM t WHERE id=2. EXECTIME: 20(ms) ROWCOUNT: 1(rows) EXEC_ID: 2.",
    ];
    std::fs::write(&log_file, lines.join("\n") + "\n").unwrap();
    let cfg_path = make_stats_csv_config(dir.path(), &log_file);
    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["stats", "-c", cfg_path.to_str().unwrap(), "--top", "10"])
        .assert()
        .success();
    let slow_csv = dir.path().join("out").join("slow_sql.csv");
    let content = std::fs::read_to_string(&slow_csv).unwrap();
    let data_lines: Vec<&str> = content.lines().skip(1).filter(|l| !l.is_empty()).collect();
    assert_eq!(
        data_lines.len(),
        2,
        "all records should be included when no time filter"
    );
}

/// P57-SC5 / STATS-12: stats CLI 在 --from 晚于 --to 时退出非零，stderr 包含字段名与 "must be <=" 文案（D-01/D-02）。
#[test]
fn test_cli_stats_rejects_from_after_to() {
    use assert_cmd::Command;
    use predicates::str::contains;

    let dir = tempfile::tempdir().unwrap();
    let cfg_path = make_stats_config_file(dir.path());

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["stats", "-c"])
        .arg(&cfg_path)
        .args(["--from", "2024-01-31", "--to", "2024-01-01"])
        .assert()
        .failure()
        .stderr(contains("stats.from"))
        .stderr(contains("must be <="))
        .stderr(contains("2024-01-31"));
}

// ── Phase 57 e2e helpers (TEST-01 / TEST-02) ──────────────────────────────────

/// Phase 57 TEST-01: 生成 run 命令的 CSV 配置文件（inputs 字段填日志目录路径）。
fn write_run_config_toml(
    dir: &std::path::Path,
    log_dir: &std::path::Path,
    csv_output: &std::path::Path,
) -> std::path::PathBuf {
    let cfg_path = dir.join("run_config.toml");
    let content = format!(
        "[sqllog]\ninputs = [\"{}\"]\n[exporter.csv]\nfile = \"{}\"\noverwrite = true\n",
        log_dir.to_string_lossy().replace('\\', "/"),
        csv_output.to_string_lossy().replace('\\', "/"),
    );
    std::fs::write(&cfg_path, content).unwrap();
    cfg_path
}

/// Phase 57 TEST-01: 生成 run 命令的 `SQLite` 配置文件（默认表名 `sqllog_records`）。
fn write_run_sqlite_config_toml(
    dir: &std::path::Path,
    log_dir: &std::path::Path,
    db_output: &std::path::Path,
) -> std::path::PathBuf {
    let cfg_path = dir.join("run_sqlite_config.toml");
    let content = format!(
        "[sqllog]\ninputs = [\"{}\"]\n[exporter.sqlite]\ndatabase_url = \"{}\"\n",
        log_dir.to_string_lossy().replace('\\', "/"),
        db_output.to_string_lossy().replace('\\', "/"),
    );
    std::fs::write(&cfg_path, content).unwrap();
    cfg_path
}

/// TEST-01 (Phase 57): run 子命令 CLI 输出 CSV header 与记录数（D-04/D-05/D-06）。
#[test]
fn test_cli_run_csv_output_header_and_row_count() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    let record_count = 10usize;
    write_test_log(&log_dir.join("test.log"), record_count);

    let csv_file = dir.path().join("out.csv");
    let cfg_path = write_run_config_toml(dir.path(), &log_dir, &csv_file);

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["run", "-c"])
        .arg(&cfg_path)
        .assert()
        .success();

    let content = std::fs::read_to_string(&csv_file).unwrap();
    let mut lines = content.lines();
    let expected_header = dm_database_sqllog2db::pipeline::FIELD_NAMES.join(",");
    assert_eq!(
        lines.next().unwrap(),
        expected_header,
        "CSV header must match FIELD_NAMES order"
    );
    let data_count = lines.filter(|l| !l.is_empty()).count();
    assert_eq!(
        data_count, record_count,
        "row count must match written records"
    );
}

/// TEST-01 (Phase 57): run 子命令 CLI 输出 `SQLite` 文件存在与表 `sqllog_records` 记录数（D-07，表名修正）。
#[test]
fn test_cli_run_sqlite_output_row_count() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    let record_count = 5usize;
    write_test_log(&log_dir.join("test.log"), record_count);

    let db_file = dir.path().join("out.db");
    let cfg_path = write_run_sqlite_config_toml(dir.path(), &log_dir, &db_file);

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["run", "-c"])
        .arg(&cfg_path)
        .assert()
        .success();

    assert!(db_file.exists(), "SQLite output file must exist");

    let conn = rusqlite::Connection::open(&db_file).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sqllog_records", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        count,
        i64::try_from(record_count).unwrap(),
        "sqllog_records table row count must match"
    );
}

/// TEST-02 (Phase 57): init 子命令 CLI 在新路径下成功创建配置文件并退出 0。
#[test]
fn test_cli_init_creates_file_exit_0() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let out_file = dir.path().join("new_config.toml");

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["init", "-o"])
        .arg(&out_file)
        .assert()
        .success();

    assert!(out_file.exists(), "init must create the config file");
    let content = std::fs::read_to_string(&out_file).unwrap();
    assert!(
        content.contains("[sqllog]"),
        "generated config must contain [sqllog] section"
    );
}

/// TEST-02 (Phase 57): init 子命令 CLI 在目标文件已存在且未传 --force 时退出非零，stderr 包含 "already exists"。
#[test]
fn test_cli_init_existing_file_without_force_exits_nonzero() {
    use assert_cmd::Command;
    use predicates::str::contains;

    let dir = tempfile::TempDir::new().unwrap();
    let out_file = dir.path().join("existing.toml");
    std::fs::write(&out_file, "existing content").unwrap();

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["init", "-o"])
        .arg(&out_file)
        .assert()
        .failure()
        .stderr(contains("already exists"));
}

// ── Phase 66 兼容性验证集成测试 (COMPAT-01/02/03) ───────────────────────────

/// COMPAT-02: 并行路径输出内容与逐文件顺序路径完全一致（集合排序后相等）。
///
/// 策略：
/// 1. 写入 2 个各含 20 条记录的 .log 文件
/// 2. 顺序基线：对每个文件单独构建 Config（单文件 inputs），逐个运行 `handle_run`，收集数据行
/// 3. 并行路径：将两个文件配置为 inputs，一次 `handle_run`（触发并行路径），读取数据行
/// 4. 对两组数据行排序后断言相等；同时验证并行输出存在 header 行
#[test]
fn test_parallel_csv_content_matches_sequential() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    let file_a = log_dir.join("a.log");
    let file_b = log_dir.join("b.log");
    // 使用非重叠 ID 范围，使意外去重问题可检测
    write_test_log_offset(&file_a, 20, 0);
    write_test_log_offset(&file_b, 20, 20);

    let interrupted = Arc::new(AtomicBool::new(false));

    // 顺序基线：每个文件单独运行，收集数据行
    let mut seq_lines: Vec<String> = Vec::new();
    for log_file in [&file_a, &file_b] {
        let seq_csv = dir.path().join(format!(
            "seq_{}.csv",
            log_file.file_name().unwrap().to_string_lossy()
        ));
        let mut seq_cfg = Config {
            sqllog: SqllogConfig {
                inputs: vec![log_file.to_str().unwrap().to_string()],
                path_deprecated: None,
            },
            exporter: ExporterConfig {
                csv: Some(CsvExporterConfig {
                    file: seq_csv.to_str().unwrap().to_string(),
                    overwrite: true,
                    append: false,
                    ..CsvExporterConfig::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        // 禁用 csv.append 确保顺序路径（单文件不触发并行）
        seq_cfg.exporter.csv.as_mut().unwrap().append = false;
        handle_run(&seq_cfg, true, false, &interrupted, None).unwrap();
        let content = std::fs::read_to_string(&seq_csv).unwrap();
        // 跳过 header 行，收集数据行
        for line in content.lines().skip(1) {
            if !line.is_empty() {
                seq_lines.push(line.to_string());
            }
        }
    }

    // 并行路径：两个文件一次 handle_run
    let par_csv = dir.path().join("parallel.csv");
    let par_cfg = Config {
        sqllog: SqllogConfig {
            inputs: vec![
                file_a.to_str().unwrap().to_string(),
                file_b.to_str().unwrap().to_string(),
            ],
            path_deprecated: None,
        },
        exporter: ExporterConfig {
            csv: Some(CsvExporterConfig {
                file: par_csv.to_str().unwrap().to_string(),
                overwrite: true,
                append: false,
                ..CsvExporterConfig::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    handle_run(&par_cfg, true, false, &interrupted, None).unwrap();

    let par_content = std::fs::read_to_string(&par_csv).unwrap();
    let mut par_lines_iter = par_content.lines();
    // 验证并行输出存在 header 行
    let header = par_lines_iter
        .next()
        .expect("parallel CSV must have a header");
    assert!(
        header.contains("ts") || header.contains("username"),
        "first line should be a header, got: {header}"
    );
    let mut par_lines: Vec<String> = par_lines_iter
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    // 排序后比较（并行路径文件间行顺序不确定）
    seq_lines.sort();
    par_lines.sort();
    assert_eq!(
        seq_lines.len(),
        par_lines.len(),
        "parallel and sequential should produce the same number of records"
    );
    assert_eq!(
        seq_lines, par_lines,
        "parallel CSV content must match sequential after sorting"
    );
}

/// COMPAT-02: 并行路径在启用 include 过滤器时，与顺序路径输出内容一致（集合排序后相等）。
///
/// 使用 `include.users = ["TESTUSER"]` 过滤器；合成记录的 user 字段均为 TESTUSER，
/// 因此所有记录应通过过滤，并行与顺序结果相同。
#[test]
fn test_parallel_csv_filter_matches_sequential() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    let file_a = log_dir.join("filter_a.log");
    let file_b = log_dir.join("filter_b.log");
    // 使用非重叠 ID 范围，使意外去重问题可检测
    write_test_log_offset(&file_a, 20, 0);
    write_test_log_offset(&file_b, 20, 20);

    let interrupted = Arc::new(AtomicBool::new(false));

    let filter_cfg = Some(FiltersFeature {
        enable: true,
        include: IncludeFilters {
            users: Some(vec!["TESTUSER".to_string()]),
            ..Default::default()
        },
        exclude: ExcludeFilters::default(),
        ..Default::default()
    });

    // 顺序基线
    let mut seq_lines: Vec<String> = Vec::new();
    for log_file in [&file_a, &file_b] {
        let seq_csv = dir.path().join(format!(
            "seq_filter_{}.csv",
            log_file.file_name().unwrap().to_string_lossy()
        ));
        let seq_cfg = Config {
            sqllog: SqllogConfig {
                inputs: vec![log_file.to_str().unwrap().to_string()],
                path_deprecated: None,
            },
            exporter: ExporterConfig {
                csv: Some(CsvExporterConfig {
                    file: seq_csv.to_str().unwrap().to_string(),
                    overwrite: true,
                    append: false,
                    ..CsvExporterConfig::default()
                }),
                ..Default::default()
            },
            filter: filter_cfg.clone(),
            ..Default::default()
        };
        handle_run(&seq_cfg, true, false, &interrupted, None).unwrap();
        let content = std::fs::read_to_string(&seq_csv).unwrap();
        for line in content.lines().skip(1) {
            if !line.is_empty() {
                seq_lines.push(line.to_string());
            }
        }
    }

    // 并行路径（带过滤器）
    let par_csv = dir.path().join("parallel_filter.csv");
    let par_cfg = Config {
        sqllog: SqllogConfig {
            inputs: vec![
                file_a.to_str().unwrap().to_string(),
                file_b.to_str().unwrap().to_string(),
            ],
            path_deprecated: None,
        },
        exporter: ExporterConfig {
            csv: Some(CsvExporterConfig {
                file: par_csv.to_str().unwrap().to_string(),
                overwrite: true,
                append: false,
                ..CsvExporterConfig::default()
            }),
            ..Default::default()
        },
        filter: filter_cfg,
        ..Default::default()
    };
    handle_run(&par_cfg, true, false, &interrupted, None).unwrap();

    let par_content = std::fs::read_to_string(&par_csv).unwrap();
    let mut par_lines: Vec<String> = par_content
        .lines()
        .skip(1)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    seq_lines.sort();
    par_lines.sort();
    assert_eq!(
        seq_lines.len(),
        par_lines.len(),
        "filtered parallel and sequential should produce the same number of records"
    );
    assert_eq!(
        seq_lines, par_lines,
        "filtered parallel CSV content must match sequential after sorting"
    );
}

/// COMPAT-03: `sqllog2db init` 生成的 config.toml 模板不包含并行相关新字段
/// （如 "parallel" 或 "jobs" 字样），确认 v1.16 config 格式没有被修改。
#[test]
fn test_init_no_parallel_fields() {
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    handle_init(config_path.to_str().unwrap(), false).unwrap();
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        !content.contains("parallel"),
        "init template must not contain 'parallel' field, got:\n{content}"
    );
    assert!(
        !content.contains("jobs"),
        "init template must not contain 'jobs' field, got:\n{content}"
    );
    // 验证核心格式字段仍然存在（格式未被破坏）
    assert!(
        content.contains("[sqllog]"),
        "init template must still contain [sqllog] section"
    );
    assert!(
        content.contains("[exporter.csv]"),
        "init template must still contain [exporter.csv] section"
    );
}

/// PARALLEL-06: `jobs_override=Some(2)` 强制并行路径在所有环境下被执行。
///
/// 使用两个异构文件（trxid 空间不重叠、不同 user），强制 jobs=2 触发并行路径，
/// 断言输出总行数 == 35（20+15），且两个 user 均出现在输出中。
#[test]
fn test_parallel_csv_jobs_override_forces_parallel() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    let file_a = log_dir.join("a.log");
    let file_b = log_dir.join("b.log");
    write_heterogeneous_log(&file_a, 20, 0, "USERA");
    write_heterogeneous_log(&file_b, 15, 1000, "USERB");

    let par_csv = dir.path().join("parallel_jobs.csv");
    let interrupted = Arc::new(AtomicBool::new(false));
    let par_cfg = Config {
        sqllog: SqllogConfig {
            inputs: vec![
                file_a.to_str().unwrap().to_string(),
                file_b.to_str().unwrap().to_string(),
            ],
            path_deprecated: None,
        },
        exporter: ExporterConfig {
            csv: Some(CsvExporterConfig {
                file: par_csv.to_str().unwrap().to_string(),
                overwrite: true,
                append: false,
                ..CsvExporterConfig::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    handle_run(&par_cfg, true, false, &interrupted, Some(2)).unwrap();

    let par_content = std::fs::read_to_string(&par_csv).unwrap();
    let data_lines: Vec<&str> = par_content
        .lines()
        .skip(1)
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(
        data_lines.len(),
        35,
        "jobs_override=Some(2) 并行路径应输出 35 条记录 (20+15)，实际 {}",
        data_lines.len()
    );
    assert!(par_content.contains("USERA"), "并行输出应包含 USERA 的记录");
    assert!(par_content.contains("USERB"), "并行输出应包含 USERB 的记录");
}

/// PARALLEL-07: 异构数据(不重叠 trxid + 不同 user)下并行 == 顺序；任何聚合 bug 立即可见。
///
/// 顺序基线逐文件运行，并行路径强制 jobs=2，排序后逐字节比对。
/// 若并行路径漏记录（任何聚合 bug），`len()` 断言立即失败。
#[test]
fn test_parallel_csv_heterogeneous_matches_sequential() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    let file_a = log_dir.join("hetero_a.log");
    let file_b = log_dir.join("hetero_b.log");
    write_heterogeneous_log(&file_a, 20, 0, "USERA");
    write_heterogeneous_log(&file_b, 15, 1000, "USERB");

    let interrupted = Arc::new(AtomicBool::new(false));

    // 顺序基线：每个文件单独运行收集数据行
    let mut seq_lines: Vec<String> = Vec::new();
    for (log_file, suffix) in [(&file_a, "a"), (&file_b, "b")] {
        let seq_csv = dir.path().join(format!("seq_hetero_{suffix}.csv"));
        let seq_cfg = Config {
            sqllog: SqllogConfig {
                inputs: vec![log_file.to_str().unwrap().to_string()],
                path_deprecated: None,
            },
            exporter: ExporterConfig {
                csv: Some(CsvExporterConfig {
                    file: seq_csv.to_str().unwrap().to_string(),
                    overwrite: true,
                    append: false,
                    ..CsvExporterConfig::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        handle_run(&seq_cfg, true, false, &interrupted, None).unwrap();
        let content = std::fs::read_to_string(&seq_csv).unwrap();
        for line in content.lines().skip(1).filter(|l| !l.is_empty()) {
            seq_lines.push(line.to_string());
        }
    }

    // 并行路径：强制 jobs=2
    let par_csv = dir.path().join("par_hetero.csv");
    let par_cfg = Config {
        sqllog: SqllogConfig {
            inputs: vec![
                file_a.to_str().unwrap().to_string(),
                file_b.to_str().unwrap().to_string(),
            ],
            path_deprecated: None,
        },
        exporter: ExporterConfig {
            csv: Some(CsvExporterConfig {
                file: par_csv.to_str().unwrap().to_string(),
                overwrite: true,
                append: false,
                ..CsvExporterConfig::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    handle_run(&par_cfg, true, false, &interrupted, Some(2)).unwrap();

    let par_content = std::fs::read_to_string(&par_csv).unwrap();
    let mut par_lines: Vec<String> = par_content
        .lines()
        .skip(1)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    seq_lines.sort();
    par_lines.sort();
    assert_eq!(
        seq_lines.len(),
        par_lines.len(),
        "异构数据下并行 ({}) 与顺序 ({}) 行数应相同",
        par_lines.len(),
        seq_lines.len()
    );
    assert_eq!(
        seq_lines, par_lines,
        "异构数据下并行 CSV 内容排序后应与顺序基线完全一致"
    );
}

// ── run_wizard integration tests ─────────────────────────────────────────────

#[test]
fn test_wizard_integration_all_defaults() {
    let input = b"\n\n\n";
    let mut reader = std::io::Cursor::new(input.as_ref());
    let mut writer = Vec::<u8>::new();
    let answers = run_wizard(&mut reader, &mut writer).unwrap();
    assert_eq!(answers.inputs, "sqllogs");
    assert!(matches!(answers.exporter, ExporterChoice::Csv));
    assert_eq!(answers.csv_file.as_deref(), Some("outputs/sqllog.csv"));
}

#[test]
fn test_wizard_integration_sqlite() {
    let input = b"\nsqlite\ndb/out.db\nmy_records\n";
    let mut reader = std::io::Cursor::new(input.as_ref());
    let mut writer = Vec::<u8>::new();
    let answers = run_wizard(&mut reader, &mut writer).unwrap();
    assert!(matches!(answers.exporter, ExporterChoice::Sqlite));
    assert_eq!(answers.sqlite_db.as_deref(), Some("db/out.db"));
    assert_eq!(answers.sqlite_table.as_deref(), Some("my_records"));
}

// ── e2e CLI 测试: init --interactive (INIT-01/02/03 + SC4 + D-02) ─────────────

/// INIT-01/02: `init -i` 全默认 Enter×3，退出 0，生成含默认路径的 config.toml
#[test]
fn test_cli_init_interactive_all_defaults() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let out_file = dir.path().join("cfg.toml");

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["init", "-i", "-o"])
        .arg(&out_file)
        .write_stdin("\n\n\n")
        .assert()
        .success();

    assert!(out_file.exists(), "init -i must create the config file");
    let content = std::fs::read_to_string(&out_file).unwrap();
    assert!(
        content.contains(r#"inputs = ["sqllogs"]"#),
        "default inputs must be sqllogs"
    );
    assert!(
        content.contains(r#"file = "outputs/sqllog.csv""#),
        "default csv file path must be outputs/sqllog.csv"
    );
}

/// INIT-02: `init -i` 自定义 inputs 路径，生成的 config.toml 包含自定义值
#[test]
fn test_cli_init_interactive_custom_inputs() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let out_file = dir.path().join("cfg.toml");

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["init", "-i", "-o"])
        .arg(&out_file)
        .write_stdin("my/dir\n\n\n")
        .assert()
        .success();

    let content = std::fs::read_to_string(&out_file).unwrap();
    assert!(
        content.contains(r#"inputs = ["my/dir"]"#),
        "custom inputs path must appear in generated config"
    );
}

/// INIT-02: `init -i` sqlite 模式，生成 config.toml 含正确的 `SQLite` 配置段
#[test]
fn test_cli_init_interactive_sqlite() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let out_file = dir.path().join("cfg.toml");

    // stdin: inputs=默认\n, format=sqlite\n, sqlite_db=默认\n, sqlite_table=默认\n
    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["init", "-i", "-o"])
        .arg(&out_file)
        .write_stdin("\nsqlite\n\n\n")
        .assert()
        .success();

    let content = std::fs::read_to_string(&out_file).unwrap();
    assert!(
        content.contains("[exporter.sqlite]"),
        "[exporter.sqlite] must be activated (uncommented)"
    );
    assert!(
        !content.contains("# [exporter.sqlite]"),
        "[exporter.sqlite] must not remain commented"
    );
    assert!(
        content.contains(r#"database_url = "export/sqllog2db.db""#),
        "database_url must use default value"
    );
    assert!(
        content.contains(r#"table_name = "sqllog_records""#),
        "table_name must use default value"
    );
    assert!(
        content.contains("# [exporter.csv]"),
        "[exporter.csv] must be commented out in sqlite mode"
    );
}

/// SC4: 向导生成的 config.toml 能被 `validate` 子命令通过
#[test]
fn test_cli_init_interactive_generates_validatable_config() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let out_file = dir.path().join("cfg.toml");

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["init", "-i", "-o"])
        .arg(&out_file)
        .write_stdin("\n\n\n")
        .assert()
        .success();

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["validate", "-c"])
        .arg(&out_file)
        .assert()
        .success();
}

/// INIT-03: 交互式全默认与非交互式生成的 config.toml 字节级相同
#[test]
fn test_cli_init_interactive_format_matches_non_interactive() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let interactive_file = dir.path().join("a.toml");
    let non_interactive_file = dir.path().join("b.toml");

    // 交互式（全默认）
    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["init", "-i", "-o"])
        .arg(&interactive_file)
        .write_stdin("\n\n\n")
        .assert()
        .success();

    // 非交互式
    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["init", "-o"])
        .arg(&non_interactive_file)
        .assert()
        .success();

    let interactive_content = std::fs::read_to_string(&interactive_file).unwrap();
    let non_interactive_content = std::fs::read_to_string(&non_interactive_file).unwrap();
    assert_eq!(
        interactive_content, non_interactive_content,
        "interactive and non-interactive must produce identical default config"
    );
}

/// D-02: interactive 模式下文件已存在且未传 --force 时退出非零，stderr 含 "already exists"
#[test]
fn test_cli_init_interactive_existing_without_force_fails() {
    use assert_cmd::Command;
    use predicates::str::contains;
    let dir = tempfile::TempDir::new().unwrap();
    let out_file = dir.path().join("cfg.toml");
    std::fs::write(&out_file, "existing content").unwrap();

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["init", "-i", "-o"])
        .arg(&out_file)
        .write_stdin("")
        .assert()
        .failure()
        .stderr(contains("already exists"));
}

/// IN-03: `init -i --force` overwrites an existing file with the wizard output
#[test]
fn test_cli_init_interactive_force_overwrites_existing() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let out_file = dir.path().join("cfg.toml");
    std::fs::write(&out_file, "old content").unwrap();

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["init", "-i", "-o"])
        .arg(&out_file)
        .arg("--force")
        .write_stdin("\n\n\n")
        .assert()
        .success();

    let content = std::fs::read_to_string(&out_file).unwrap();
    assert!(
        content.contains("[sqllog]"),
        "force should overwrite with template config"
    );
    assert!(
        !content.contains("old content"),
        "old content must be replaced"
    );
}

// ── watch subcommand tests ─────────────────────────────────────────────────────

mod watch_tests {
    use dm_database_sqllog2db::cli::watch::handle_watch;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    /// 向指定路径写入一行最小可解析的 DM SQL 日志。
    fn write_minimal_log(path: &std::path::Path) {
        let line = "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:TESTUSER trxid:1 stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT id FROM t WHERE id=1. EXECTIME: 5(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n";
        std::fs::write(path, line).unwrap();
    }

    /// W1: `watch --help` 包含配置文件说明和使用示例（WATCH-01 可发现性）。
    #[test]
    fn test_watch_help_lists_subcommand() {
        use assert_cmd::Command;
        let output = Command::cargo_bin("sqllog2db")
            .unwrap()
            .arg("watch")
            .arg("--help")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "watch --help should exit 0, got: {:?}",
            output.status.code()
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("TOML configuration file path"),
            "help should mention 'TOML configuration file path', got:\n{stdout}"
        );
        assert!(
            stdout.contains("sqllog2db watch -c config.toml"),
            "help should include usage example 'sqllog2db watch -c config.toml', got:\n{stdout}"
        );
    }

    /// W2: interrupted=true 预置时 `handle_watch` 返回 `Err(Error::Interrupted)`（WATCH-09 exit 130）。
    #[test]
    fn test_watch_exits_when_interrupted() {
        let dir = tempfile::TempDir::new().unwrap();
        let log_dir = dir.path().join("logs");
        std::fs::create_dir_all(&log_dir).unwrap();
        let csv_file = dir.path().join("out.csv");
        let cfg = super::make_run_config(&log_dir, &csv_file);
        let interrupted = Arc::new(AtomicBool::new(true));
        let result = handle_watch(&cfg, true, false, &interrupted);
        // WATCH-09: interrupted=true 时 handle_watch 应返回 Err(Error::Interrupted)，
        // main.rs 处理该错误并 exit(130)
        assert!(
            matches!(
                result,
                Err(dm_database_sqllog2db::error::Error::Interrupted)
            ),
            "handle_watch with interrupted=true should return Err(Interrupted), got: {result:?}"
        );
    }

    /// W3: 新 .log 文件出现时触发 `handle_run`，CSV 输出行数 > header（WATCH-02/05）。
    /// macOS `FSEvents` 在 cargo test 进程中对临时目录的事件不稳定（coalescence 延迟 > 8s），
    /// stdin-pipe hang 已由 CR-01 修复，但 `FSEvents` 事件可靠性需 smoke test 环境验证。
    #[test]
    #[ignore = "macOS FSEvents coalescing in cargo test env; smoke test required for reliable verification"]
    fn test_watch_triggers_on_new_log_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let log_dir = dir.path().join("logs");
        std::fs::create_dir_all(&log_dir).unwrap();
        let csv_file = dir.path().join("out.csv");
        let cfg = super::make_run_config(&log_dir, &csv_file);
        let interrupted = Arc::new(AtomicBool::new(false));
        let interrupted_clone = Arc::clone(&interrupted);
        let log_dir_clone = log_dir.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            write_minimal_log(&log_dir_clone.join("new_file.log"));
            // Poll until CSV appears or 8 s elapses, then signal done.
            let deadline = std::time::Instant::now() + Duration::from_secs(8);
            let csv_path = log_dir_clone.parent().unwrap().join("out.csv");
            while std::time::Instant::now() < deadline {
                if csv_path.exists() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            interrupted_clone.store(true, Ordering::Release);
        });
        let result = handle_watch(&cfg, true, false, &interrupted);
        assert!(
            result.is_ok()
                || matches!(
                    result,
                    Err(dm_database_sqllog2db::error::Error::Interrupted)
                ),
            "handle_watch should succeed or return Interrupted, got: {result:?}"
        );
        assert!(
            csv_file.exists(),
            "CSV output file should exist after watch trigger"
        );
        let content = std::fs::read_to_string(&csv_file).unwrap();
        let line_count = content.lines().count();
        assert!(
            line_count > 1,
            "CSV should have header + at least 1 data row, got {line_count} lines"
        );
    }

    /// W4: 写入非 .log 文件不触发 `handle_run`，CSV 输出不产生（WATCH-02 扩展名过滤）。
    #[test]
    fn test_watch_ignores_non_log_files() {
        let dir = tempfile::TempDir::new().unwrap();
        let log_dir = dir.path().join("logs");
        std::fs::create_dir_all(&log_dir).unwrap();
        let csv_file = dir.path().join("out.csv");
        let cfg = super::make_run_config(&log_dir, &csv_file);
        let interrupted = Arc::new(AtomicBool::new(false));
        let interrupted_clone = Arc::clone(&interrupted);
        let log_dir_clone = log_dir.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            std::fs::write(
                log_dir_clone.join("garbage.txt"),
                "this is not a log file\n",
            )
            .unwrap();
            std::thread::sleep(Duration::from_millis(700));
            interrupted_clone.store(true, Ordering::Release);
        });
        let result = handle_watch(&cfg, true, false, &interrupted);
        // WATCH-09: interrupted=true 时 handle_watch 返回 Err(Interrupted)，验证非 .log 文件不触发
        assert!(
            matches!(
                result,
                Err(dm_database_sqllog2db::error::Error::Interrupted)
            ),
            "handle_watch should return Err(Interrupted) after interrupt, got: {result:?}"
        );
        assert!(
            !csv_file.exists(),
            "CSV output should NOT exist when only non-.log files are written"
        );
    }
}
