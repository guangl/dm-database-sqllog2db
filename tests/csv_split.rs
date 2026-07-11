//! 集成测试 — CSV `max_rows_per_file` 自动分割。
//!
//! 覆盖：
//! - 多文件输入下分割生效（回退顺序路径，不被并行 concat 成单文件）
//! - 记录数恰为 `max_rows_per_file` 整数倍时不产生仅含表头的空尾文件
//! - overwrite 模式清理上一轮遗留的分割文件（含编号空洞）
//! - 配置校验拒绝非法组合

use dm_database_sqllog2db::cli::run::handle_run;
use dm_database_sqllog2db::config::{Config, CsvExporterConfig, ExporterConfig, SqllogConfig};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

// ── helpers ──────────────────────────────────────────────────────────────────

/// 向 `path` 写入 `count` 条达梦格式日志，trxid 从 `start` 递增（跨文件不重叠）。
fn write_log(path: &Path, count: usize, start: usize) {
    use std::fmt::Write as _;
    let mut buf = String::with_capacity(count * 180);
    for n in 0..count {
        let i = start + n;
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

fn split_config(log_dir: &Path, csv_file: &Path, max_rows: usize) -> Config {
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
                max_rows_per_file: Some(max_rows),
                ..CsvExporterConfig::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// 分割文件路径 `{parent}/{stem}_{index}.{ext}`。
fn split_path(csv_file: &Path, index: usize) -> PathBuf {
    let parent = csv_file.parent().unwrap();
    let stem = csv_file.file_stem().unwrap().to_str().unwrap();
    let ext = csv_file.extension().unwrap().to_str().unwrap();
    parent.join(format!("{stem}_{index}.{ext}"))
}

/// 返回文件的数据行数（总行数减去表头行）。
fn data_rows(path: &Path) -> usize {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let lines = content.lines().count();
    assert!(
        lines >= 1,
        "{} should have at least a header",
        path.display()
    );
    // 首行必须是表头（以字段名开头，而非日志时间戳）。
    let first = content.lines().next().unwrap();
    assert!(
        !first.starts_with("2025-"),
        "{} first line must be a header, got: {first}",
        path.display()
    );
    lines - 1
}

// ── tests ────────────────────────────────────────────────────────────────────

/// 多文件输入 + 分割：必须回退到顺序路径并真正分割，而不是被并行 concat 成单文件。
#[tokio::test(flavor = "multi_thread")]
async fn multi_file_input_honors_splitting() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_log(&log_dir.join("a.log"), 20, 0);
    write_log(&log_dir.join("b.log"), 10, 1000);

    let csv_file = dir.path().join("out.csv");
    let cfg = split_config(&log_dir, &csv_file, 10);

    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(&cfg, true, false, &interrupted, None)
        .await
        .unwrap();

    // 30 条记录 / 每文件 10 行 → 恰好 3 个分割文件。
    assert!(
        !csv_file.exists(),
        "unsplit output {} must NOT exist in split mode",
        csv_file.display()
    );
    let total: usize = (1..=3).map(|i| data_rows(&split_path(&csv_file, i))).sum();
    assert_eq!(total, 30, "all 30 records must be exported across splits");
    for i in 1..=3 {
        assert_eq!(
            data_rows(&split_path(&csv_file, i)),
            10,
            "split file _{i} should hold exactly max_rows rows"
        );
    }
    assert!(
        !split_path(&csv_file, 4).exists(),
        "no 4th split file expected for 30 records at max_rows=10"
    );
}

/// 记录数恰为 `max_rows` 整数倍：不得产生仅含表头的空尾文件。
#[tokio::test(flavor = "multi_thread")]
async fn exact_multiple_produces_no_empty_trailing_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_log(&log_dir.join("single.log"), 20, 0);

    let csv_file = dir.path().join("out.csv");
    let cfg = split_config(&log_dir, &csv_file, 10);

    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(&cfg, true, false, &interrupted, None)
        .await
        .unwrap();

    assert_eq!(data_rows(&split_path(&csv_file, 1)), 10);
    assert_eq!(data_rows(&split_path(&csv_file, 2)), 10);
    assert!(
        !split_path(&csv_file, 3).exists(),
        "20 records at max_rows=10 must yield exactly 2 files, not an empty _3"
    );
}

/// overwrite 模式清理上一轮遗留的分割文件，包括编号有空洞的情况。
#[tokio::test(flavor = "multi_thread")]
async fn stale_split_files_are_removed_even_with_gaps() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    write_log(&log_dir.join("single.log"), 5, 0);

    let csv_file = dir.path().join("out.csv");
    // 预置上一轮遗留：_1、_2、_5（缺 _3/_4 制造编号空洞）。
    for i in [1usize, 2, 5] {
        std::fs::write(split_path(&csv_file, i), "stale,leftover\nx,y\n").unwrap();
    }

    let cfg = split_config(&log_dir, &csv_file, 10);
    let interrupted = Arc::new(AtomicBool::new(false));
    handle_run(&cfg, true, false, &interrupted, None)
        .await
        .unwrap();

    // 5 条记录 → 仅 _1，且是本轮新写（数据行数为 5）。
    assert_eq!(data_rows(&split_path(&csv_file, 1)), 5);
    assert!(
        !split_path(&csv_file, 2).exists(),
        "stale _2 must be removed"
    );
    assert!(
        !split_path(&csv_file, 5).exists(),
        "stale _5 (gap-numbered) must be removed too"
    );
}

/// 配置校验拒绝非法的分割组合。
#[test]
fn validate_rejects_invalid_split_config() {
    let base = CsvExporterConfig {
        file: "out.csv".to_string(),
        overwrite: true,
        append: false,
        max_rows_per_file: None,
        ..CsvExporterConfig::default()
    };

    // max_rows = 0 非法。
    let zero = CsvExporterConfig {
        max_rows_per_file: Some(0),
        ..base.clone()
    };
    assert!(
        zero.validate().is_err(),
        "max_rows_per_file=0 must be rejected"
    );

    // append=true 与分割不兼容。
    let appended = CsvExporterConfig {
        max_rows_per_file: Some(10),
        append: true,
        overwrite: false,
        ..base.clone()
    };
    assert!(
        appended.validate().is_err(),
        "append + splitting must be rejected"
    );

    // 合法：overwrite=true, append=false, max_rows>0。
    let ok = CsvExporterConfig {
        max_rows_per_file: Some(10),
        ..base
    };
    assert!(ok.validate().is_ok(), "valid split config must pass");
}
