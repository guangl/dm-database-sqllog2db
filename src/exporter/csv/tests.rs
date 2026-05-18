use super::super::Exporter;
use super::CsvExporter;
use super::writer::write_csv_escaped;
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
fn test_csv_basic_export() {
    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("test.log");
    let outfile = dir.path().join("out.csv");
    write_test_log(&logfile, 5);

    let parser = LogParser::from_path(logfile.to_str().unwrap()).unwrap();
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

    let parser = LogParser::from_path(logfile.to_str().unwrap()).unwrap();
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

    let parser = LogParser::from_path(logfile.to_str().unwrap()).unwrap();
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

    let parser = LogParser::from_path(logfile.to_str().unwrap()).unwrap();
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

    let parser = LogParser::from_path(logfile.to_str().unwrap()).unwrap();
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

    let parser = LogParser::from_path(logfile.to_str().unwrap()).unwrap();
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

    let parser = LogParser::from_path(log.to_str().unwrap()).unwrap();
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

    let parser = LogParser::from_path(log.to_str().unwrap()).unwrap();
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

    let parser = LogParser::from_path(log.to_str().unwrap()).unwrap();
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

    let parser = LogParser::from_path(log.to_str().unwrap()).unwrap();
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

    let parser = LogParser::from_path(logfile.to_str().unwrap()).unwrap();
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

    let parser = LogParser::from_path(logfile.to_str().unwrap()).unwrap();
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

/// TMPL-04-B：验证 `write_template_stats` 写入指定路径，含 CSV 转义
#[test]
fn test_csv_write_template_stats() {
    let dir = tempfile::TempDir::new().unwrap();
    let outfile = dir.path().join("output.csv");
    let companion = dir.path().join("out_templates.csv");
    let companion_str = companion.to_string_lossy().into_owned();

    let mut exporter = CsvExporter::new(&outfile);
    exporter.initialize().unwrap();
    exporter.finalize().unwrap();

    // 第一个 template_key 含逗号，需转义
    let stats = vec![
        crate::pipeline::TemplateStats {
            template_key: r#"SELECT * FROM t WHERE name = "John", age = ?"#.to_string(),
            count: 42,
            avg_us: 150,
            min_us: 10,
            max_us: 500,
            p50_us: 120,
            p95_us: 400,
            p99_us: 480,
            first_seen: "2025-01-01 00:00:00".to_string(),
            last_seen: "2025-01-01 12:00:00".to_string(),
        },
        crate::pipeline::TemplateStats {
            template_key: "INSERT INTO t VALUES (?)".to_string(),
            count: 7,
            avg_us: 80,
            min_us: 5,
            max_us: 200,
            p50_us: 70,
            p95_us: 180,
            p99_us: 195,
            first_seen: "2025-01-01 01:00:00".to_string(),
            last_seen: "2025-01-01 11:00:00".to_string(),
        },
    ];

    exporter
        .write_template_stats(&stats, Some(&companion_str), None)
        .unwrap();

    assert!(companion.exists(), "指定路径伴随文件应存在");

    let content = std::fs::read_to_string(&companion).unwrap();
    let mut lines = content.lines();

    // 验证表头精确匹配
    let header = lines.next().unwrap();
    assert_eq!(
        header,
        "template_key,count,avg_us,min_us,max_us,p50_us,p95_us,p99_us,first_seen,last_seen"
    );

    // 验证数据行数量 = 2
    let data_lines: Vec<&str> = lines.collect();
    assert_eq!(data_lines.len(), 2);

    // 验证第一行：template_key 含引号+逗号，应被双引号包裹且引号转义
    let first_row = data_lines[0];
    assert!(
        first_row.starts_with('"'),
        "含特殊字符的 template_key 应被双引号包裹"
    );
    assert!(
        first_row.contains("\"\"John\"\""),
        "引号应被转义为 \"\", row: {first_row}"
    );

    // 验证数值字段可直接 parse
    // 格式："<key>",count,avg_us,...,p99_us,"first_seen","last_seen"
    // template_key 以 ," 开头，找到 key 结束引号后提取数值部分（不含末尾时间戳字段）
    // 第一个 template_key 是 `"SELECT * FROM t WHERE name = ""John"", age = ?"`,
    // 末尾 `?"` 即 key 结束。之后的格式为 ,count,avg,...,p99_us,"first_seen","last_seen"
    // 用逗号分割，提取第 1、2 个数值字段（count, avg_us）。
    let after_key = {
        // key 的结束引号后紧跟 `,count`，key 内部含有 `?` 后接 `"` 的组合。
        // 找到第一组 `,"` 之后的第一个逗号分隔边界——更稳妥地直接按 CSV 字段拆分。
        // 简化做法：用 `","` 定位 key 的末尾（key 以 `?"` 结尾，其后紧随 `,42,`）。
        // key 末尾实际是 `= ?"`, 故找 `?"` 再跳过一个 `,` 最简单。
        let end_marker = "?\"";
        let pos = first_row.find(end_marker).expect("应找到 key 结尾标记");
        &first_row[pos + end_marker.len()..]
    };
    let nums: Vec<&str> = after_key.trim_start_matches(',').split(',').collect();
    assert_eq!(nums[0].parse::<u64>().unwrap(), 42u64);
    assert_eq!(nums[1].parse::<u64>().unwrap(), 150u64);
    // first_seen 和 last_seen 应被双引号包裹（nums[7] 和 nums[8]）
    assert_eq!(nums[7], "\"2025-01-01 00:00:00\"");
    assert_eq!(nums[8], "\"2025-01-01 12:00:00\"");
}

/// TMPL-04-H：验证显式路径写入（不再推导 companion 路径）
#[test]
fn test_parallel_csv_companion_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let self_path = dir.path().join("output.csv");
    // 显式指定目标路径
    let explicit_path = dir.path().join("actual_output_templates.csv");
    let explicit_str = explicit_path.to_string_lossy().into_owned();

    let mut exporter = CsvExporter::new(&self_path);

    let stats = vec![crate::pipeline::TemplateStats {
        template_key: "SELECT 1".to_string(),
        count: 1,
        avg_us: 100,
        min_us: 10,
        max_us: 200,
        p50_us: 90,
        p95_us: 180,
        p99_us: 195,
        first_seen: "2025-01-01 00:00:00".to_string(),
        last_seen: "2025-01-01 01:00:00".to_string(),
    }];

    exporter
        .write_template_stats(&stats, Some(&explicit_str), None)
        .unwrap();

    // 文件应在显式指定路径
    assert!(
        explicit_path.exists(),
        "文件应在 actual_output_templates.csv"
    );
    // 旧推导路径不存在
    let old_companion = dir.path().join("output_templates.csv");
    assert!(
        !old_companion.exists(),
        "output_templates.csv 不应存在（已改为显式路径）"
    );
}

// 新增：csv_output_path=None 时跳过，不创建任何文件
#[test]
fn test_csv_write_template_stats_none_skips() {
    let dir = tempfile::TempDir::new().unwrap();
    let outfile = dir.path().join("output.csv");

    let mut exporter = CsvExporter::new(&outfile);
    exporter.initialize().unwrap();
    exporter.finalize().unwrap();

    let stats = vec![crate::pipeline::TemplateStats {
        template_key: "SELECT 1".to_string(),
        count: 1,
        avg_us: 100,
        min_us: 10,
        max_us: 200,
        p50_us: 90,
        p95_us: 180,
        p99_us: 195,
        first_seen: "2025-01-01 00:00:00".to_string(),
        last_seen: "2025-01-01 01:00:00".to_string(),
    }];

    exporter.write_template_stats(&stats, None, None).unwrap();

    // 目录中除 output.csv 外不应有额外文件
    let extra: Vec<std::fs::DirEntry> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path() != outfile)
        .collect();
    assert!(extra.is_empty(), "None 路径时不应创建任何额外文件");
}

// 新增：csv_output_path=Some("") 时跳过，不创建任何文件
#[test]
fn test_csv_write_template_stats_empty_path_skips() {
    let dir = tempfile::TempDir::new().unwrap();
    let outfile = dir.path().join("output.csv");

    let mut exporter = CsvExporter::new(&outfile);
    exporter.initialize().unwrap();
    exporter.finalize().unwrap();

    let stats = vec![crate::pipeline::TemplateStats {
        template_key: "SELECT 1".to_string(),
        count: 1,
        avg_us: 100,
        min_us: 10,
        max_us: 200,
        p50_us: 90,
        p95_us: 180,
        p99_us: 195,
        first_seen: "2025-01-01 00:00:00".to_string(),
        last_seen: "2025-01-01 01:00:00".to_string(),
    }];

    exporter
        .write_template_stats(&stats, Some(""), None)
        .unwrap();

    let extra: Vec<std::fs::DirEntry> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path() != outfile)
        .collect();
    assert!(extra.is_empty(), "空路径时不应创建任何额外文件");
}
