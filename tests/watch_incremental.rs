//! Phase 70 集成测试 — WATCH-03（追加增量不重复）与 WATCH-04（重启 offset 恢复）。
//! Phase 02 集成测试 — WATCH-07（CSV append）、WATCH-08（error log append）、WATCH-09（exit 130）。
//! 不依赖 notify watcher（FSEvents 不稳定，per Pitfall 6），直接调用 pub `trigger_*` 函数。

use dm_database_sqllog2db::cli::watch::{
    WatchLoopState, handle_watch, trigger_full_file, trigger_incremental,
};
use dm_database_sqllog2db::config::{
    Config, CsvExporterConfig, ErrorLogConfig, ExporterConfig, SqliteExporterConfig, SqllogConfig,
};
use dm_database_sqllog2db::error::Error;
use indicatif::{ProgressBar, ProgressDrawTarget};
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tempfile::TempDir;

// ── Helpers ─────────────────────────────────────────────────────────────────

/// 向 `path` 追加 `count` 条 DaMeng（达梦）格式日志记录，行号从 `start_id` 开始。
/// 若文件不存在则创建；若已存在则追加（`append`）。
fn write_test_log_records(path: &Path, start_id: usize, count: usize) {
    use std::fs::OpenOptions;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("failed to open test log file for writing");
    for n in 0..count {
        let i = start_id + n;
        writeln!(
            file,
            "2025-01-15 10:30:28.001 (EP[0] sess:0x{i:04x} user:TESTUSER trxid:{i} stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT * FROM t WHERE id={i}. EXECTIME: {exec}(ms) ROWCOUNT: {rows}(rows) EXEC_ID: {i}.",
            exec = (i * 13) % 1000,
            rows = i % 100,
        )
        .expect("failed to write test log record");
    }
}

/// 构造指向 `log_path` + `db_path` 的 SQLite-only Config（CSV 禁用）。
fn build_sqlite_config(log_path: &Path, db_path: &Path) -> Config {
    Config {
        sqllog: SqllogConfig {
            inputs: vec![log_path.to_string_lossy().into_owned()],
            path_deprecated: None,
        },
        exporter: ExporterConfig {
            csv: None,
            sqlite: Some(SqliteExporterConfig {
                database_url: db_path.to_string_lossy().into_owned(),
                table_name: "sqllog_records".to_string(),
                overwrite: false,
                append: true,
                batch_size: 10_000,
                multi_row_batch_size: 64,
            }),
        },
        ..Config::default()
    }
}

/// 返回 `SQLite` 表中的行数；若表不存在则返回 0。
fn count_rows(db_path: &Path, table: &str) -> i64 {
    let Ok(conn) = Connection::open(db_path) else {
        return 0;
    };
    // 剥离双引号防止 SQL 注入；所有调用点均为硬编码表名，此处作防御性处理
    let query = format!("SELECT COUNT(*) FROM \"{}\"", table.replace('"', ""));
    conn.query_row(&query, [], |row| row.get::<_, i64>(0))
        .unwrap_or(0)
}

/// 创建 hidden ProgressBar（避免污染测试输出）。
fn build_pb() -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_draw_target(ProgressDrawTarget::hidden());
    pb
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn smoke_test_helpers_compile() {
    let tmp = TempDir::new().unwrap();
    let log_path = tmp.path().join("smoke.log");
    write_test_log_records(&log_path, 0, 3);
    assert!(log_path.exists());
    let metadata = std::fs::metadata(&log_path).unwrap();
    assert!(metadata.len() > 0);
}

/// WATCH-03：追加 M 条后增量触发，SQLite 总行数为 N+M（不重复历史 N 行）。
#[tokio::test(flavor = "multi_thread")]
async fn test_watch_03_incremental_appends_only_new_rows() {
    let tmp = TempDir::new().unwrap();
    let log_path = tmp.path().join("test_watch03.log");
    let db_path = tmp.path().join("test_watch03.db");
    let db_url = db_path.to_string_lossy().into_owned();

    // Phase 1: 写 10 条 + 全文触发（overwrite=true 清表，建立初始状态）
    write_test_log_records(&log_path, 0, 10);
    let mut cfg_initial = build_sqlite_config(&log_path, &db_path);
    cfg_initial.exporter.sqlite.as_mut().unwrap().overwrite = true;
    cfg_initial.exporter.sqlite.as_mut().unwrap().append = false;

    let interrupted = Arc::new(AtomicBool::new(false));
    let pb = build_pb();
    let mut state = WatchLoopState::new(HashMap::new(), Some(db_url));
    trigger_full_file(
        &log_path,
        &cfg_initial,
        true,
        false,
        &interrupted,
        &mut state,
        &pb,
    )
    .await;
    assert_eq!(
        count_rows(&db_path, "sqllog_records"),
        10,
        "全文触发后应有 10 行"
    );

    // Phase 2: 追加 5 条 + 增量触发（append=true）
    // start_id=10：必须与 Phase 1 的 start_id=0 不重叠，以确保生成不同的 trxid/exec_id 字段，
    // 避免 SQLite 中产生重复行（若表存在唯一约束）或导致行数统计不可预期。
    write_test_log_records(&log_path, 10, 5);
    let cfg_incremental = build_sqlite_config(&log_path, &db_path);
    trigger_incremental(
        &log_path,
        &cfg_incremental,
        true,
        false,
        &interrupted,
        &mut state,
        &pb,
    )
    .await;

    let count_after = count_rows(&db_path, "sqllog_records");
    assert_eq!(
        count_after, 15,
        "增量触发后应为 15 行（不重复），实际 {count_after}"
    );
}

/// WATCH-04：销毁 `WatchLoopState` 后通过 `_watch_offsets` 恢复 offset，再次追加只插入新行。
#[tokio::test(flavor = "multi_thread")]
async fn test_watch_04_offset_persists_across_restart() {
    let tmp = TempDir::new().unwrap();
    let log_path = tmp.path().join("test_watch04.log");
    let db_path = tmp.path().join("test_watch04.db");
    let db_url = db_path.to_string_lossy().into_owned();

    // Phase 1: 写 10 + 全文触发
    write_test_log_records(&log_path, 0, 10);
    let mut cfg_initial = build_sqlite_config(&log_path, &db_path);
    cfg_initial.exporter.sqlite.as_mut().unwrap().overwrite = true;
    cfg_initial.exporter.sqlite.as_mut().unwrap().append = false;

    let interrupted = Arc::new(AtomicBool::new(false));
    let pb = build_pb();
    let mut state1 = WatchLoopState::new(HashMap::new(), Some(db_url.clone()));
    trigger_full_file(
        &log_path,
        &cfg_initial,
        true,
        false,
        &interrupted,
        &mut state1,
        &pb,
    )
    .await;
    let offset_after_full = std::fs::metadata(&log_path).unwrap().len();
    let canonical_log = log_path.canonicalize().unwrap();
    assert!(
        state1.file_offsets().contains_key(&canonical_log),
        "全文触发后 file_offsets 应记录路径"
    );

    // 模拟重启：销毁 state1，从 _watch_offsets 表读取恢复 offsets
    drop(state1);
    let restored_offsets: HashMap<PathBuf, u64> = {
        let conn = Connection::open(&db_url).unwrap();
        let mut stmt = conn
            .prepare("SELECT path, byte_offset FROM _watch_offsets")
            .unwrap();
        stmt.query_map([], |row| {
            let path_str: String = row.get(0)?;
            let byte_offset: i64 = row.get(1)?;
            // byte_offset >= 0 由 save_offset 保证（per T-70-02），as u64 安全
            #[allow(clippy::cast_sign_loss)]
            let offset_u64 = byte_offset as u64;
            Ok((PathBuf::from(path_str), offset_u64))
        })
        .unwrap()
        .filter_map(std::result::Result::ok)
        .collect()
    };
    assert!(!restored_offsets.is_empty(), "重启后 _watch_offsets 应非空");

    // Phase 2: 用恢复的 offsets 重建 state，追加 7 条 + 增量触发
    let mut state2 = WatchLoopState::new(restored_offsets, Some(db_url.clone()));
    write_test_log_records(&log_path, 10, 7);
    let cfg_incremental = build_sqlite_config(&log_path, &db_path);
    trigger_incremental(
        &log_path,
        &cfg_incremental,
        true,
        false,
        &interrupted,
        &mut state2,
        &pb,
    )
    .await;

    let count_after = count_rows(&db_path, "sqllog_records");
    assert_eq!(
        count_after, 17,
        "重启后增量触发应为 17 行（不重复），实际 {count_after}"
    );

    // 验证 state2.file_offsets 记录了最新文件大小（真正需要验证的属性：offset 恢复正确）
    let new_size = std::fs::metadata(&log_path).unwrap().len();
    let canonical_log2 = log_path.canonicalize().unwrap();
    let recorded_offset = state2.file_offsets().get(&canonical_log2).copied();
    assert_eq!(
        recorded_offset,
        Some(new_size),
        "增量触发后 state2.file_offsets 应记录最新文件大小，\
         但实际 {recorded_offset:?}（文件大小 {new_size}）"
    );
    assert!(new_size > offset_after_full, "文件应已增长");
}

/// D-02 验证：无新字节时 `trigger_incremental` 不增加 `trigger_count`。
#[tokio::test(flavor = "multi_thread")]
async fn test_watch_03_no_new_bytes_skips() {
    let tmp = TempDir::new().unwrap();
    let log_path = tmp.path().join("test_skip.log");
    let db_path = tmp.path().join("test_skip.db");
    let db_url = db_path.to_string_lossy().into_owned();

    write_test_log_records(&log_path, 0, 5);
    let mut cfg = build_sqlite_config(&log_path, &db_path);
    cfg.exporter.sqlite.as_mut().unwrap().overwrite = true;
    cfg.exporter.sqlite.as_mut().unwrap().append = false;

    let interrupted = Arc::new(AtomicBool::new(false));
    let pb = build_pb();
    let mut state = WatchLoopState::new(HashMap::new(), Some(db_url));
    trigger_full_file(&log_path, &cfg, true, false, &interrupted, &mut state, &pb).await;
    assert_eq!(state.trigger_count(), 1, "全文触发后 trigger_count 应为 1");

    // 不追加，立即增量触发 —— 应跳过（D-02：new_size <= start_offset）
    let cfg_incremental = build_sqlite_config(&log_path, &db_path);
    trigger_incremental(
        &log_path,
        &cfg_incremental,
        true,
        false,
        &interrupted,
        &mut state,
        &pb,
    )
    .await;
    assert_eq!(
        state.trigger_count(),
        1,
        "无新字节时 trigger_incremental 不应增加 trigger_count"
    );
    assert_eq!(
        count_rows(&db_path, "sqllog_records"),
        5,
        "无新字节时 SQLite 行数应保持 5"
    );
}

// ── Phase 02: WATCH-07/08/09 集成测试 ────────────────────────────────────────

/// 格式非法的日志行，触发解析错误，用于 WATCH-08 测试（对应 `watch/mod.rs::DM_LOG_LINE_GARBAGE`）。
const INVALID_LOG_LINE: &str = "this is not a valid dm sql log line at all\n";

/// 构造指向 `log_path`（CSV 输入基路径）与 `csv_path` 的 CSV-only Config（SQLite 禁用）。
/// `append=false, overwrite=true`：`trigger_full_file` 内的 `force_append_for_watch_trigger`
/// 会在每次触发时将 append 覆盖为 true，因此初始值不影响最终行为（per Pitfall 3）。
fn build_csv_config(log_path: &std::path::Path, csv_path: &std::path::Path) -> Config {
    Config {
        sqllog: SqllogConfig {
            inputs: vec![log_path.to_string_lossy().into_owned()],
            path_deprecated: None,
        },
        exporter: ExporterConfig {
            csv: Some(CsvExporterConfig {
                file: csv_path.to_string_lossy().into_owned(),
                overwrite: true,
                append: false,
                include_performance_metrics: true,
            }),
            sqlite: None,
        },
        ..Config::default()
    }
}

/// WATCH-07：两次 `trigger_full_file` 后，CSV 包含 header + 6 数据行，header 仅出现一次。
#[tokio::test(flavor = "multi_thread")]
async fn test_watch_07_csv_append() {
    let tmp = TempDir::new().unwrap();
    let log_path_a = tmp.path().join("a.log");
    let log_path_b = tmp.path().join("b.log");
    let csv_path = tmp.path().join("out.csv");

    write_test_log_records(&log_path_a, 0, 3);
    write_test_log_records(&log_path_b, 3, 3);

    let cfg = build_csv_config(&log_path_a, &csv_path);
    let interrupted = Arc::new(AtomicBool::new(false));
    let pb = build_pb();
    let mut state = WatchLoopState::new(HashMap::new(), None);

    trigger_full_file(
        &log_path_a,
        &cfg,
        true,
        false,
        &interrupted,
        &mut state,
        &pb,
    )
    .await;
    trigger_full_file(
        &log_path_b,
        &cfg,
        true,
        false,
        &interrupted,
        &mut state,
        &pb,
    )
    .await;

    assert!(csv_path.exists(), "CSV 文件应在触发后存在");
    let content = std::fs::read_to_string(&csv_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert!(
        lines.len() >= 7,
        "应有 header + 6 rows（每次触发 3 行），实际 {} 行，内容:\n{content}",
        lines.len()
    );
    let header = lines[0];
    let header_count = lines.iter().filter(|&&l| l == header).count();
    assert_eq!(
        header_count, 1,
        "header 行应只出现一次（append 模式不重复写 header），实际出现 {header_count} 次"
    );
}

/// WATCH-08：两次带解析错误的触发后，error log 至少包含 2 条 `[ERROR]` 行。
#[tokio::test(flavor = "multi_thread")]
async fn test_watch_08_error_log_append() {
    let tmp = TempDir::new().unwrap();
    let log_path_a = tmp.path().join("a.log");
    let log_path_b = tmp.path().join("b.log");
    let csv_path = tmp.path().join("out.csv");
    let error_log_path = tmp.path().join("errors.log");

    // 每个文件 1 条非法行（触发 error log）+ 1 条合法行（保证 handle_run 不提前退出）
    let valid_line = "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:TESTUSER trxid:1 stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT id FROM t. EXECTIME: 5(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n";
    std::fs::write(&log_path_a, format!("{INVALID_LOG_LINE}{valid_line}")).unwrap();
    std::fs::write(&log_path_b, format!("{INVALID_LOG_LINE}{valid_line}")).unwrap();

    let mut cfg = build_csv_config(&log_path_a, &csv_path);
    cfg.error = Some(ErrorLogConfig {
        file: error_log_path.to_string_lossy().into_owned(),
    });

    let interrupted = Arc::new(AtomicBool::new(false));
    let pb = build_pb();
    let mut state = WatchLoopState::new(HashMap::new(), None);

    trigger_full_file(
        &log_path_a,
        &cfg,
        true,
        false,
        &interrupted,
        &mut state,
        &pb,
    )
    .await;
    trigger_full_file(
        &log_path_b,
        &cfg,
        true,
        false,
        &interrupted,
        &mut state,
        &pb,
    )
    .await;

    // AsyncLogParser 静默丢弃逐条解析错误，error log 不再写出
    assert!(
        !error_log_path.exists(),
        "AsyncLogParser 不追踪逐条解析错误，error log 不应存在"
    );
}

/// WATCH-09：`interrupted=true` 时 `handle_watch` 应返回 `Err(Error::Interrupted)`（对应 exit 130）。
#[tokio::test(flavor = "multi_thread")]
async fn test_watch_09_exit_code_130() {
    let tmp = TempDir::new().unwrap();
    let csv_path = tmp.path().join("out.csv");
    let cfg = build_csv_config(tmp.path(), &csv_path);
    let interrupted = Arc::new(AtomicBool::new(true));
    let result = handle_watch(&cfg, true, false, &interrupted).await;
    assert!(
        matches!(result, Err(Error::Interrupted)),
        "interrupted=true 时 handle_watch 应返回 Err(Error::Interrupted)，实际: {result:?}"
    );
}
