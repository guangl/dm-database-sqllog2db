use super::*;
use dm_database_parser_sqllog::LogParser;

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

    let parser = LogParser::from_path(logfile.to_str().unwrap()).unwrap();
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();

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

    let parser = LogParser::from_path(logfile.to_str().unwrap()).unwrap();
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();

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

    let parser = LogParser::from_path(logfile.to_str().unwrap()).unwrap();
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();
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
        batch_size: 10_000,
    };
    let mut exporter = SqliteExporter::from_config(&cfg);
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

    let parser = LogParser::from_path(logfile.to_str().unwrap()).unwrap();
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();

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

    let parser = LogParser::from_path(logfile.to_str().unwrap()).unwrap();
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();

    {
        let mut exporter =
            SqliteExporter::new(dbfile.to_string_lossy().into(), "tbl".into(), true, false);
        exporter.initialize().unwrap();
        for r in &records {
            let meta = r.parse_meta();
            let pm = r.parse_performance_metrics();
            exporter.export_one_preparsed(r, &meta, &pm, None).unwrap();
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

    let parser = LogParser::from_path(logfile.to_str().unwrap()).unwrap();
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();

    let mut exporter =
        SqliteExporter::new(dbfile.to_string_lossy().into(), "tbl".into(), true, false);
    exporter.initialize().unwrap();
    for r in &records {
        exporter.export(r).unwrap();
    }
    let snap = exporter.stats_snapshot().unwrap();
    assert_eq!(snap.exported, 4);
    exporter.finalize().unwrap();
}

#[test]
fn test_sqlite_debug_format() {
    let exporter = SqliteExporter::new("/tmp/debug.db".to_string(), "tbl".to_string(), true, false);
    let s = format!("{exporter:?}");
    assert!(s.contains("SqliteExporter"));
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

        let parser = LogParser::from_path(log.to_str().unwrap()).unwrap();
        for record in parser.iter().flatten() {
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

    let parser = LogParser::from_path(logfile.to_str().unwrap()).unwrap();
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();

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

    let parser = LogParser::from_path(logfile.to_str().unwrap()).unwrap();
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();

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

    let parser = LogParser::from_path(logfile.to_str().unwrap()).unwrap();
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();

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
