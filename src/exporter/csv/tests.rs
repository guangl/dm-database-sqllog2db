use super::super::Exporter;
use super::CsvExporter;
use super::writer::write_csv_escaped;
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
fn test_csv_basic_export() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("test.log");
    let outfile = dir.path().join("out.csv");
    write_test_log(&logfile, 5);

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();
    assert!(!records.is_empty());

    let mut exporter = CsvExporter::new(&outfile);
    exporter.initialize().unwrap();
    for r in &records {
        exporter.export_one_normalized(r, None).unwrap();
    }
    exporter.finalize().unwrap();

    let content = std::fs::read_to_string(&outfile).unwrap();
    assert!(content.starts_with("ts,ep,"));
    assert!(content.contains("normalized_sql"));
    // Should have header + 5 data rows
    assert_eq!(content.lines().count(), 6);
}

#[test]
fn test_csv_no_normalize() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("test.log");
    let outfile = dir.path().join("out.csv");
    write_test_log(&logfile, 2);

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();

    let mut exporter = CsvExporter::new(&outfile);
    exporter.normalize = false;
    exporter.initialize().unwrap();
    for r in &records {
        exporter.export_one_normalized(r, None).unwrap();
    }
    exporter.finalize().unwrap();

    let content = std::fs::read_to_string(&outfile).unwrap();
    assert!(!content.contains("normalized_sql"));
}

#[test]
fn test_csv_export_with_normalized() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("test.log");
    let outfile = dir.path().join("out.csv");
    write_test_log(&logfile, 3);

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();

    let mut exporter = CsvExporter::new(&outfile);
    exporter.normalize = true;
    exporter.initialize().unwrap();
    for (i, r) in records.iter().enumerate() {
        let ns = format!("SELECT * FROM t WHERE id=?_{i}");
        exporter.export_one_normalized(r, Some(&ns)).unwrap();
    }
    exporter.finalize().unwrap();

    let content = std::fs::read_to_string(&outfile).unwrap();
    assert!(content.contains("SELECT * FROM t WHERE id=?_0"));
}

#[test]
fn test_csv_append_mode() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("test.log");
    let outfile = dir.path().join("out.csv");
    write_test_log(&logfile, 2);

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();

    // First write
    {
        let mut exporter = CsvExporter::from_config(&crate::config::CsvExporterConfig {
            file: outfile.to_string_lossy().into(),
            overwrite: true,
            append: false,
            ..crate::config::CsvExporterConfig::default()
        });
        exporter.initialize().unwrap();
        for r in &records {
            exporter.export_one_normalized(r, None).unwrap();
        }
        exporter.finalize().unwrap();
    }
    let first_count = std::fs::read_to_string(&outfile).unwrap().lines().count();

    // Append second write
    {
        let mut exporter = CsvExporter::from_config(&crate::config::CsvExporterConfig {
            file: outfile.to_string_lossy().into(),
            overwrite: false,
            append: true,
            ..crate::config::CsvExporterConfig::default()
        });
        exporter.initialize().unwrap();
        for r in &records {
            exporter.export_one_normalized(r, None).unwrap();
        }
        exporter.finalize().unwrap();
    }
    let second_count = std::fs::read_to_string(&outfile).unwrap().lines().count();
    // Append adds rows (no header on second write)
    assert!(second_count > first_count);
}

#[test]
fn test_csv_empty_export_is_noop() {
    let dir = tempfile::TempDir::new().unwrap();
    let outfile = dir.path().join("out.csv");
    let mut exporter = CsvExporter::new(&outfile);
    exporter.initialize().unwrap();
    exporter.finalize().unwrap();
    // Only header
    let content = std::fs::read_to_string(&outfile).unwrap();
    assert_eq!(content.lines().count(), 1);
}

#[test]
fn test_csv_debug_format() {
    let exporter = CsvExporter::new("/tmp/debug.csv");
    let s = format!("{exporter:?}");
    assert!(s.contains("CsvExporter"));
}

#[test]
fn test_csv_export_method() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("test.log");
    let outfile = dir.path().join("out.csv");
    write_test_log(&logfile, 3);

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();

    let mut exporter = CsvExporter::new(&outfile);
    exporter.initialize().unwrap();
    for r in &records {
        // Use export() directly instead of export_one_normalized
        exporter.export(r).unwrap();
    }
    exporter.finalize().unwrap();

    let lines = std::fs::read_to_string(&outfile).unwrap().lines().count();
    assert_eq!(lines, records.len() + 1); // data + header
}

#[test]
fn test_csv_stats_snapshot() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("test.log");
    let outfile = dir.path().join("out.csv");
    write_test_log(&logfile, 5);

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();

    let mut exporter = CsvExporter::new(&outfile);
    exporter.initialize().unwrap();
    for r in &records {
        exporter.export(r).unwrap();
    }
    let snap = exporter.stats_snapshot().unwrap();
    assert_eq!(snap.exported, 5);
    exporter.finalize().unwrap();
}

#[test]
fn test_write_csv_escaped_with_quotes() {
    // write_csv_escaped handles '"' characters by doubling them
    let mut buf = Vec::new();
    write_csv_escaped(&mut buf, b"say \"hello\"");
    assert_eq!(buf, b"say \"\"hello\"\"");
}

#[test]
fn test_write_csv_escaped_no_quotes() {
    let mut buf = Vec::new();
    write_csv_escaped(&mut buf, b"no quotes here");
    assert_eq!(buf, b"no quotes here");
}

#[test]
fn test_csv_from_config() {
    use crate::config;
    let cfg = config::CsvExporterConfig {
        file: "/tmp/cfg.csv".to_string(),
        overwrite: true,
        append: false,
        ..config::CsvExporterConfig::default()
    };
    let exporter = CsvExporter::from_config(&cfg);
    let s = format!("{exporter:?}");
    assert!(s.contains("CsvExporter"));
}

#[test]
fn test_csv_header_field_order() {
    use crate::pipeline::FieldMask;
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("out.csv");
    let mut exporter = CsvExporter::new(&path);
    exporter.field_mask =
        FieldMask::from_names(&["sql".to_string(), "username".to_string()]).unwrap();
    exporter.ordered_indices = vec![10, 4]; // sql=10, username=4
    exporter.initialize().unwrap();
    exporter.finalize().unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    let header_line = content.lines().next().unwrap();
    assert_eq!(header_line, "sql,username");
}

#[test]
fn test_csv_header_full_order() {
    use crate::pipeline::FIELD_NAMES;
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("out.csv");
    let mut exporter = CsvExporter::new(&path);
    exporter.normalize = true;
    // ordered_indices 默认全量 [0..14]，无需修改
    exporter.initialize().unwrap();
    exporter.finalize().unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    let header_line = content.lines().next().unwrap();
    let expected: Vec<&str> = FIELD_NAMES.to_vec();
    assert_eq!(header_line, expected.join(","));
}

#[test]
fn test_csv_header_no_normalized_sql_when_normalize_false() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("out.csv");
    let mut exporter = CsvExporter::new(&path);
    exporter.normalize = false;
    // ordered_indices 默认全量，但 idx=14 应被跳过
    exporter.initialize().unwrap();
    exporter.finalize().unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    let header_line = content.lines().next().unwrap();
    assert!(!header_line.contains("normalized_sql"));
    assert!(header_line.contains("sql")); // idx=10 的 "sql" 字段仍存在
}

#[test]
fn test_csv_field_order() {
    // 验证数据行按 ordered_indices=[10,4] 顺序输出（sql, username 两列）
    use crate::pipeline::FieldMask;

    let dir = tempfile::TempDir::new().unwrap();
    let log = dir.path().join("t.log");
    std::fs::write(
        &log,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:testuser trxid:1 stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT 1. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n",
    )
    .unwrap();

    let out = dir.path().join("out.csv");
    let mut exporter = CsvExporter::new(&out);
    exporter.normalize = false;
    exporter.field_mask =
        FieldMask::from_names(&["sql".to_string(), "username".to_string()]).unwrap();
    exporter.ordered_indices = vec![10, 4]; // sql=10, username=4
    exporter.initialize().unwrap();

    let parser = LogParserBuilder::new(log.to_str().unwrap())
        .build()
        .unwrap();
    for record in parser.iter().flatten() {
        exporter.export(&record).unwrap();
    }
    exporter.finalize().unwrap();

    let content = std::fs::read_to_string(&out).unwrap();
    let mut lines = content.lines();
    let header = lines.next().unwrap();
    let data = lines.next().unwrap();

    assert_eq!(header, "sql,username");
    // 数据行第一列是 sql 内容（含引号），第二列是 username=testuser
    assert!(data.ends_with(",testuser"), "data line: {data}");
}

#[test]
fn test_csv_field_order_normalized_sql_skipped_when_normalize_false() {
    use crate::pipeline::FieldMask;

    let dir = tempfile::TempDir::new().unwrap();
    let log = dir.path().join("t.log");
    std::fs::write(
        &log,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT 1. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n",
    )
    .unwrap();

    let out = dir.path().join("out.csv");
    let mut exporter = CsvExporter::new(&out);
    exporter.normalize = false;
    // ordered_indices 含 14（normalized_sql），但 normalize=false 时应跳过（D-03）
    exporter.ordered_indices = vec![10, 14]; // sql, normalized_sql（后者被跳过）
    exporter.field_mask = FieldMask::from_names(&["sql".to_string()]).unwrap();
    exporter.initialize().unwrap();

    let parser = LogParserBuilder::new(log.to_str().unwrap())
        .build()
        .unwrap();
    for record in parser.iter().flatten() {
        exporter.export(&record).unwrap();
    }
    exporter.finalize().unwrap();

    let content = std::fs::read_to_string(&out).unwrap();
    let header = content.lines().next().unwrap();
    // normalize=false 时 normalized_sql 不出现在 header 中
    assert!(!header.contains("normalized_sql"), "header: {header}");
}

#[test]
fn test_csv_reserve_boundary_short_sql() {
    // 回归：极短 SQL（10 字节级）触发 reserve 路径，输出格式必须完整
    let dir = tempfile::TempDir::new().unwrap();
    let log = dir.path().join("short.log");
    std::fs::write(
        &log,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT 1. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n",
    ).unwrap();

    let out = dir.path().join("out.csv");
    let mut exporter = CsvExporter::new(&out);
    exporter.initialize().unwrap();

    let parser = LogParserBuilder::new(log.to_str().unwrap())
        .build()
        .unwrap();
    for record in parser.iter().flatten() {
        exporter.export(&record).unwrap();
    }
    exporter.finalize().unwrap();

    let content = std::fs::read_to_string(&out).unwrap();
    // header + 1 data row
    assert_eq!(content.lines().count(), 2);
    let data = content.lines().nth(1).unwrap();
    assert!(data.contains("\"SELECT 1"), "data row: {data}");
}

#[test]
fn test_csv_reserve_boundary_long_sql() {
    // 回归：长 SQL（>2KB）触发 reserve 扩容路径，line_buf 容量正确扩展
    let dir = tempfile::TempDir::new().unwrap();
    let log = dir.path().join("long.log");
    let big_sql = "x".repeat(4096);
    let line = format!(
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT '{big_sql}'. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n"
    );
    std::fs::write(&log, &line).unwrap();

    let out = dir.path().join("out.csv");
    let mut exporter = CsvExporter::new(&out);
    exporter.initialize().unwrap();

    let parser = LogParserBuilder::new(log.to_str().unwrap())
        .build()
        .unwrap();
    for record in parser.iter().flatten() {
        exporter.export(&record).unwrap();
    }
    exporter.finalize().unwrap();

    let content = std::fs::read_to_string(&out).unwrap();
    assert_eq!(content.lines().count(), 2);
    // 长 SQL 在数据行内完整存在
    assert!(content.contains(&big_sql), "long SQL missing from output");
}

#[test]
fn test_csv_header_skips_pm_when_disabled() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("out.csv");
    let mut exporter = CsvExporter::new(&path);
    exporter.include_performance_metrics = false;
    exporter.initialize().unwrap();
    exporter.finalize().unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    let header = content.lines().next().unwrap();
    assert!(!header.contains("exec_time_ms"), "header: {header}");
    assert!(!header.contains("row_count"), "header: {header}");
    assert!(!header.contains("exec_id"), "header: {header}");
    assert!(header.contains("sql"), "sql column should remain");
}

#[test]
fn test_csv_data_row_skips_pm_when_disabled() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("test.log");
    let outfile = dir.path().join("out.csv");
    write_test_log(&logfile, 3);

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();

    let mut exporter = CsvExporter::new(&outfile);
    exporter.include_performance_metrics = false;
    exporter.initialize().unwrap();
    for r in &records {
        exporter.export(r).unwrap();
    }
    exporter.finalize().unwrap();

    let content = std::fs::read_to_string(&outfile).unwrap();
    let header = content.lines().next().unwrap();
    let header_cols = header.split(',').count();
    // 关闭性能指标后 header 列数 == 全量列数 - 3
    // 全量含 normalized_sql：15；关闭性能指标后剩 12 列
    assert_eq!(header_cols, 12, "header: {header}");
    // 数据行列数也应为 12（注意 SQL 列含双引号但不含逗号）
    for line in content.lines().skip(1) {
        let cols = line.split(',').count();
        assert_eq!(cols, 12, "data row: {line}");
    }
}

#[test]
fn test_csv_default_include_pm_true_keeps_existing_behavior() {
    // 默认（include_performance_metrics=true）输出与历史行为一致
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("test.log");
    let outfile = dir.path().join("out.csv");
    write_test_log(&logfile, 2);

    let parser = LogParserBuilder::new(logfile.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();

    let mut exporter = CsvExporter::new(&outfile);
    // 不显式设置 include_performance_metrics，应为默认 true
    exporter.initialize().unwrap();
    for r in &records {
        exporter.export(r).unwrap();
    }
    exporter.finalize().unwrap();

    let content = std::fs::read_to_string(&outfile).unwrap();
    let header = content.lines().next().unwrap();
    assert!(header.contains("exec_time_ms"));
    assert!(header.contains("row_count"));
    assert!(header.contains("exec_id"));
}

// ---- 新增覆盖测试（Plan 02 Task 1）----

#[test]
fn test_csv_all_zero_metrics_outputs_empty_columns() {
    // 覆盖 writer.rs:82 的 b",," 分支：has_metrics=false（exec_id=0 && exectime=0.0 && rowcount=0）
    use crate::pipeline::FieldMask;

    let dir = tempfile::TempDir::new().unwrap();
    let log = dir.path().join("zero.log");
    // EXECTIME: 0(ms) ROWCOUNT: 0(rows) EXEC_ID: 0 => has_metrics = false
    std::fs::write(
        &log,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT 1. EXECTIME: 0(ms) ROWCOUNT: 0(rows) EXEC_ID: 0.\n",
    )
    .unwrap();

    let out = dir.path().join("out.csv");
    let mut exporter = CsvExporter::new(&out);
    // 全量字段（FieldMask::ALL），走 writer.rs:46 全量路径
    exporter.field_mask = FieldMask::ALL;
    exporter.initialize().unwrap();

    let parser = LogParserBuilder::new(log.to_str().unwrap())
        .build()
        .unwrap();
    for record in parser.iter().flatten() {
        exporter.export(&record).unwrap();
    }
    exporter.finalize().unwrap();

    let content = std::fs::read_to_string(&out).unwrap();
    let data_row = content.lines().nth(1).unwrap();
    // has_metrics=false 时，exec_time_ms/row_count/exec_id 三列为空，输出 ",,"
    assert!(
        data_row.contains(",,"),
        "has_metrics=false 时数据行应含 ',,' 表示空指标列，实际: {data_row}"
    );
}

#[test]
fn test_csv_projection_subset_emits_only_requested_columns() {
    // 覆盖 writer.rs:93 非全量分支 + idx=0/1/2 match 分支
    use crate::pipeline::FieldMask;

    let dir = tempfile::TempDir::new().unwrap();
    let log = dir.path().join("t.log");
    write_test_log(&log, 3);

    let out = dir.path().join("out.csv");
    let mut exporter = CsvExporter::new(&out);
    exporter.normalize = false;
    // 设置三字段投影：ts(0), ep(1), sess_id(2)
    exporter.field_mask =
        FieldMask::from_names(&["ts".to_string(), "ep".to_string(), "sess_id".to_string()])
            .unwrap();
    exporter.ordered_indices = vec![0, 1, 2];
    exporter.initialize().unwrap();

    let parser = LogParserBuilder::new(log.to_str().unwrap())
        .build()
        .unwrap();
    for record in parser.iter().flatten() {
        exporter.export(&record).unwrap();
    }
    exporter.finalize().unwrap();

    let content = std::fs::read_to_string(&out).unwrap();
    let header = content.lines().next().unwrap();
    assert_eq!(
        header, "ts,ep,sess_id",
        "header 应只含投影字段，实际: {header}"
    );

    // 每行数据只有 3 列
    for (i, line) in content.lines().skip(1).enumerate() {
        let col_count = line.split(',').count();
        assert_eq!(col_count, 3, "数据行 {i} 应有 3 列，实际: {line}");
    }
}

#[test]
fn test_csv_projection_statement_appname_client_ip_tag() {
    // 覆盖 writer.rs idx=6/7/8/9 四条 match 分支
    use crate::pipeline::FieldMask;

    let dir = tempfile::TempDir::new().unwrap();
    let log = dir.path().join("t.log");
    std::fs::write(
        &log,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x99 appname:MyApp ip:10.0.0.1) [SEL] SELECT 1. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n",
    )
    .unwrap();

    let out = dir.path().join("out.csv");
    let mut exporter = CsvExporter::new(&out);
    exporter.normalize = false;
    exporter.field_mask = FieldMask::from_names(&[
        "statement".to_string(),
        "appname".to_string(),
        "client_ip".to_string(),
        "tag".to_string(),
    ])
    .unwrap();
    exporter.ordered_indices = vec![6, 7, 8, 9]; // statement/appname/client_ip/tag
    exporter.initialize().unwrap();

    let parser = LogParserBuilder::new(log.to_str().unwrap())
        .build()
        .unwrap();
    for record in parser.iter().flatten() {
        exporter.export(&record).unwrap();
    }
    exporter.finalize().unwrap();

    let content = std::fs::read_to_string(&out).unwrap();
    let header = content.lines().next().unwrap();
    assert_eq!(
        header, "statement,appname,client_ip,tag",
        "header 应含 statement/appname/client_ip/tag，实际: {header}"
    );
    let data = content.lines().nth(1).unwrap();
    // appname 字段应含 MyApp
    assert!(
        data.contains("MyApp"),
        "数据行应含 appname=MyApp，实际: {data}"
    );
    // client_ip 字段应含 10.0.0.1
    assert!(
        data.contains("10.0.0.1"),
        "数据行应含 client_ip=10.0.0.1，实际: {data}"
    );
}

#[test]
fn test_csv_projection_zero_metrics_skips_idx_11_12_13() {
    // 覆盖 writer.rs:161/172/183 has_metrics=false 时不写 itoa 的分支（投影路径）
    use crate::pipeline::FieldMask;

    let dir = tempfile::TempDir::new().unwrap();
    let log = dir.path().join("zero.log");
    // EXEC_ID=0, EXECTIME=0, ROWCOUNT=0 => has_metrics = false
    std::fs::write(
        &log,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT 1. EXECTIME: 0(ms) ROWCOUNT: 0(rows) EXEC_ID: 0.\n",
    )
    .unwrap();

    let out = dir.path().join("out.csv");
    let mut exporter = CsvExporter::new(&out);
    exporter.normalize = false;
    // 仅投影性能指标三列 idx=11/12/13
    exporter.field_mask = FieldMask::from_names(&[
        "exec_time_ms".to_string(),
        "row_count".to_string(),
        "exec_id".to_string(),
    ])
    .unwrap();
    exporter.ordered_indices = vec![11, 12, 13];
    exporter.initialize().unwrap();

    let parser = LogParserBuilder::new(log.to_str().unwrap())
        .build()
        .unwrap();
    for record in parser.iter().flatten() {
        exporter.export(&record).unwrap();
    }
    exporter.finalize().unwrap();

    let content = std::fs::read_to_string(&out).unwrap();
    let data_row = content.lines().nth(1).unwrap();
    // has_metrics=false 时投影路径下三列均为空，数据行应为 ",,"
    assert_eq!(
        data_row, ",,",
        "has_metrics=false 时投影路径下三列均应为空，实际: {data_row}"
    );
}

#[test]
fn test_csv_projection_with_normalize_idx_14() {
    // 覆盖 writer.rs:187-194 idx=14 normalize=true 分支
    use crate::pipeline::FieldMask;

    let dir = tempfile::TempDir::new().unwrap();
    let log = dir.path().join("t.log");
    std::fs::write(
        &log,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT * FROM t WHERE id=1. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n",
    )
    .unwrap();

    let out = dir.path().join("out.csv");
    let mut exporter = CsvExporter::new(&out);
    exporter.normalize = true;
    // 仅投影 normalized_sql 列（idx=14）
    exporter.field_mask = FieldMask::from_names(&["normalized_sql".to_string()]).unwrap();
    exporter.ordered_indices = vec![14];
    exporter.initialize().unwrap();

    let normalized_sql = "SELECT * FROM t WHERE id=?";
    let parser = LogParserBuilder::new(log.to_str().unwrap())
        .build()
        .unwrap();
    for record in parser.iter().flatten() {
        exporter
            .export_one_normalized(&record, Some(normalized_sql))
            .unwrap();
    }
    exporter.finalize().unwrap();

    let content = std::fs::read_to_string(&out).unwrap();
    let header = content.lines().next().unwrap();
    assert_eq!(
        header, "normalized_sql",
        "header 应为 normalized_sql，实际: {header}"
    );
    let data = content.lines().nth(1).unwrap();
    assert!(
        data.contains(normalized_sql),
        "数据行应含 normalized SQL，实际: {data}"
    );
}

#[test]
fn test_csv_projection_with_normalize_none_emits_empty_idx_14() {
    // 覆盖 writer.rs:189-193 normalized_sql=None 时 idx=14 输出空的子分支
    use crate::pipeline::FieldMask;

    let dir = tempfile::TempDir::new().unwrap();
    let log = dir.path().join("t.log");
    std::fs::write(
        &log,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT 1. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n",
    )
    .unwrap();

    let out = dir.path().join("out.csv");
    let mut exporter = CsvExporter::new(&out);
    exporter.normalize = true;
    // 仅投影 normalized_sql 列（idx=14），但传入 None
    exporter.field_mask = FieldMask::from_names(&["normalized_sql".to_string()]).unwrap();
    exporter.ordered_indices = vec![14];
    exporter.initialize().unwrap();

    let parser = LogParserBuilder::new(log.to_str().unwrap())
        .build()
        .unwrap();
    for record in parser.iter().flatten() {
        // 传入 None，期望 idx=14 列为空
        exporter.export_one_normalized(&record, None).unwrap();
    }
    exporter.finalize().unwrap();

    let content = std::fs::read_to_string(&out).unwrap();
    let data_row = content.lines().nth(1).unwrap();
    // normalized_sql=None 时列应为空字符串（数据行为空行）
    assert_eq!(
        data_row, "",
        "normalized_sql=None 时数据行应为空，实际: {data_row}"
    );
}
