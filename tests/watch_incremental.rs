//! Phase 70 集成测试 — WATCH-03（追加增量不重复）与 WATCH-04（重启 offset 恢复）。
//! 不依赖 notify watcher（FSEvents 不稳定，per Pitfall 6），直接调用 pub `trigger_*` 函数。

use dm_database_sqllog2db::cli::watch::{WatchLoopState, trigger_full_file, trigger_incremental};
use dm_database_sqllog2db::config::{Config, ExporterConfig, SqliteExporterConfig, SqllogConfig};
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
    let query = format!("SELECT COUNT(*) FROM \"{table}\"");
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
#[test]
fn test_watch_03_incremental_appends_only_new_rows() {
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
    );
    assert_eq!(
        count_rows(&db_path, "sqllog_records"),
        10,
        "全文触发后应有 10 行"
    );

    // Phase 2: 追加 5 条 + 增量触发（append=true）
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
    );

    let count_after = count_rows(&db_path, "sqllog_records");
    assert_eq!(
        count_after, 15,
        "增量触发后应为 15 行（不重复），实际 {count_after}"
    );
}

/// WATCH-04：销毁 `WatchLoopState` 后通过 `_watch_offsets` 恢复 offset，再次追加只插入新行。
#[test]
fn test_watch_04_offset_persists_across_restart() {
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
    );
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
    );

    let count_after = count_rows(&db_path, "sqllog_records");
    assert_eq!(
        count_after, 17,
        "重启后增量触发应为 17 行（不重复），实际 {count_after}"
    );
    let new_size = std::fs::metadata(&log_path).unwrap().len();
    assert!(new_size > offset_after_full, "文件应已增长");
}

/// D-02 验证：无新字节时 `trigger_incremental` 不增加 `trigger_count`。
#[test]
fn test_watch_03_no_new_bytes_skips() {
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
    trigger_full_file(&log_path, &cfg, true, false, &interrupted, &mut state, &pb);
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
    );
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
