use super::*;
use dm_database_parser_sqllog::LogParserBuilder;

fn write_test_log(path: &std::path::Path, count: usize) {
    use std::fmt::Write as _;
    let mut buf = String::with_capacity(count * 170);
    for i in 0..count {
        writeln!(
            buf,
            "2025-01-15 10:30:28.001 (EP[0] sess:0x{i:04x} user:TESTUSER trxid:{i} stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT * FROM t WHERE id={i}. EXECTIME: {exec}(ms) ROWCOUNT: {rows}(rows) EXEC_ID: {i}.",
            exec = (i * 13) % 1000,
            rows = i % 100,
        ).unwrap();
    }
    std::fs::write(path, buf).unwrap();
}

#[test]
fn test_sqlite_basic_export() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("test.log");
    let dbfile = dir.path().join("out.db");
    write_test_log(&logfile, 5);

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser
        .iter()
        .unwrap()
        .filter_map(std::result::Result::ok)
        .collect();

    {
        let mut exporter = SqliteExporter::new(
            dbfile.to_string_lossy().into(),
            "sqllog_records".into(),
            true,
            false,
        );
        exporter.initialize().unwrap();
        for r in &records {
            exporter.export_one_normalized(r, None).unwrap();
        }
        exporter.finalize().unwrap();
    } // exporter drops here, releasing EXCLUSIVE lock

    // Verify rows inserted
    let conn = rusqlite::Connection::open(&dbfile).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sqllog_records", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 5);
}

#[test]
fn test_sqlite_overwrite_drops_existing_table() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("test.log");
    let dbfile = dir.path().join("out.db");
    write_test_log(&logfile, 3);

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser
        .iter()
        .unwrap()
        .filter_map(std::result::Result::ok)
        .collect();

    // First run: insert 3 rows
    {
        let mut e =
            SqliteExporter::new(dbfile.to_string_lossy().into(), "tbl".into(), false, false);
        e.initialize().unwrap();
        for r in &records {
            e.export_one_normalized(r, None).unwrap();
        }
        e.finalize().unwrap();
    }

    // Second run with overwrite: should have only 3 rows again (not 6)
    {
        let mut e = SqliteExporter::new(dbfile.to_string_lossy().into(), "tbl".into(), true, false);
        e.initialize().unwrap();
        for r in &records {
            e.export_one_normalized(r, None).unwrap();
        }
        e.finalize().unwrap();
    }

    let conn = rusqlite::Connection::open(&dbfile).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tbl", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 3);
}

#[test]
fn test_sqlite_with_normalized() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("test.log");
    let dbfile = dir.path().join("out.db");
    write_test_log(&logfile, 2);

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser
        .iter()
        .unwrap()
        .filter_map(std::result::Result::ok)
        .collect();
    let normalized: Vec<Option<String>> = records
        .iter()
        .map(|_| Some("SELECT * FROM t WHERE id=?".into()))
        .collect();

    {
        let mut exporter =
            SqliteExporter::new(dbfile.to_string_lossy().into(), "tbl".into(), true, false);
        exporter.normalize = true;
        exporter.initialize().unwrap();
        for (r, ns) in records.iter().zip(normalized.iter()) {
            exporter.export_one_normalized(r, ns.as_deref()).unwrap();
        }
        exporter.finalize().unwrap();
    } // exporter drops here, releasing EXCLUSIVE lock

    let conn = rusqlite::Connection::open(&dbfile).unwrap();
    let ns: Option<String> = conn
        .query_row("SELECT normalized_sql FROM tbl LIMIT 1", [], |r| r.get(0))
        .unwrap();
    assert_eq!(ns, Some("SELECT * FROM t WHERE id=?".to_string()));
}

#[test]
fn test_sqlite_from_config() {
    let dir = tempfile::TempDir::new().unwrap();
    let dbfile = dir.path().join("cfg.db");
    let cfg = crate::config::SqliteExporterConfig {
        database_url: dbfile.to_string_lossy().into_owned(),
        table_name: "records".to_string(),
        overwrite: true,
        append: false,
        batch_size: 42_000,
        multi_row_batch_size: 32,
    };
    let mut exporter = SqliteExporter::from_config(&cfg);
    // 验证 from_config 正确映射 batch_size 字段（默认值是 10_000，这里用 42_000 区分）
    assert_eq!(
        exporter.batch_size, cfg.batch_size,
        "from_config must map batch_size correctly"
    );
    assert_eq!(
        exporter.multi_row_batch_size, cfg.multi_row_batch_size,
        "from_config must map multi_row_batch_size correctly"
    );
    exporter.initialize().unwrap();
    exporter.finalize().unwrap();
    assert!(dbfile.exists());
}

#[test]
fn test_sqlite_export_method() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("test.log");
    let dbfile = dir.path().join("export.db");
    write_test_log(&logfile, 3);

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser
        .iter()
        .unwrap()
        .filter_map(std::result::Result::ok)
        .collect();

    {
        let mut exporter =
            SqliteExporter::new(dbfile.to_string_lossy().into(), "tbl".into(), true, false);
        exporter.initialize().unwrap();
        for r in &records {
            // Use export() instead of export_one_normalized
            exporter.export(r).unwrap();
        }
        exporter.finalize().unwrap();
    }

    let conn = rusqlite::Connection::open(&dbfile).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tbl", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 3);
}

#[test]
fn test_sqlite_export_one_preparsed() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("test.log");
    let dbfile = dir.path().join("preparsed.db");
    write_test_log(&logfile, 2);

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser
        .iter()
        .unwrap()
        .filter_map(std::result::Result::ok)
        .collect();

    {
        let mut exporter =
            SqliteExporter::new(dbfile.to_string_lossy().into(), "tbl".into(), true, false);
        exporter.initialize().unwrap();
        for r in &records {
            exporter.export_one_preparsed(r, true, None).unwrap();
        }
        exporter.finalize().unwrap();
    }

    let conn = rusqlite::Connection::open(&dbfile).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tbl", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn test_sqlite_stats_snapshot() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("test.log");
    let dbfile = dir.path().join("stats.db");
    write_test_log(&logfile, 4);

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser
        .iter()
        .unwrap()
        .filter_map(std::result::Result::ok)
        .collect();

    let mut exporter =
        SqliteExporter::new(dbfile.to_string_lossy().into(), "tbl".into(), true, false);
    exporter.initialize().unwrap();
    for r in &records {
        exporter.export(r).unwrap();
    }
    // 4 条记录未满 batch_size（64），尚在缓冲区中，未写入 DB，exported 应为 0
    let snap_before = exporter.stats_snapshot().unwrap();
    assert_eq!(snap_before.exported, 0, "flush 前 exported 应为 0");
    exporter.finalize().unwrap();
    // finalize() 触发 flush，记录写入 DB 后才计入 exported
    let snap_after = exporter.stats_snapshot().unwrap();
    assert_eq!(snap_after.exported, 4);
}

#[test]
fn test_sqlite_debug_format() {
    let dir = tempfile::TempDir::new().unwrap();
    let dbfile = dir.path().join("debug.db");
    let exporter = SqliteExporter::new(
        dbfile.to_string_lossy().into_owned(),
        "my_table".to_string(),
        true,
        false,
    );
    let s = format!("{exporter:?}");
    assert!(
        s.contains("SqliteExporter"),
        "Debug output should contain struct name"
    );
    // 验证 Debug 输出包含关键字段，而非只检查结构名
    assert!(
        s.contains("my_table"),
        "Debug output should contain table_name, got: {s}"
    );
    assert!(
        s.contains("database_url"),
        "Debug output should contain database_url field, got: {s}"
    );
}

#[test]
fn test_sqlite_field_order() {
    use crate::pipeline::FieldMask;

    let dir = tempfile::TempDir::new().unwrap();
    let log = dir.path().join("t.log");
    std::fs::write(
        &log,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:testuser trxid:1 stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT 42. EXECTIME: 5(ms) ROWCOUNT: 2(rows) EXEC_ID: 99.\n",
    )
    .unwrap();

    let db = dir.path().join("out.db");
    {
        let mut exporter = SqliteExporter::new(
            db.to_str().unwrap().to_string(),
            "records".to_string(),
            true,
            false,
        );
        exporter.normalize = false;
        exporter.field_mask =
            FieldMask::from_names(&["sql".to_string(), "username".to_string()]).unwrap();
        exporter.ordered_indices = vec![10, 4]; // sql=10, username=4
        exporter.initialize().unwrap();

        let parser = LogParserBuilder::new(log.to_str().unwrap())
            .build()
            .unwrap();
        for record in parser.iter().unwrap().flatten() {
            exporter.export(&record).unwrap();
        }
        exporter.finalize().unwrap();
    } // exporter drops here, releasing EXCLUSIVE lock

    let conn = rusqlite::Connection::open(&db).unwrap();
    let (sql_val, username_val): (String, String) = conn
        .query_row("SELECT sql, username FROM records", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();

    assert!(sql_val.contains("SELECT 42"), "sql_val: {sql_val}");
    assert_eq!(username_val, "testuser");
}

#[test]
fn test_sqlite_append_mode() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("test.log");
    let dbfile = dir.path().join("append.db");
    write_test_log(&logfile, 3);

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser
        .iter()
        .unwrap()
        .filter_map(std::result::Result::ok)
        .collect();

    // First run: create table with 3 rows
    {
        let mut e =
            SqliteExporter::new(dbfile.to_string_lossy().into(), "tbl".into(), false, false);
        e.initialize().unwrap();
        for r in &records {
            e.export(r).unwrap();
        }
        e.finalize().unwrap();
    }

    // Second run with append=true: adds 3 more rows
    {
        let mut e = SqliteExporter::new(dbfile.to_string_lossy().into(), "tbl".into(), false, true);
        e.initialize().unwrap();
        for r in &records {
            e.export(r).unwrap();
        }
        e.finalize().unwrap();
    }

    let conn = rusqlite::Connection::open(&dbfile).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tbl", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 6);
}

#[test]
fn test_sqlite_initialize_creates_quoted_table() {
    let dir = tempfile::TempDir::new().unwrap();
    let dbfile = dir.path().join("quoted.db");
    {
        let mut exporter = SqliteExporter::new(
            dbfile.to_string_lossy().into(),
            "my_records".to_string(),
            true,
            false,
        );
        exporter.initialize().unwrap();
        exporter.finalize().unwrap();
    }
    let conn = rusqlite::Connection::open(&dbfile).unwrap();
    let create_stmt: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='my_records'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        create_stmt.contains("\"my_records\""),
        "table name should be double-quoted; actual: {create_stmt}"
    );
}

#[test]
fn test_sqlite_initialize_silent_when_table_missing() {
    let dir = tempfile::TempDir::new().unwrap();
    let dbfile = dir.path().join("missing_tbl.db");

    // overwrite=false 且 append=false → initialize 会走 DELETE FROM 分支
    // 由于 DB 全新，表不存在，DELETE 应触发 "no such table" 并被静默吃掉
    {
        let mut exporter = SqliteExporter::new(
            dbfile.to_string_lossy().into(),
            "fresh_tbl".to_string(),
            false,
            false,
        );
        // 必须 Ok —— 不能因 "no such table" 返回 Err
        exporter
            .initialize()
            .expect("initialize should silently succeed when table missing");
        exporter.finalize().unwrap();
    } // exporter drops here, releasing EXCLUSIVE lock

    // 表应已被 CREATE TABLE IF NOT EXISTS 创建
    let conn = rusqlite::Connection::open(&dbfile).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='fresh_tbl'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "table fresh_tbl should be created");
}

#[test]
fn test_sqlite_initialize_clears_existing_table_via_delete() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("test.log");
    let dbfile = dir.path().join("clear.db");
    write_test_log(&logfile, 4);

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser
        .iter()
        .unwrap()
        .filter_map(std::result::Result::ok)
        .collect();

    // 第一次 run：写入 4 条
    {
        let mut e = SqliteExporter::new(
            dbfile.to_string_lossy().into(),
            "clr_tbl".into(),
            false,
            false,
        );
        e.initialize().unwrap();
        for r in &records {
            e.export(r).unwrap();
        }
        e.finalize().unwrap();
    }

    // 第二次 run：overwrite=false、append=false → 走 DELETE FROM 清空已有数据
    // 然后写入同样 4 条 —— 期望最终行数 == 4 而非 8
    {
        let mut e = SqliteExporter::new(
            dbfile.to_string_lossy().into(),
            "clr_tbl".into(),
            false,
            false,
        );
        e.initialize().unwrap();
        for r in &records {
            e.export(r).unwrap();
        }
        e.finalize().unwrap();
    }

    let conn = rusqlite::Connection::open(&dbfile).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM clr_tbl", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        count, 4,
        "DELETE FROM should clear previous rows; got {count}"
    );
}

#[test]
fn test_sqlite_batch_commit() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("batch.log");
    let dbfile = dir.path().join("batch.db");
    write_test_log(&logfile, 5);

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser
        .iter()
        .unwrap()
        .filter_map(std::result::Result::ok)
        .collect();

    {
        let mut exporter =
            SqliteExporter::new(dbfile.to_string_lossy().into(), "tbl".into(), true, false);
        // batch_size=2：每 2 条触发一次中间 COMMIT（5 条 → 2 次中间 COMMIT，finalize 提交第5条）
        exporter.batch_size = 2;
        exporter.initialize().unwrap();
        for r in &records {
            exporter.export_one_normalized(r, None).unwrap();
        }
        exporter.finalize().unwrap();
    }

    let conn = rusqlite::Connection::open(&dbfile).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tbl", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        count, 5,
        "5 条记录经过批量提交后必须全部持久化，实际: {count}"
    );
}

// ---- 新增覆盖测试（Plan 02 Task 2）----

#[test]
fn test_sqlite_export_without_initialize_returns_err() {
    // 覆盖 mod.rs:209-212 conn=None 的 ok_or_else 分支 + db_err 调用
    let dir = tempfile::TempDir::new().unwrap();
    let dbfile = dir.path().join("no_init.db");
    let logfile = dir.path().join("t.log");
    std::fs::write(
        &logfile,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT 1. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n",
    )
    .unwrap();

    // 构造 exporter 但不调用 initialize()，conn = None
    let mut exporter = SqliteExporter::new(
        dbfile.to_string_lossy().into(),
        "tbl".to_string(),
        true,
        false,
    );

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    for record in parser.iter().unwrap().flatten() {
        let result = exporter.export(&record);
        assert!(result.is_err(), "未调用 initialize() 时 export 应返回 Err");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("not initialized"),
            "错误消息应含 'not initialized'，实际: {err_msg}"
        );
    }
}

#[test]
fn test_sqlite_export_one_normalized_without_initialize_returns_err() {
    // 覆盖 export_one_normalized → export_one_preparsed 未初始化路径
    let dir = tempfile::TempDir::new().unwrap();
    let dbfile = dir.path().join("no_init2.db");
    let logfile = dir.path().join("t.log");
    std::fs::write(
        &logfile,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT 1. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n",
    )
    .unwrap();

    let mut exporter = SqliteExporter::new(
        dbfile.to_string_lossy().into(),
        "tbl".to_string(),
        true,
        false,
    );

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    for record in parser.iter().unwrap().flatten() {
        let result = exporter.export_one_normalized(&record, Some("SELECT 1"));
        assert!(
            result.is_err(),
            "未初始化时 export_one_normalized 应返回 Err"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("not initialized"),
            "错误消息应含 'not initialized'，实际: {err_msg}"
        );
    }
}

#[test]
fn test_sqlite_initialize_succeeds_and_creates_db() {
    // 验证 initialize() 成功执行 initialize_pragmas + CREATE TABLE 并创建 DB 文件。
    // journal_mode=OFF 是会话级 pragma，不持久化到连接关闭后，所以在连接期间验证。
    let dir = tempfile::TempDir::new().unwrap();
    let dbfile = dir.path().join("pragma.db");

    {
        let mut exporter = SqliteExporter::new(
            dbfile.to_string_lossy().into(),
            "tbl".to_string(),
            true,
            false,
        );
        let result = exporter.initialize();
        assert!(
            result.is_ok(),
            "initialize() 应成功（initialize_pragmas 已正确执行），实际: {result:?}"
        );
        exporter.finalize().unwrap();
    } // exporter drop，释放 EXCLUSIVE 锁

    // DB 文件应存在且非空（pragma 和 CREATE TABLE 均已执行）
    assert!(dbfile.exists(), "DB 文件应在 initialize 后存在");
    assert!(
        dbfile.metadata().unwrap().len() > 0,
        "DB 文件不应为空（至少含 SQLite header）"
    );

    // 验证 tbl 表已创建（CREATE TABLE IF NOT EXISTS 由 initialize 执行）
    let conn = rusqlite::Connection::open(&dbfile).unwrap();
    let table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='tbl'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        table_count, 1,
        "initialize 应创建目标表 'tbl'，实际: {table_count}"
    );
}

#[test]
fn test_sqlite_projection_subset_export() {
    // 覆盖 export_one_preparsed 中 ordered_indices 字段投影非全量路径
    use crate::pipeline::FieldMask;

    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("t.log");
    let dbfile = dir.path().join("proj.db");
    write_test_log(&logfile, 3);

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser
        .iter()
        .unwrap()
        .filter_map(std::result::Result::ok)
        .collect();

    {
        let mut exporter = SqliteExporter::new(
            dbfile.to_string_lossy().into(),
            "proj_tbl".to_string(),
            true,
            false,
        );
        exporter.normalize = false;
        exporter.field_mask =
            FieldMask::from_names(&["ts".to_string(), "username".to_string(), "sql".to_string()])
                .unwrap();
        exporter.ordered_indices = vec![0, 4, 10]; // ts=0, username=4, sql=10
        exporter.initialize().unwrap();
        for r in &records {
            exporter.export(r).unwrap();
        }
        exporter.finalize().unwrap();
    } // exporter drop，释放 EXCLUSIVE 锁

    // 重新打开 DB 验证投影列结构
    let conn = rusqlite::Connection::open(&dbfile).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM proj_tbl", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 3, "应插入 3 条记录，实际: {count}");

    // 验证表只有 3 列（ts, username, sql）
    let col_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('proj_tbl')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(col_count, 3, "投影后表应有 3 列，实际: {col_count}");
}

// ---- multi-row batch INSERT 正确性测试（73-01 Task 2）----

fn parse_records(logfile: &std::path::Path) -> Vec<dm_database_parser_sqllog::Sqllog> {
    LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap()
        .iter()
        .unwrap()
        .filter_map(std::result::Result::ok)
        .collect()
}

fn count_rows_in_db(dbfile: &std::path::Path, table: &str) -> i64 {
    let conn = rusqlite::Connection::open(dbfile).unwrap();
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}

#[test]
fn test_sqlite_multi_row_basic() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("t.log");
    let dbfile = dir.path().join("out.db");
    write_test_log(&logfile, 100);
    let records = parse_records(&logfile);

    {
        let mut exporter =
            SqliteExporter::new(dbfile.to_string_lossy().into(), "tbl".into(), true, false);
        exporter.multi_row_batch_size = 64;
        exporter.initialize().unwrap();
        for r in &records {
            exporter.export_one_normalized(r, None).unwrap();
        }
        exporter.finalize().unwrap();
    }

    assert_eq!(count_rows_in_db(&dbfile, "tbl"), 100);
}

#[test]
fn test_sqlite_multi_row_partial_tail() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("t.log");
    let dbfile = dir.path().join("out.db");
    write_test_log(&logfile, 65);
    let records = parse_records(&logfile);

    {
        let mut exporter =
            SqliteExporter::new(dbfile.to_string_lossy().into(), "tbl".into(), true, false);
        exporter.multi_row_batch_size = 64;
        exporter.initialize().unwrap();
        for r in &records {
            exporter.export_one_normalized(r, None).unwrap();
        }
        exporter.finalize().unwrap();
    }

    assert_eq!(
        count_rows_in_db(&dbfile, "tbl"),
        65,
        "finalize 前 flush 应刷尾部 1 条"
    );
}

#[test]
fn test_sqlite_multi_row_empty_input() {
    let dir = tempfile::TempDir::new().unwrap();
    let dbfile = dir.path().join("out.db");

    {
        let mut exporter =
            SqliteExporter::new(dbfile.to_string_lossy().into(), "tbl".into(), true, false);
        exporter.multi_row_batch_size = 64;
        exporter.initialize().unwrap();
        exporter.finalize().unwrap();
    }

    assert_eq!(count_rows_in_db(&dbfile, "tbl"), 0);
}

#[test]
fn test_sqlite_multi_row_batch1_equals_single() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("t.log");
    let dbfile = dir.path().join("out.db");
    write_test_log(&logfile, 5);
    let records = parse_records(&logfile);

    {
        let mut exporter =
            SqliteExporter::new(dbfile.to_string_lossy().into(), "tbl".into(), true, false);
        exporter.multi_row_batch_size = 1;
        exporter.initialize().unwrap();
        for r in &records {
            exporter.export_one_normalized(r, None).unwrap();
        }
        exporter.finalize().unwrap();
    }

    assert_eq!(
        count_rows_in_db(&dbfile, "tbl"),
        5,
        "batch_size=1 应等价于改造前单行模式"
    );
}

#[test]
fn test_sqlite_multi_row_projection_equivalence() {
    use crate::pipeline::FieldMask;

    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("t.log");
    let dbfile_full = dir.path().join("full.db");
    let dbfile_proj = dir.path().join("proj.db");
    write_test_log(&logfile, 5);
    let records = parse_records(&logfile);

    // 全量路径
    {
        let mut exporter = SqliteExporter::new(
            dbfile_full.to_string_lossy().into(),
            "tbl".into(),
            true,
            false,
        );
        exporter.multi_row_batch_size = 4;
        exporter.initialize().unwrap();
        for r in &records {
            exporter.export_one_normalized(r, None).unwrap();
        }
        exporter.finalize().unwrap();
    }

    // 投影路径：ts(0)/username(4)/sql(10)
    {
        let mut exporter = SqliteExporter::new(
            dbfile_proj.to_string_lossy().into(),
            "tbl".into(),
            true,
            false,
        );
        exporter.multi_row_batch_size = 4;
        exporter.normalize = false;
        exporter.field_mask =
            FieldMask::from_names(&["ts".to_string(), "username".to_string(), "sql".to_string()])
                .unwrap();
        exporter.ordered_indices = vec![0, 4, 10];
        exporter.initialize().unwrap();
        for r in &records {
            exporter.export_one_normalized(r, None).unwrap();
        }
        exporter.finalize().unwrap();
    }

    let conn_full = rusqlite::Connection::open(&dbfile_full).unwrap();
    let conn_proj = rusqlite::Connection::open(&dbfile_proj).unwrap();

    for i in 0..5i64 {
        let (ts_full, username_full, sql_full): (String, String, String) = conn_full
            .query_row(
                "SELECT ts, username, sql FROM tbl ORDER BY rowid LIMIT 1 OFFSET ?1",
                [i],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        let (ts_proj, username_proj, sql_proj): (String, String, String) = conn_proj
            .query_row(
                "SELECT ts, username, sql FROM tbl ORDER BY rowid LIMIT 1 OFFSET ?1",
                [i],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(ts_full, ts_proj, "row {i}: ts mismatch");
        assert_eq!(username_full, username_proj, "row {i}: username mismatch");
        assert_eq!(sql_full, sql_proj, "row {i}: sql mismatch");
    }
}

#[test]
fn test_sqlite_multi_row_batch_commit_interaction() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("t.log");
    let dbfile = dir.path().join("out.db");
    write_test_log(&logfile, 10);
    let records = parse_records(&logfile);

    {
        let mut exporter =
            SqliteExporter::new(dbfile.to_string_lossy().into(), "tbl".into(), true, false);
        exporter.batch_size = 2;
        exporter.multi_row_batch_size = 4;
        exporter.initialize().unwrap();
        for r in &records {
            exporter.export_one_normalized(r, None).unwrap();
        }
        exporter.finalize().unwrap();
    }

    assert_eq!(
        count_rows_in_db(&dbfile, "tbl"),
        10,
        "batch_size=2 + multi_row_batch_size=4 交互，10 条记录应全部持久化"
    );
}
