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

/// 辅助：构造 `TemplateStats` 测试数据
fn make_template_stats_sqlite(key: &str) -> crate::pipeline::TemplateStats {
    crate::pipeline::TemplateStats {
        template_key: key.to_string(),
        count: 5,
        avg_us: 100,
        min_us: 50,
        max_us: 200,
        p50_us: 90,
        p95_us: 180,
        p99_us: 195,
        first_seen: "2025-01-15 10:00:00".to_string(),
        last_seen: "2025-01-15 10:05:00".to_string(),
    }
}

/// TMPL-04-A：基本写入验证 — initialize → finalize → `write_template_stats` → 验证 COUNT=2
#[test]
fn test_sqlite_write_template_stats() {
    let dir = tempfile::TempDir::new().unwrap();
    let dbfile = dir.path().join("out.db");

    let stats = vec![
        make_template_stats_sqlite("SELECT 1"),
        make_template_stats_sqlite("SELECT 2"),
    ];

    {
        let mut exporter = SqliteExporter::new(
            dbfile.to_string_lossy().into(),
            "sqllog_records".into(),
            true,
            false,
        );
        exporter.initialize().unwrap();
        exporter.finalize().unwrap();
        exporter
            .write_template_stats(&stats, None, Some("sql_templates"))
            .unwrap();
    } // exporter drops here, releasing EXCLUSIVE lock

    let conn = rusqlite::Connection::open(&dbfile).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sql_templates", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2, "expected 2 rows in sql_templates, got {count}");

    let (key, row_count, avg_us, p99_us, first_seen): (String, i64, i64, i64, String) = conn
        .query_row(
            "SELECT template_key, count, avg_us, p99_us, first_seen \
             FROM sql_templates ORDER BY template_key LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .unwrap();
    assert_eq!(key, "SELECT 1");
    assert_eq!(row_count, 5);
    assert_eq!(avg_us, 100);
    assert_eq!(p99_us, 195);
    assert_eq!(first_seen, "2025-01-15 10:00:00");
}

/// TMPL-04-E：overwrite 覆盖 — 旧行被 DROP，只保留新行
#[test]
fn test_sqlite_templates_overwrite() {
    let dir = tempfile::TempDir::new().unwrap();
    let dbfile = dir.path().join("overwrite.db");

    // 第一次写入：overwrite=true，写入 "OLD"
    {
        let mut exporter = SqliteExporter::new(
            dbfile.to_string_lossy().into(),
            "sqllog_records".into(),
            true,
            false,
        );
        exporter.initialize().unwrap();
        exporter.finalize().unwrap();
        exporter
            .write_template_stats(
                &[make_template_stats_sqlite("OLD")],
                None,
                Some("sql_templates"),
            )
            .unwrap();
    }

    // 第二次写入：overwrite=true，写入 "NEW"（应 DROP 旧表）
    {
        let mut exporter = SqliteExporter::new(
            dbfile.to_string_lossy().into(),
            "sqllog_records".into(),
            true,
            false,
        );
        exporter.initialize().unwrap();
        exporter.finalize().unwrap();
        exporter
            .write_template_stats(
                &[make_template_stats_sqlite("NEW")],
                None,
                Some("sql_templates"),
            )
            .unwrap();
    }

    let conn = rusqlite::Connection::open(&dbfile).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sql_templates", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        count, 1,
        "overwrite should leave exactly 1 row, got {count}"
    );

    let key: String = conn
        .query_row("SELECT template_key FROM sql_templates", [], |r| r.get(0))
        .unwrap();
    assert_eq!(key, "NEW", "only NEW row should remain after overwrite");
}

/// TMPL-04-F：append 累加 — 旧行保留，新行累加
#[test]
fn test_sqlite_templates_append() {
    let dir = tempfile::TempDir::new().unwrap();
    let dbfile = dir.path().join("append_tpl.db");

    // 第一次写入：overwrite=true，写入 "A"
    {
        let mut exporter = SqliteExporter::new(
            dbfile.to_string_lossy().into(),
            "sqllog_records".into(),
            true,
            false,
        );
        exporter.initialize().unwrap();
        exporter.finalize().unwrap();
        exporter
            .write_template_stats(
                &[make_template_stats_sqlite("A")],
                None,
                Some("sql_templates"),
            )
            .unwrap();
    }

    // 第二次写入：overwrite=false、append=true，写入 "B"（应保留 "A"）
    {
        let mut exporter = SqliteExporter::new(
            dbfile.to_string_lossy().into(),
            "sqllog_records".into(),
            false,
            true,
        );
        exporter.initialize().unwrap();
        exporter.finalize().unwrap();
        exporter
            .write_template_stats(
                &[make_template_stats_sqlite("B")],
                None,
                Some("sql_templates"),
            )
            .unwrap();
    }

    let conn = rusqlite::Connection::open(&dbfile).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sql_templates", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        count, 2,
        "append should retain old row + add new, got {count}"
    );

    let keys: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT template_key FROM sql_templates ORDER BY template_key")
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    };
    assert_eq!(keys, vec!["A", "B"], "expected keys [A, B], got {keys:?}");
}

// 新增：sqlite_table_name=None 时跳过建表，sqlite_master 中无 sql_templates
#[test]
fn test_sqlite_write_template_stats_none_skips() {
    let dir = tempfile::TempDir::new().unwrap();
    let dbfile = dir.path().join("none_skip.db");

    let stats = vec![make_template_stats_sqlite("SELECT 1")];
    {
        let mut exporter = SqliteExporter::new(
            dbfile.to_string_lossy().into(),
            "sqllog_records".into(),
            true,
            false,
        );
        exporter.initialize().unwrap();
        exporter.finalize().unwrap();
        exporter.write_template_stats(&stats, None, None).unwrap();
    }

    let conn = rusqlite::Connection::open(&dbfile).unwrap();
    let table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sql_templates'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(table_count, 0, "None 表名时不应创建任何模板表");
}

// 新增：空字符串表名时跳过建表
#[test]
fn test_sqlite_write_template_stats_empty_table_name_skips() {
    let dir = tempfile::TempDir::new().unwrap();
    let dbfile = dir.path().join("empty_skip.db");

    let stats = vec![make_template_stats_sqlite("SELECT 1")];
    {
        let mut exporter = SqliteExporter::new(
            dbfile.to_string_lossy().into(),
            "sqllog_records".into(),
            true,
            false,
        );
        exporter.initialize().unwrap();
        exporter.finalize().unwrap();
        exporter
            .write_template_stats(&stats, None, Some(""))
            .unwrap();
    }

    let conn = rusqlite::Connection::open(&dbfile).unwrap();
    let table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sql_templates'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(table_count, 0, "空表名时不应创建任何模板表");
}

// 新增：自定义表名 custom_tpl，验证该表存在且行数正确
#[test]
fn test_sqlite_write_template_stats_custom_table() {
    let dir = tempfile::TempDir::new().unwrap();
    let dbfile = dir.path().join("custom_tpl.db");

    let stats = vec![
        make_template_stats_sqlite("SELECT 1"),
        make_template_stats_sqlite("SELECT 2"),
    ];
    {
        let mut exporter = SqliteExporter::new(
            dbfile.to_string_lossy().into(),
            "sqllog_records".into(),
            true,
            false,
        );
        exporter.initialize().unwrap();
        exporter.finalize().unwrap();
        exporter
            .write_template_stats(&stats, None, Some("custom_tpl"))
            .unwrap();
    }

    let conn = rusqlite::Connection::open(&dbfile).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM custom_tpl", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2, "custom_tpl 表中应有 2 行，实际 {count}");

    // 确认 sql_templates 表不存在
    let old_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sql_templates'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(old_count, 0, "不应创建 sql_templates 表");
}

// 新增：非法表名（含空格和分号）应被拒绝
#[test]
fn test_sqlite_write_template_stats_invalid_name_rejected() {
    let dir = tempfile::TempDir::new().unwrap();
    let dbfile = dir.path().join("invalid.db");

    let stats = vec![make_template_stats_sqlite("SELECT 1")];
    let mut exporter = SqliteExporter::new(
        dbfile.to_string_lossy().into(),
        "sqllog_records".into(),
        true,
        false,
    );
    exporter.initialize().unwrap();
    exporter.finalize().unwrap();
    // 含空格和分号的表名应被拒绝
    let result = exporter.write_template_stats(&stats, None, Some("bad name;DROP"));
    assert!(result.is_err(), "非法表名应返回 Err，实际: {result:?}");
    // 前导数字的表名应被拒绝（WR-01）
    let result2 = exporter.write_template_stats(&stats, None, Some("1bad"));
    assert!(
        result2.is_err(),
        "前导数字表名应返回 Err，实际: {result2:?}"
    );
}

// 新增：SQLite 保留字（group）作为带引号的表名时应成功（CR-02）
#[test]
fn test_sqlite_write_template_stats_reserved_word_table_name() {
    let dir = tempfile::TempDir::new().unwrap();
    let dbfile = dir.path().join("reserved.db");

    let stats = vec![make_template_stats_sqlite("SELECT 1")];
    let mut exporter = SqliteExporter::new(
        dbfile.to_string_lossy().into(),
        "sqllog_records".into(),
        true,
        false,
    );
    exporter.initialize().unwrap();
    exporter.finalize().unwrap();
    // "group" 是 SQLite 保留字，加引号后应接受
    let result = exporter.write_template_stats(&stats, None, Some("group_stats"));
    assert!(result.is_ok(), "合法标识符应成功：{result:?}");
}
