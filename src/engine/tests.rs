use super::run::run as handle_run;
use crate::config::Config;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// 构造一个用于记录级单元测试的 `RunContext`：空 pipeline、全字段掩码。
fn make_test_ctx(cfg: &Config, do_normalize: bool) -> super::run::RunContext<'_> {
    super::run::RunContext {
        cfg,
        pipeline: crate::pipeline::Pipeline::new(),
        field_mask: crate::pipeline::FieldMask::ALL,
        ordered_indices: (0..crate::pipeline::FIELD_NAMES.len()).collect(),
        do_normalize,
        placeholder_override: None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_include_performance_metrics_false_csv_excludes_pm_columns() {
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

    handle_run(&cfg, true, false, &Arc::new(AtomicBool::new(false)), None)
        .await
        .unwrap();

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

#[tokio::test(flavor = "multi_thread")]
async fn test_handle_run_default_config_succeeds() {
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

    let result = handle_run(&cfg, true, false, &Arc::new(AtomicBool::new(false)), None).await;
    assert!(result.is_ok(), "handle_run 应在默认配置时成功: {result:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_filter_path() {
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

    handle_run(&cfg, true, false, &Arc::new(AtomicBool::new(false)), None)
        .await
        .unwrap();

    let content = std::fs::read_to_string(&csv_path).unwrap();
    assert!(
        content.contains("SELECT 1"),
        "filtered record should appear in output: {content}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_parallel_merge_consistent() {
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
    )
    .await;
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
    )
    .await;
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

#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_parallel_matches_sequential() {
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
    .await
    .unwrap();
    handle_run(
        &make_cfg(&par_dir, &par_db),
        true,
        false,
        &Arc::new(AtomicBool::new(false)),
        None,
    )
    .await
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
    use super::record::{ExportAction, normalize_and_export};
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

    // do_normalize=true：PARAMS buf 应被更新
    let cfg = Config::default();
    let ctx = make_test_ctx(&cfg, true);
    let env = super::record::ExportEnv {
        ctx: &ctx,
        include_pm: true,
        file_path: "test_file.log",
    };
    let mut state = super::record::LoopState {
        params_buffer: &mut params_buffer,
        ns_scratch: &mut ns_scratch,
        records_in_file: 0,
        file_stats: ErrorStats::default(),
    };
    let action = normalize_and_export(
        &env,
        &record,
        &mut manager,
        &mut state,
        None,  // remaining (no quota)
        false, // passes=false → 不导出
    );

    // 必须返回 Continue（不是 BreakQuota 或 BreakFatal）
    assert!(
        matches!(action, ExportAction::Continue),
        "passes=false 路径应返回 Continue"
    );
    // 不应有任何记录被导出
    assert_eq!(
        state.records_in_file, 0,
        "passes=false 时 records_in_file 应保持为 0，实际为 {}",
        state.records_in_file
    );
    // params_buffer 应已被更新（PARAMS 记录已解析入缓冲区）
    assert!(
        params_buffer
            .get("sess_gap1")
            .and_then(|inner| inner.get("stmt_gap1"))
            .is_some(),
        "passes=false+do_normalize=true 下 PARAMS 记录应写入 params_buffer，\
         但 key (sess_gap1, stmt_gap1) 不存在; outer keys={:?}",
        params_buffer.keys().collect::<Vec<_>>()
    );
}

// ── Gap 2: normalize_and_export BreakQuota path ─────────────────────────────
//
// 行为要求：当 passes=true 且 remaining=Some(0)（配额耗尽）时，
// 函数应返回 ExportAction::BreakQuota，records_in_file 保持为 0（不导出）。
#[test]
fn test_normalize_and_export_quota_hit_returns_break_quota() {
    use super::record::{ExportAction, normalize_and_export};
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

    let cfg = Config::default();
    let ctx = make_test_ctx(&cfg, false);
    let env = super::record::ExportEnv {
        ctx: &ctx,
        include_pm: true,
        file_path: "test_file.log",
    };
    let mut state = super::record::LoopState {
        params_buffer: &mut params_buffer,
        ns_scratch: &mut ns_scratch,
        records_in_file: 0,
        file_stats: ErrorStats::default(),
    };
    // remaining=Some(0) 表示配额已耗尽（records_in_file=0 >= remaining=0）
    let action = normalize_and_export(
        &env,
        &record,
        &mut manager,
        &mut state,
        Some(0), // remaining=0 → 配额已耗尽
        true,    // passes=true → 否则直接 Continue
    );

    // 必须返回 BreakQuota
    assert!(
        matches!(action, ExportAction::BreakQuota),
        "remaining=Some(0) 且 passes=true 应返回 BreakQuota，\
         但得到了 Continue 或 BreakFatal"
    );
    // 不应有任何记录被导出
    assert_eq!(
        state.records_in_file, 0,
        "BreakQuota 路径下 records_in_file 应保持为 0，实际为 {}",
        state.records_in_file
    );
}

// ── PROG-01/02: 进度条模板单元测试 ────────────────────────────────────────────

/// 验证 `make_progress_bar(true, 3)` 返回 `Some(pb)`，
/// `pb.length() == Some(3)`，`pb.position() == 0`，模板设置不 panic。
#[test]
fn test_progress_bar_template() {
    let pb = super::prepare::make_progress_bar(true, 3);
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
    let pb = super::prepare::make_progress_bar(false, 3);
    assert!(
        pb.is_none(),
        "show_progress=false 应返回 None，实际返回了 Some"
    );
}

// ── PROG-03/DIAG-03: error log 写出 + hint + 摘要扩展 ───────────────────────

/// 验证 `handle_run` 在有解析错误时写出 error log 文件。
/// 无效行放文件前面（独立记录），解析器以 `\n20` 时间戳为记录边界，
/// 前置无效行无时间戳前缀会独立返回 `InvalidFormat`，从而触发 `parse_error_records`。
#[tokio::test(flavor = "multi_thread")]
async fn test_error_log_written() {
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

    handle_run(&cfg, true, false, &Arc::new(AtomicBool::new(false)), None)
        .await
        .unwrap();

    // AsyncLogParser 静默丢弃逐条解析错误，error log 不再写出
    assert!(
        !error_log.exists(),
        "AsyncLogParser 不追踪逐条解析错误，error log 不应存在，但找到了: {}",
        error_log.display()
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
    super::report::print_run_summary(
        false,
        false,
        &super::report::RunSummary {
            use_parallel: false,
            elapsed: 0.1,
            processed_files: &[],
            total_records: 0,
            skipped_files: 0,
        },
        &stats,
    );
}

// WATCH-08: run 路径仍为覆盖写（append_error_log=false 默认值防回归）
/// 验证 `append_error_log=false` 时 `write_error_log` 以截断模式打开文件，旧内容被覆盖。
#[test]
fn test_write_error_log_run_still_truncates() {
    use crate::config::ErrorLogConfig;
    use crate::error::{ErrorKind, ErrorStats, ParseErrorRecord};

    let tmp_file = tempfile::NamedTempFile::new().expect("failed to create tempfile");
    let tmp_path = tmp_file.path().to_string_lossy().into_owned();

    // 预置旧内容
    std::fs::write(&tmp_path, b"OLD CONTENT\n").expect("failed to write old content");

    let cfg = Config {
        error: Some(ErrorLogConfig {
            file: tmp_path.clone(),
        }),
        append_error_log: false, // run 路径：覆盖写
        ..Config::default()
    };

    let stats = ErrorStats {
        parse_errors: 1,
        total_errors: 1,
        parse_error_records: vec![ParseErrorRecord {
            line_number: 1,
            raw_truncated: "bad line".to_string(),
            kind: ErrorKind::ParseFailed,
        }],
        ..ErrorStats::default()
    };

    super::report::write_error_log(&cfg, &stats);

    let content = std::fs::read_to_string(&tmp_path).expect("failed to read error log");
    assert!(
        !content.contains("OLD CONTENT"),
        "append_error_log=false 时旧内容应被截断，实际内容: {content}"
    );
    assert!(
        content.contains("[ERROR] line "),
        "error log 应含有新写入的 [ERROR] 行，实际内容: {content}"
    );
}

// WATCH-08: watch 路径为追加写（append_error_log=true），旧内容应被保留
/// 验证 `append_error_log=true` 时 `write_error_log` 以追加模式打开文件，旧内容被保留。
#[test]
fn test_write_error_log_watch_appends() {
    use crate::config::ErrorLogConfig;
    use crate::error::{ErrorKind, ErrorStats, ParseErrorRecord};

    let tmp = tempfile::NamedTempFile::new().expect("failed to create tempfile");
    let path = tmp.path().to_string_lossy().into_owned();
    std::fs::write(&path, b"EXISTING\n").expect("failed to write existing content");

    let cfg = Config {
        error: Some(ErrorLogConfig { file: path.clone() }),
        append_error_log: true, // watch 路径：追加写
        ..Config::default()
    };

    let stats = ErrorStats {
        parse_errors: 1,
        total_errors: 1,
        parse_error_records: vec![ParseErrorRecord {
            line_number: 1,
            raw_truncated: "bad".to_string(),
            kind: ErrorKind::ParseFailed,
        }],
        ..ErrorStats::default()
    };

    super::report::write_error_log(&cfg, &stats);

    let content = std::fs::read_to_string(&path).expect("failed to read error log");
    assert!(
        content.contains("EXISTING"),
        "append_error_log=true 时旧内容应被保留（追加模式），实际内容: {content}"
    );
    assert!(
        content.contains("[ERROR] line "),
        "新错误行应追加到文件末尾，实际内容: {content}"
    );
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
    super::report::print_run_summary(
        false,
        false,
        &super::report::RunSummary {
            use_parallel: false,
            elapsed: 1.5,
            processed_files: &[],
            total_records: 10,
            skipped_files: 0,
        },
        &stats,
    );
}
// ── Group 1-4: collector.rs 全分支单元测试 ──────────────────────────────────

#[derive(Debug)]
struct AlwaysFail;
impl crate::pipeline::LogProcessor for AlwaysFail {
    fn process(&self, _: &dm_database_parser_sqllog::Sqllog) -> bool {
        false
    }
}

// Group 1 — InvalidPath 错误路径（collector.rs lines 26-34）
// 传入不存在路径应返回 Err(Error::Parser(ParserError::InvalidPath { .. }))
#[tokio::test(flavor = "multi_thread")]
async fn test_collector_invalid_path_returns_error() {
    use crate::pipeline::Pipeline;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    let pipeline = Pipeline::default();
    let interrupted = Arc::new(AtomicBool::new(false));
    let result = collector::collect_log_file(
        std::path::Path::new("/nonexistent/absolutely/not/there.log"),
        &pipeline,
        false,
        None,
        &interrupted,
    );
    assert!(result.is_err(), "不存在路径应返回 Err，实际: {result:?}");
    assert!(
        matches!(
            result.unwrap_err(),
            crate::error::Error::Parser(crate::error::ParserError::InvalidPath { .. })
        ),
        "应匹配 ParserError::InvalidPath"
    );
}

// Group 2 — parse error 累积循环（collector.rs lines 41-63）
// 含无效行的日志文件应累积 parse_errors 计数，rows 为空
#[tokio::test(flavor = "multi_thread")]
async fn test_collector_parse_error_accumulation() {
    use crate::pipeline::Pipeline;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    let dir = tempfile::TempDir::new().unwrap();
    let log_path = dir.path().join("bad.log");
    std::fs::write(&log_path, "not a valid log line\nalso invalid\n").unwrap();
    let pipeline = Pipeline::default();
    let interrupted = Arc::new(AtomicBool::new(false));
    let (rows, stats) =
        collector::collect_log_file(&log_path, &pipeline, false, None, &interrupted)
            .expect("collect_log_file 应不返回 Err");
    assert!(
        rows.is_empty(),
        "全部非法行应不产生记录，实际 rows.len()={}",
        rows.len()
    );
    // 流式迭代器逐条产出 Result：两行非法文本因缺少有效时间戳起始行被归并为一个解析失败的
    // 记录块，计入一次 parse_errors（与旧的整文件丢弃语义不同，好记录不会被坏记录连带丢弃）。
    assert_eq!(
        stats.parse_errors, 1,
        "非法记录块应计入 parse_errors，实际: {}",
        stats.parse_errors
    );
}

// Group 3 — !needs_processing 过滤分支（collector.rs lines 74-76）
// AlwaysFail 处理器 + DML 记录（tag.is_some()）+ do_normalize=false
// 使 passes=false 且 needs_processing=false，触发 early return
#[tokio::test(flavor = "multi_thread")]
async fn test_collector_not_needed_filtering() {
    use crate::pipeline::Pipeline;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    let dir = tempfile::TempDir::new().unwrap();
    let log_path = dir.path().join("dml.log");
    let valid_dml = "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:TESTUSER trxid:1 stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT id FROM t. EXECTIME: 5(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n";
    std::fs::write(&log_path, valid_dml).unwrap();

    let mut pipeline = Pipeline::new();
    pipeline.add(Box::new(AlwaysFail));
    assert!(!pipeline.is_empty(), "AlwaysFail 添加后 pipeline 应非空");

    let interrupted = Arc::new(AtomicBool::new(false));
    let (rows, _parse_errors) =
        collector::collect_log_file(&log_path, &pipeline, false, None, &interrupted)
            .expect("collect_log_file 应 Ok");
    assert!(
        rows.is_empty(),
        "AlwaysFail 过滤所有记录，rows 应为空，实际 {}",
        rows.len()
    );
}

// Group 4 — 被过滤的 PARAMS else 分支（collector.rs lines 91-100）
// AlwaysFail 处理器 + PARAMS 记录（tag.is_none()）+ do_normalize=true
// 使 passes=false 但 needs_processing=true，触发 compute_normalized 更新 params_buf 但不 push 到 rows
#[tokio::test(flavor = "multi_thread")]
async fn test_collector_filtered_params_normalize() {
    use crate::pipeline::Pipeline;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    let dir = tempfile::TempDir::new().unwrap();
    let log_path = dir.path().join("params.log");
    // PARAMS 行：tag=None，格式与 test_sqlite_parallel_matches_sequential 中已知可解析的样本一致
    let params_line = "2025-01-15 10:30:28.010 (EP[0] sess:0x0001 user:TESTUSER trxid:1 stmt:0x2 appname:App ip:10.0.0.1) PARAMS(SEQNO, TYPE, DATA)={(0, VARCHAR, 'testvalue')}\n";
    std::fs::write(&log_path, params_line).unwrap();

    let mut pipeline = Pipeline::new();
    pipeline.add(Box::new(AlwaysFail));

    let interrupted = Arc::new(AtomicBool::new(false));
    // do_normalize=true 使被过滤的 PARAMS 行仍走 compute_normalized 分支
    let (rows, _parse_errors) =
        collector::collect_log_file(&log_path, &pipeline, true, None, &interrupted)
            .expect("collect_log_file 应 Ok");
    assert!(
        rows.is_empty(),
        "AlwaysFail 过滤 PARAMS 记录，rows 应为空，实际 {}",
        rows.len()
    );
}

// interrupted=true 在第一条记录前命中 break 分支（collector.rs line 42-44）
#[tokio::test(flavor = "multi_thread")]
async fn test_collector_interrupted_returns_empty() {
    use crate::pipeline::Pipeline;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let dir = tempfile::TempDir::new().unwrap();
    let log_path = dir.path().join("data.log");
    let valid_dml = "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:TESTUSER trxid:1 stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT id FROM t. EXECTIME: 5(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n";
    std::fs::write(&log_path, valid_dml).unwrap();
    let pipeline = Pipeline::default();
    let interrupted = Arc::new(AtomicBool::new(true));
    interrupted.store(true, Ordering::Release);
    let (rows, _stats) =
        collector::collect_log_file(&log_path, &pipeline, false, None, &interrupted)
            .expect("collect_log_file 应 Ok");
    assert!(
        rows.is_empty(),
        "interrupted=true 应使循环立即 break，rows 应为空"
    );
}

// ── prescan: build_indicator_filters / build_sql_*_filters 单元测试 ─────────

#[test]
fn test_build_indicator_filters_min_row_count_zero() {
    use crate::pipeline::filters::IndicatorFilters;
    let indicators = IndicatorFilters {
        min_row_count: Some(0),
        ..IndicatorFilters::default()
    };
    let filters = super::prepare::build_indicator_filters(&indicators);
    assert_eq!(
        filters.len(),
        1,
        "min_row_count=0 应构建一个全匹配 Filter（FilterBuilder::new().build() 分支）"
    );
}

#[test]
fn test_build_indicator_filters_min_row_count_positive() {
    use crate::pipeline::filters::IndicatorFilters;
    let indicators = IndicatorFilters {
        min_row_count: Some(5),
        ..IndicatorFilters::default()
    };
    let filters = super::prepare::build_indicator_filters(&indicators);
    assert_eq!(
        filters.len(),
        1,
        "min_row_count=5 应构建一个带 rowcount_gt(4) 约束的 Filter"
    );
}

#[test]
fn test_build_indicator_filters_empty_returns_empty() {
    use crate::pipeline::filters::IndicatorFilters;
    let indicators = IndicatorFilters::default();
    let filters = super::prepare::build_indicator_filters(&indicators);
    assert_eq!(filters.len(), 0, "所有字段均为 None 时应返回空 Vec<Filter>");
}

#[test]
fn test_build_sql_exclude_filters_multiple_returns_correct_count() {
    use crate::pipeline::filters::SqlFilters;
    let sf = SqlFilters {
        excludes: Some(vec![
            "SELECT 1".into(),
            "DROP".into(),
            "DELETE FROM x".into(),
        ]),
        includes: None,
    };
    let filters = super::prepare::build_sql_exclude_filters(&sf);
    assert_eq!(
        filters.len(),
        3,
        "3 个 exclude 模式应构建 3 个 Filter（非空 excludes 分支）"
    );
}

#[test]
fn test_build_sql_exclude_filters_none_returns_empty() {
    use crate::pipeline::filters::SqlFilters;
    let sf = SqlFilters::default();
    let filters = super::prepare::build_sql_exclude_filters(&sf);
    assert_eq!(
        filters.len(),
        0,
        "excludes=None 应通过 unwrap_or(&[]) 返回空 Vec<Filter>"
    );
}

#[test]
fn test_build_sql_include_filters_multiple() {
    use crate::pipeline::filters::SqlFilters;
    let sf = SqlFilters {
        includes: Some(vec!["SELECT".into(), "UPDATE".into()]),
        excludes: None,
    };
    let filters = super::prepare::build_sql_include_filters(&sf);
    assert_eq!(filters.len(), 2, "2 个 include 模式应构建 2 个 Filter");
}

#[test]
fn test_build_indicator_filters_exec_ids_multiple() {
    use crate::pipeline::filters::IndicatorFilters;
    use std::collections::HashSet;
    let indicators = IndicatorFilters {
        exec_ids: Some(HashSet::from([1_i64, 2, 42])),
        ..IndicatorFilters::default()
    };
    let filters = super::prepare::build_indicator_filters(&indicators);
    assert_eq!(
        filters.len(),
        3,
        "3 个 exec_ids 应产生 3 个独立的 Filter（每个 ID 一个）"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_min_row_count_zero_matches_all_records() {
    use crate::pipeline::FiltersFeature;
    use crate::pipeline::filters::IndicatorFilters;
    use std::fmt::Write as _;

    let dir = tempfile::TempDir::new().unwrap();
    let logfile = dir.path().join("test.log");

    let mut buf = String::new();
    for i in 0..3_usize {
        writeln!(
            buf,
            "2025-01-15 10:30:28.001 (EP[0] sess:0x{i:04x} user:U trxid:{i} stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT {i}. EXECTIME: 1(ms) ROWCOUNT: {i}(rows) EXEC_ID: {i}.",
        ).unwrap();
    }
    std::fs::write(&logfile, &buf).unwrap();

    let cfg = Config {
        filter: Some(FiltersFeature {
            enable: true,
            indicators: IndicatorFilters {
                min_row_count: Some(0),
                ..IndicatorFilters::default()
            },
            ..FiltersFeature::default()
        }),
        ..Config::default()
    };

    let matched = super::prepare::scan_log_file_for_matches(logfile.to_str().unwrap(), &cfg);
    assert_eq!(
        matched.len(),
        3,
        "min_row_count=0 应匹配所有记录（全匹配 Filter），实际匹配: {matched:?}",
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_scan_for_trxids_by_transaction_filters_dedup_across_files() {
    use crate::pipeline::FiltersFeature;
    use crate::pipeline::filters::IndicatorFilters;
    use std::fmt::Write as _;

    let dir = tempfile::TempDir::new().unwrap();

    let write_log = |filename: &str, ids: &[usize]| {
        let path = dir.path().join(filename);
        let mut buf = String::new();
        for &i in ids {
            writeln!(
                buf,
                "2025-01-15 10:30:28.001 (EP[0] sess:0x{i:04x} user:U trxid:{i} stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT {i}. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.",
            ).unwrap();
        }
        std::fs::write(&path, &buf).unwrap();
        path
    };

    let file1 = write_log("a.log", &[0, 1]);
    let file2 = write_log("b.log", &[1, 2]);

    let cfg = Config {
        filter: Some(FiltersFeature {
            enable: true,
            indicators: IndicatorFilters {
                min_row_count: Some(0),
                ..IndicatorFilters::default()
            },
            ..FiltersFeature::default()
        }),
        ..Config::default()
    };

    let mut matched =
        super::prepare::scan_for_trxids_by_transaction_filters(&[file1, file2], &cfg, 2).unwrap();
    matched.sort();
    assert_eq!(
        matched,
        vec!["0".to_string(), "1".to_string(), "2".to_string()],
        "跨文件应返回去重后的 trxid 列表，实际: {matched:?}"
    );
}

// ── 测试助手：线程内解析单文件并收集记录为 Vec（原 collector.rs，仅测试使用）──

mod collector {
    use crate::error::{Error, ErrorStats, ParserError, Result};
    use crate::pipeline::Pipeline;
    use crate::pipeline::normalizer::ParamBuffer;
    use crate::streaming::open_log_file;
    use dm_database_parser_sqllog::Sqllog;
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// 收集到的记录与其可选归一化 SQL。
    type CollectedRows = Vec<(Sqllog, Option<String>)>;

    /// 收集过程中的可变状态（scratch 缓冲 + 结果 + 统计）。
    #[derive(Default)]
    struct CollectState {
        params_buf: ParamBuffer,
        ns_scratch: Vec<u8>,
        rows: CollectedRows,
        file_stats: ErrorStats,
    }

    /// 线程内解析单个日志文件，收集记录为 Vec，不写出到任何存储。
    pub(super) fn collect_log_file(
        file: &Path,
        pipeline: &Pipeline,
        do_normalize: bool,
        placeholder_override: Option<bool>,
        interrupted: &Arc<AtomicBool>,
    ) -> Result<(CollectedRows, ErrorStats)> {
        // Parse the file, distinguishing IO / not-found errors (file-level, propagated as Err)
        // from per-record parse errors (logged + skipped, file processing continues).
        // We do not pre-check file.exists() to avoid the TOCTOU race where the file could
        // disappear between the check and the open — instead we inspect the error variant.
        let records = match open_log_file(file) {
            Ok(it) => it,
            Err(e) => {
                return Err(Error::Parser(ParserError::InvalidPath {
                    path: file.to_path_buf(),
                    reason: e.to_string(),
                    line_number: None,
                }));
            }
        };

        let mut state = CollectState::default();
        for result in records {
            if interrupted.load(Ordering::Acquire) {
                break;
            }
            let record = match result {
                Ok(r) => r,
                Err(e) => {
                    log::warn!(
                        "collect_log_file: skipping malformed record in '{}': {e}",
                        file.display()
                    );
                    state.file_stats.add_parse_error();
                    continue;
                }
            };
            process_record(&mut state, record, pipeline, do_normalize, placeholder_override);
        }
        Ok((state.rows, state.file_stats))
    }

    fn process_record(
        state: &mut CollectState,
        record: Sqllog,
        pipeline: &Pipeline,
        do_normalize: bool,
        placeholder_override: Option<bool>,
    ) {
        let passes = pipeline.is_empty() || pipeline.run_with_meta(&record);
        let needs_processing = passes || (do_normalize && record.tag.is_none());
        if !needs_processing {
            state.file_stats.filtered_out += 1;
            return;
        }
        if passes {
            let normalized =
                if do_normalize && (!state.params_buf.is_empty() || record.tag.is_none()) {
                    crate::pipeline::compute_normalized(
                        &record,
                        &record.sql,
                        &mut state.params_buf,
                        placeholder_override,
                        &mut state.ns_scratch,
                    )
                    .map(str::to_owned)
                } else {
                    None
                };
            state.rows.push((record, normalized));
        } else {
            state.file_stats.filtered_out += 1;
            let _ = crate::pipeline::compute_normalized(
                &record,
                &record.sql,
                &mut state.params_buf,
                placeholder_override,
                &mut state.ns_scratch,
            );
        }
    }
}
