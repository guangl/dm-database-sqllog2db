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

    handle_run(&cfg, true, false, &Arc::new(AtomicBool::new(false))).unwrap();

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

    let result = handle_run(&cfg, true, false, &Arc::new(AtomicBool::new(false)));
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

    handle_run(&cfg, true, false, &Arc::new(AtomicBool::new(false))).unwrap();

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
    let result_seq = handle_run(&cfg_seq, true, false, &Arc::new(AtomicBool::new(false)));
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
    let result_par = handle_run(&cfg_par, true, false, &Arc::new(AtomicBool::new(false)));
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
    )
    .unwrap();
    handle_run(
        &make_cfg(&par_dir, &par_db),
        true,
        false,
        &Arc::new(AtomicBool::new(false)),
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
