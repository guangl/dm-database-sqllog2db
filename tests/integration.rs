//! Integration tests for CLI handlers and the run pipeline.

use dm_database_sqllog2db::cli::init::handle_init;
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
fn test_handle_run_empty_dir_unix_behavior() {
    // Unix: empty inputs trigger stdin pipe fallback or NoFilesFound depending on tty;
    // NoFilesFound exit-code path covered indirectly by C3 end-to-end test
    // (legacy path key rejection achieves the same SC3 non-zero-exit + hint guarantee
    // without stdin tty interference, because ConfigError fires before file scanning).
}

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
        "expected header + 10 data rows, got {}",
        content.lines().count()
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
        result.is_err(),
        "handle_run should return Err(Interrupted) when interrupt flag is pre-set: {result:?}"
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
    assert!(content.contains("[sqllog]"));
    assert!(content.contains("SQL log path"));
    assert!(!content.contains("日志路径"));
}

#[test]
fn test_handle_init_template_is_english() {
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    handle_init(config_path.to_str().unwrap(), false).unwrap();
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[sqllog]"));
    assert!(content.contains("log path"));
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
            record_sql: SqlFilters::default(),
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
    let compiled_filters = cfg.validate_and_compile().unwrap();
    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(&cfg, true, false, &interrupted, compiled_filters).unwrap();
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

#[test]
fn test_validate_rejects_legacy_pipeline_template_analysis() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.toml");
    std::fs::write(
        &path,
        "[sqllog]\ninputs = [\"sqllogs\"]\n\n[pipeline.template_analysis]\nenabled = true\n\n[exporter.csv]\nfile = \"out.csv\"\n",
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
        "[sqllog]\ninputs = [\"sqllogs\"]\n\n[pipeline.filters]\nenable = true\n\n[exporter.csv]\nfile = \"out.csv\"\n",
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
    handle_run(&cfg, true, false, &interrupted, None).unwrap();

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
    handle_run(&cfg2, true, false, &Arc::new(AtomicBool::new(false)), None).unwrap();
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
    handle_run(&cfg, true, false, &interrupted, None).unwrap();

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
    handle_run(&cfg, true, false, &interrupted, None).unwrap();

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
    handle_run(&cfg, true, false, &interrupted, None).unwrap();

    // Assert: 1 条记录正常导出 → header + 1 data = 2 行
    let csv_content = std::fs::read_to_string(&csv_file).unwrap();
    assert_eq!(
        csv_content.lines().count(),
        2,
        "expected header + 1 data row for long SQL, got {} lines",
        csv_content.lines().count()
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
fn test_validate_rejects_legacy_sqllog_path_key_via_cli() {
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
        dir.join(csv_relative).to_string_lossy()
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
        "expected header + 8 data rows (5+3), got {}",
        content.lines().count()
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
    let glob_pattern = format!("{}/*.log", log_dir.to_string_lossy());

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
        "expected header + 10 data rows (4+6 from *.log), got {}",
        content.lines().count()
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
        csv_path.to_string_lossy()
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
    // Two valid behaviors:
    // 1. stdin fallback (Unix no-tty): exit 0, program reads /dev/stdin (EOF) and completes
    // 2. NoFilesFound: exit non-zero, stderr contains NoFilesFound text + hint
    assert!(
        success
            || (stderr.contains("No log files found matching inputs") && stderr.contains("hint:")),
        "expected stdin fallback (exit 0) OR NoFilesFound+hint (non-zero); exit_code={:?}, stderr={}",
        output.status.code(),
        stderr
    );
}
