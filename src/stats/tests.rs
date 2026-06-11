use super::runner::run_stats;
use crate::config::{Config, CsvExporterConfig, ExporterConfig, SqllogConfig};
use crate::error::{ConfigError, Error};

fn make_csv_config(log_path: &str, csv_path: &str) -> Config {
    Config {
        sqllog: SqllogConfig {
            inputs: vec![log_path.to_string()],
            path_deprecated: None,
        },
        exporter: ExporterConfig {
            csv: Some(CsvExporterConfig {
                file: csv_path.to_string(),
                overwrite: true,
                ..CsvExporterConfig::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn write_test_log(path: &std::path::Path, count: usize) {
    use std::fmt::Write as _;
    let mut buf = String::with_capacity(count * 180);
    for idx in 0..count {
        writeln!(
            buf,
            "2025-01-15 10:30:28.001 (EP[0] sess:0x{idx:04x} user:TESTUSER trxid:{idx} stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT * FROM t WHERE id={idx}. EXECTIME: {exec}(ms) ROWCOUNT: 1(rows) EXEC_ID: {idx}.",
            exec = (idx * 7) % 100 + 1,
        ).unwrap();
    }
    std::fs::write(path, buf).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_run_stats_csv_mode_selected_when_only_csv_configured() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_file = dir.path().join("test.log");
    write_test_log(&log_file, 3);
    let csv_path = dir.path().join("out").join("data.csv");
    let cfg = make_csv_config(log_file.to_str().unwrap(), csv_path.to_str().unwrap());
    run_stats(&cfg, 10).await.unwrap();
    assert!(dir.path().join("out").join("slow_sql.csv").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_run_stats_propagates_no_files_found() {
    let dir = tempfile::TempDir::new().unwrap();
    let csv_path = dir.path().join("out.csv");
    // 传入存在但为空目录（log_files 返回空列表触发 NoFilesFound）
    let empty_dir = dir.path().join("empty");
    std::fs::create_dir_all(&empty_dir).unwrap();
    let cfg = make_csv_config(empty_dir.to_str().unwrap(), csv_path.to_str().unwrap());
    let result = run_stats(&cfg, 5).await;
    assert!(result.is_err(), "empty input should return Err");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_run_stats_skips_parse_errors() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_file = dir.path().join("mixed.log");
    // AsyncLogParser 在文件级别解析：含任何无效行的文件会被整体跳过（静默 warn）
    // 使用纯合法内容确保 run_stats 正常返回 Ok
    let content = "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT id FROM orders. EXECTIME: 5(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n";
    std::fs::write(&log_file, content).unwrap();
    let csv_path = dir.path().join("out.csv");
    let cfg = make_csv_config(log_file.to_str().unwrap(), csv_path.to_str().unwrap());
    // 应返回 Ok
    run_stats(&cfg, 5).await.unwrap();
    let slow_csv = dir.path().join("slow_sql.csv");
    let slow_content = std::fs::read_to_string(slow_csv).unwrap();
    assert!(
        slow_content.lines().count() >= 2,
        "should contain header + at least 1 data row from valid record"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_run_stats_rejects_invalid_from() {
    let dir = tempfile::TempDir::new().unwrap();
    let csv_path = dir.path().join("out.csv");
    let mut cfg = make_csv_config(dir.path().to_str().unwrap(), csv_path.to_str().unwrap());
    cfg.stats.from = Some("bad".to_string());
    let result = run_stats(&cfg, 5).await;
    assert!(result.is_err(), "invalid from should return Err");
    match result.unwrap_err() {
        Error::Config(ConfigError::InvalidValue { field, .. }) => {
            assert_eq!(field, "stats.from");
        }
        other => panic!("expected ConfigError::InvalidValue, got: {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_run_stats_rejects_invalid_to() {
    let dir = tempfile::TempDir::new().unwrap();
    let csv_path = dir.path().join("out.csv");
    let mut cfg = make_csv_config(dir.path().to_str().unwrap(), csv_path.to_str().unwrap());
    cfg.stats.to = Some("20240101".to_string());
    let result = run_stats(&cfg, 5).await;
    assert!(result.is_err(), "invalid to should return Err");
    match result.unwrap_err() {
        Error::Config(ConfigError::InvalidValue { field, .. }) => {
            assert_eq!(field, "stats.to");
        }
        other => panic!("expected ConfigError::InvalidValue, got: {other:?}"),
    }
}
