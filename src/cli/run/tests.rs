use super::*;
use crate::config::Config;

#[test]
fn test_include_performance_metrics_false_csv_excludes_pm_columns() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_path = dir.path().join("t.log");
    std::fs::write(
        &log_path,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT 1. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n",
    )
    .unwrap();
    let csv_path = dir.path().join("out.csv");
    let error_log = dir.path().join("errors.log");
    let app_log = dir.path().join("app.log");

    let toml = format!(
        "[sqllog]\ninputs = [\"{logdir}\"]\n[error]\nfile = \"{errlog}\"\n[logging]\nfile = \"{applog}\"\nlevel = \"warn\"\nretention_days = 1\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\ninclude_performance_metrics = false\n",
        logdir = dir.path().to_string_lossy().replace('\\', "/"),
        errlog = error_log.to_string_lossy().replace('\\', "/"),
        applog = app_log.to_string_lossy().replace('\\', "/"),
        csv = csv_path.to_string_lossy().replace('\\', "/"),
    );
    let cfg: Config = toml::from_str(&toml).unwrap();

    handle_run(&cfg, true, false, &Arc::new(AtomicBool::new(false)), None).unwrap();

    let content = std::fs::read_to_string(&csv_path).unwrap();
    let header = content.lines().next().unwrap();
    assert!(
        !header.contains("exec_time_ms"),
        "header should skip exec_time_ms: {header}"
    );
    assert!(
        !header.contains("row_count"),
        "header should skip row_count: {header}"
    );
    assert!(
        !header.contains("exec_id"),
        "header should skip exec_id: {header}"
    );
    assert!(header.contains("sql"), "sql column should remain: {header}");
}

#[test]
fn test_handle_run_default_config_succeeds() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_path = dir.path().join("t.log");
    std::fs::write(
        &log_path,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT 1. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n",
    )
    .unwrap();
    let csv_path = dir.path().join("out.csv");
    let error_log = dir.path().join("errors.log");
    let app_log = dir.path().join("app.log");

    let toml = format!(
        "[sqllog]\ninputs = [\"{logdir}\"]\n[error]\nfile = \"{errlog}\"\n[logging]\nfile = \"{applog}\"\nlevel = \"warn\"\nretention_days = 1\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\n",
        logdir = dir.path().to_string_lossy().replace('\\', "/"),
        errlog = error_log.to_string_lossy().replace('\\', "/"),
        applog = app_log.to_string_lossy().replace('\\', "/"),
        csv = csv_path.to_string_lossy().replace('\\', "/"),
    );
    let cfg: Config = toml::from_str(&toml).unwrap();

    let result = handle_run(&cfg, true, false, &Arc::new(AtomicBool::new(false)), None);
    assert!(result.is_ok(), "handle_run 应在默认配置时成功: {result:?}");
}

#[test]
fn test_filter_path() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_path = dir.path().join("t.log");
    std::fs::write(
        &log_path,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT 1. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n",
    )
    .unwrap();
    let csv_path = dir.path().join("out.csv");
    let error_log = dir.path().join("errors.log");
    let app_log = dir.path().join("app.log");

    let toml = format!(
        "[sqllog]\ninputs = [\"{logdir}\"]\n[error]\nfile = \"{errlog}\"\n[logging]\nfile = \"{applog}\"\nlevel = \"warn\"\nretention_days = 1\n[filter]\nenable = true\nusernames = [\"U\"]\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\n",
        logdir = dir.path().to_string_lossy().replace('\\', "/"),
        errlog = error_log.to_string_lossy().replace('\\', "/"),
        applog = app_log.to_string_lossy().replace('\\', "/"),
        csv = csv_path.to_string_lossy().replace('\\', "/"),
    );
    let cfg: Config = toml::from_str(&toml).unwrap();

    handle_run(&cfg, true, false, &Arc::new(AtomicBool::new(false)), None).unwrap();

    let content = std::fs::read_to_string(&csv_path).unwrap();
    assert!(
        content.contains("SELECT 1"),
        "filtered record should appear in output: {content}"
    );
}

#[test]
fn test_parallel_merge_consistent() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_line = "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT id FROM orders WHERE user_id = 42. EXECTIME: 5(ms) ROWCOUNT: 3(rows) EXEC_ID: 1.\n";
    let error_log = dir.path().join("errors.log");
    let app_log = dir.path().join("app.log");

    let make_cfg_dir = |logdir: &std::path::Path, csv_file: &str| {
        let toml = format!(
            "[sqllog]\ninputs = [\"{logdir}\"]\n[error]\nfile = \"{errlog}\"\n[logging]\nfile = \"{applog}\"\nlevel = \"warn\"\nretention_days = 1\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\n",
            logdir = logdir.to_string_lossy().replace('\\', "/"),
            errlog = error_log.to_string_lossy().replace('\\', "/"),
            applog = app_log.to_string_lossy().replace('\\', "/"),
            csv = csv_file,
        );
        toml::from_str::<Config>(&toml).unwrap()
    };

    // Sequential: single file in its own directory → log_files.len() == 1 → parallel never
    // triggered regardless of available_parallelism(). This is the pattern used in
    // test_sqlite_parallel_matches_sequential.
    let seq_dir = dir.path().join("seq");
    std::fs::create_dir(&seq_dir).unwrap();
    std::fs::write(seq_dir.join("only.log"), log_line).unwrap();
    let csv_seq = dir
        .path()
        .join("out_seq.csv")
        .to_string_lossy()
        .replace('\\', "/");
    let cfg_seq = make_cfg_dir(&seq_dir, &csv_seq);
    let result_seq = handle_run(
        &cfg_seq,
        true,
        false,
        &Arc::new(AtomicBool::new(false)),
        None,
    );
    assert!(result_seq.is_ok(), "顺序路径应成功: {result_seq:?}");

    // Parallel: two files trigger multi-file parallel path on modern multi-core machines
    let par_dir = dir.path().join("par");
    std::fs::create_dir(&par_dir).unwrap();
    for name in ["a.log", "b.log"] {
        std::fs::write(par_dir.join(name), log_line).unwrap();
    }
    let csv_par = dir
        .path()
        .join("out_par.csv")
        .to_string_lossy()
        .replace('\\', "/");
    let cfg_par = make_cfg_dir(&par_dir, &csv_par);
    let result_par = handle_run(
        &cfg_par,
        true,
        false,
        &Arc::new(AtomicBool::new(false)),
        Some(2),
    );
    assert!(result_par.is_ok(), "并行路径应成功: {result_par:?}");

    // Sequential has 1 file (1 data row + 1 header), parallel has 2 files (2 data rows + 1 header)
    let seq_lines = std::fs::read_to_string(&csv_seq).unwrap().lines().count();
    let par_lines = std::fs::read_to_string(&csv_par).unwrap().lines().count();
    assert_eq!(
        par_lines,
        seq_lines + 1,
        "并行路径（2 个文件）应比顺序路径（1 个文件）多 1 条数据行"
    );
}

#[test]
fn test_sqlite_parallel_matches_sequential() {
    let dir = tempfile::TempDir::new().unwrap();
    let error_log = dir.path().join("errors.log");
    let app_log = dir.path().join("app.log");

    // Each file: plain INS + PARAMS(same sess/stmt) + parametrized INS using those PARAMS.
    // PARAMS line intentionally omits trailing '.' so parse_params succeeds (no EXEC_ID/ROWCOUNT).
    let log_a = "\
2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [INS] INSERT INTO t VALUES (1). EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 100.\n\
2025-01-15 10:30:28.010 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x2 appname:A ip:10.0.0.1) PARAMS(SEQNO, TYPE, DATA)={(0, VARCHAR, 'alice')}\n\
2025-01-15 10:30:28.011 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x2 appname:A ip:10.0.0.1) [INS] INSERT INTO t(name) VALUES (?). EXECTIME: 2(ms) ROWCOUNT: 1(rows) EXEC_ID: 101.\n";
    let log_b = "\
2025-01-15 10:30:28.001 (EP[0] sess:0x0002 user:U trxid:2 stmt:0x3 appname:A ip:10.0.0.1) [INS] INSERT INTO t VALUES (2). EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 200.\n\
2025-01-15 10:30:28.010 (EP[0] sess:0x0002 user:U trxid:2 stmt:0x4 appname:A ip:10.0.0.1) PARAMS(SEQNO, TYPE, DATA)={(0, VARCHAR, 'bob')}\n\
2025-01-15 10:30:28.011 (EP[0] sess:0x0002 user:U trxid:2 stmt:0x4 appname:A ip:10.0.0.1) [INS] INSERT INTO t(name) VALUES (?). EXECTIME: 2(ms) ROWCOUNT: 1(rows) EXEC_ID: 201.\n";
    let log_c = "\
2025-01-15 10:30:28.001 (EP[0] sess:0x0003 user:U trxid:3 stmt:0x5 appname:A ip:10.0.0.1) [INS] INSERT INTO t VALUES (3). EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 300.\n\
2025-01-15 10:30:28.010 (EP[0] sess:0x0003 user:U trxid:3 stmt:0x6 appname:A ip:10.0.0.1) PARAMS(SEQNO, TYPE, DATA)={(0, VARCHAR, 'carol')}\n\
2025-01-15 10:30:28.011 (EP[0] sess:0x0003 user:U trxid:3 stmt:0x6 appname:A ip:10.0.0.1) [INS] INSERT INTO t(name) VALUES (?). EXECTIME: 2(ms) ROWCOUNT: 1(rows) EXEC_ID: 301.\n";

    let par_dir = dir.path().join("par");
    std::fs::create_dir(&par_dir).unwrap();
    std::fs::write(par_dir.join("a.log"), log_a).unwrap();
    std::fs::write(par_dir.join("b.log"), log_b).unwrap();
    std::fs::write(par_dir.join("c.log"), log_c).unwrap();

    let seq_dir = dir.path().join("seq");
    std::fs::create_dir(&seq_dir).unwrap();
    std::fs::write(seq_dir.join("all.log"), format!("{log_a}{log_b}{log_c}")).unwrap();

    let make_cfg = |logdir: &std::path::Path, db_path: &str| {
        let toml = format!(
            "[sqllog]\ninputs = [\"{logdir}\"]\n[error]\nfile = \"{errlog}\"\n[logging]\nfile = \"{applog}\"\nlevel = \"warn\"\nretention_days = 1\n[exporter.sqlite]\ndatabase_url = \"{db}\"\ntable_name = \"sqllog\"\noverwrite = true\nappend = false\nbatch_size = 1000\n",
            logdir = logdir.to_string_lossy().replace('\\', "/"),
            errlog = error_log.to_string_lossy().replace('\\', "/"),
            applog = app_log.to_string_lossy().replace('\\', "/"),
            db = db_path,
        );
        toml::from_str::<Config>(&toml).unwrap()
    };

    let seq_db = dir
        .path()
        .join("seq.db")
        .to_string_lossy()
        .replace('\\', "/");
    let par_db = dir
        .path()
        .join("par.db")
        .to_string_lossy()
        .replace('\\', "/");

    handle_run(
        &make_cfg(&seq_dir, &seq_db),
        true,
        false,
        &Arc::new(AtomicBool::new(false)),
        None,
    )
    .unwrap();
    handle_run(
        &make_cfg(&par_dir, &par_db),
        true,
        false,
        &Arc::new(AtomicBool::new(false)),
        None,
    )
    .unwrap();

    let read_rows = |path: &str| {
        let conn = rusqlite::Connection::open(path).unwrap();
        let mut stmt = conn
            .prepare("SELECT exec_id, sql, normalized_sql FROM sqllog ORDER BY exec_id, sql")
            .unwrap();
        stmt.query_map([], |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
    };

    let seq_rows = read_rows(&seq_db);
    let par_rows = read_rows(&par_db);

    assert_eq!(
        seq_rows, par_rows,
        "并行 SQLite 输出与顺序模式记录集合应一致"
    );
    assert!(std::fs::metadata(&par_db).unwrap().len() > 0);
}

// ── Gap 1: normalize_and_export with passes=false + do_normalize=true ──────────
//
// 行为要求：当 passes=false 且 do_normalize=true 且 record.tag.is_none() 时，
// 函数应更新 params_buffer（不导出），返回 ExportAction::Continue，
// records_in_file 保持为 0。
#[test]
fn test_normalize_and_export_filtered_params_updates_buffer() {
    use super::processor::{ExportAction, normalize_and_export};
    use crate::error::ErrorStats;
    use crate::exporter::CsvExporter;
    use crate::exporter::ExporterManager;
    use crate::pipeline::normalizer::ParamBuffer;
    use dm_database_parser_sqllog::Sqllog;

    let dir = tempfile::TempDir::new().unwrap();
    let csv_path = dir.path().join("out.csv");

    let exporter = CsvExporter::new(&csv_path);
    let mut manager = ExporterManager::from_csv(exporter);
    manager.initialize().unwrap();

    // PARAMS 记录：tag=None，sql 包含 PARAMS(…) 语法
    let record = Sqllog {
        ts: "2024-01-01 00:00:00.000".to_string(),
        tag: None,
        ep: 0,
        sess_id: "sess_gap1".to_string(),
        thrd_id: "t1".to_string(),
        username: "usr".to_string(),
        trxid: "tx1".to_string(),
        statement: "stmt_gap1".to_string(),
        appname: "app".to_string(),
        client_ip: "127.0.0.1".to_string(),
        sql: "PARAMS(SEQNO, TYPE, DATA)={(0, VARCHAR, 'hello')}".to_string(),
        exectime: 0.0,
        rowcount: 0,
        exec_id: 0,
    };

    let mut params_buffer: ParamBuffer = ParamBuffer::new();
    let mut ns_scratch: Vec<u8> = Vec::new();
    let mut records_in_file: usize = 0;
    let mut file_stats = ErrorStats::default();

    let action = normalize_and_export(
        &record,
        &mut manager,
        true, // include_pm
        true, // do_normalize: PARAMS buf 应被更新
        &mut params_buffer,
        None, // placeholder_override
        &mut ns_scratch,
        None, // remaining (no quota)
        &mut records_in_file,
        &mut file_stats,
        "test_file.log",
        false, // passes=false → 不导出
    );

    // 必须返回 Continue（不是 BreakQuota 或 BreakFatal）
    assert!(
        matches!(action, ExportAction::Continue),
        "passes=false 路径应返回 Continue"
    );
    // 不应有任何记录被导出
    assert_eq!(
        records_in_file, 0,
        "passes=false 时 records_in_file 应保持为 0，实际为 {records_in_file}"
    );
    // params_buffer 应已被更新（PARAMS 记录已解析入缓冲区）
    let buf_key = ("sess_gap1".to_string(), "stmt_gap1".to_string());
    assert!(
        params_buffer.contains_key(&buf_key),
        "passes=false+do_normalize=true 下 PARAMS 记录应写入 params_buffer，\
         但 key ({:?}) 不存在; buffer keys={:?}",
        buf_key,
        params_buffer.keys().collect::<Vec<_>>()
    );
}

// ── Gap 2: normalize_and_export BreakQuota path ─────────────────────────────
//
// 行为要求：当 passes=true 且 remaining=Some(0)（配额耗尽）时，
// 函数应返回 ExportAction::BreakQuota，records_in_file 保持为 0（不导出）。
#[test]
fn test_normalize_and_export_quota_hit_returns_break_quota() {
    use super::processor::{ExportAction, normalize_and_export};
    use crate::error::ErrorStats;
    use crate::exporter::CsvExporter;
    use crate::exporter::ExporterManager;
    use crate::pipeline::normalizer::ParamBuffer;
    use dm_database_parser_sqllog::Sqllog;

    let dir = tempfile::TempDir::new().unwrap();
    let csv_path = dir.path().join("out.csv");

    let mut manager = ExporterManager::from_csv(CsvExporter::new(&csv_path));
    manager.initialize().unwrap();

    // 普通 SEL 记录，passes=true
    let record = Sqllog {
        ts: "2024-01-01 00:00:00.000".to_string(),
        tag: Some("SEL".to_string()),
        ep: 0,
        sess_id: "sess_gap2".to_string(),
        thrd_id: "t2".to_string(),
        username: "usr".to_string(),
        trxid: "tx2".to_string(),
        statement: "stmt_gap2".to_string(),
        appname: "app".to_string(),
        client_ip: "127.0.0.1".to_string(),
        sql: "SELECT 1".to_string(),
        exectime: 1.0,
        rowcount: 1,
        exec_id: 42,
    };

    let mut params_buffer: ParamBuffer = ParamBuffer::new();
    let mut ns_scratch: Vec<u8> = Vec::new();
    let mut records_in_file: usize = 0;
    let mut file_stats = ErrorStats::default();

    // remaining=Some(0) 表示配额已耗尽（records_in_file=0 >= remaining=0）
    let action = normalize_and_export(
        &record,
        &mut manager,
        true,  // include_pm
        false, // do_normalize
        &mut params_buffer,
        None,
        &mut ns_scratch,
        Some(0), // remaining=0 → 配额已耗尽
        &mut records_in_file,
        &mut file_stats,
        "test_file.log",
        true, // passes=true → 否则直接 Continue
    );

    // 必须返回 BreakQuota
    assert!(
        matches!(action, ExportAction::BreakQuota),
        "remaining=Some(0) 且 passes=true 应返回 BreakQuota，\
         但得到了 Continue 或 BreakFatal"
    );
    // 不应有任何记录被导出
    assert_eq!(
        records_in_file, 0,
        "BreakQuota 路径下 records_in_file 应保持为 0，实际为 {records_in_file}"
    );
}

// ── PROG-01/02: 进度条模板单元测试 ────────────────────────────────────────────

/// 验证 `make_progress_bar(true, 3)` 返回 `Some(pb)`，
/// `pb.length() == Some(3)`，`pb.position() == 0`，模板设置不 panic。
#[test]
fn test_progress_bar_template() {
    let pb = super::make_progress_bar(true, 3);
    assert!(pb.is_some(), "show_progress=true 应返回 Some(ProgressBar)");
    let pb = pb.unwrap();
    assert_eq!(
        pb.length(),
        Some(3),
        "length() 应为 Some(3)，实际为 {:?}",
        pb.length()
    );
    assert_eq!(
        pb.position(),
        0,
        "初始 position() 应为 0，实际为 {}",
        pb.position()
    );
    // 确认模板已设置（调用 set_message 不应 panic）
    pb.set_message("test message");
    pb.finish_and_clear();
}

/// 验证 `make_progress_bar(false, 3)` 返回 `None`。
#[test]
fn test_progress_bar_disabled() {
    let pb = super::make_progress_bar(false, 3);
    assert!(
        pb.is_none(),
        "show_progress=false 应返回 None，实际返回了 Some"
    );
}

// ── PROG-03/DIAG-03: error log 写出 + hint + 摘要扩展 ───────────────────────

/// 验证 `handle_run` 在有解析错误时写出 error log 文件。
/// 无效行放文件前面（独立记录），解析器以 `\n20` 时间戳为记录边界，
/// 前置无效行无时间戳前缀会独立返回 `InvalidFormat`，从而触发 `parse_error_records`。
#[test]
fn test_error_log_written() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_path = dir.path().join("t.log");
    // 无效行放前面（独立记录）+ 合法 SEL 放后面
    std::fs::write(
        &log_path,
        "garbage line that cannot be parsed\n2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT 1. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n",
    )
    .unwrap();
    let csv_path = dir.path().join("out.csv");
    let error_log = dir.path().join("errors.log");
    let app_log = dir.path().join("app.log");

    let toml = format!(
        "[sqllog]\ninputs = [\"{logdir}\"]\n[error]\nfile = \"{errlog}\"\n[logging]\nfile = \"{applog}\"\nlevel = \"warn\"\nretention_days = 1\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\n",
        logdir = dir.path().to_string_lossy().replace('\\', "/"),
        errlog = error_log.to_string_lossy().replace('\\', "/"),
        applog = app_log.to_string_lossy().replace('\\', "/"),
        csv = csv_path.to_string_lossy().replace('\\', "/"),
    );
    let cfg: Config = toml::from_str(&toml).unwrap();

    handle_run(&cfg, true, false, &Arc::new(AtomicBool::new(false)), None).unwrap();

    assert!(
        error_log.exists(),
        "error log 文件应在有解析错误时被写出，但未找到: {}",
        error_log.display()
    );
    let content = std::fs::read_to_string(&error_log).unwrap();
    assert!(
        content.contains("[ERROR] line "),
        "error log 应含有 '[ERROR] line '，实际内容: {content}"
    );
    assert!(
        content.contains("reason:"),
        "error log 应含有 'reason:'，实际内容: {content}"
    );
}

/// 验证 `print_run_summary` 接受含 `EncodingError` 的 `ErrorStats` 时不 panic。
/// hint 输出走 stderr，集成层手动验证；本测试只保证编译 + 不 panic（防回归）。
#[test]
fn test_hint_output() {
    use crate::error::{ErrorKind, ErrorStats};

    let stats = ErrorStats {
        total_errors: 3,
        parse_errors: 3,
        by_type: {
            let mut m = std::collections::HashMap::new();
            m.insert(ErrorKind::EncodingError, 3u64);
            m
        },
        ..Default::default()
    };
    // 验证 ErrorStats 字段正确构造（hint 行为由手动验证支撑）
    assert_eq!(
        stats
            .by_type
            .get(&ErrorKind::EncodingError)
            .copied()
            .unwrap_or(0),
        3,
        "by_type[EncodingError] 应为 3"
    );
    // 调用 print_run_summary 确认不 panic
    super::print_run_summary(false, false, false, 0.1, &[], 0, 0, &stats);
}

/// 验证含 `filtered_out` 的 `ErrorStats` 传入 `print_run_summary` 时不 panic（防回归）。
#[test]
fn test_run_summary() {
    use crate::error::{ErrorKind, ErrorStats};

    let stats = ErrorStats {
        total_errors: 2,
        parse_errors: 2,
        filtered_out: 5,
        by_type: {
            let mut m = std::collections::HashMap::new();
            m.insert(ErrorKind::EncodingError, 2u64);
            m
        },
        ..Default::default()
    };
    // 调用 print_run_summary 确认不 panic
    super::print_run_summary(false, false, false, 1.5, &[], 10, 0, &stats);
}
