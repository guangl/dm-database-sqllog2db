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
        "[sqllog]\npath = \"{logdir}\"\n[error]\nfile = \"{errlog}\"\n[logging]\nfile = \"{applog}\"\nlevel = \"warn\"\nretention_days = 1\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\ninclude_performance_metrics = false\n",
        logdir = dir.path().to_string_lossy().replace('\\', "/"),
        errlog = error_log.to_string_lossy().replace('\\', "/"),
        applog = app_log.to_string_lossy().replace('\\', "/"),
        csv = csv_path.to_string_lossy().replace('\\', "/"),
    );
    let cfg: Config = toml::from_str(&toml).unwrap();

    handle_run(
        &cfg,
        None,
        false,
        true,
        &Arc::new(AtomicBool::new(false)),
        80,
        false,
        None,
        1,
        None,
    )
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

#[test]
fn test_aggregator_disabled_none_path() {
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
        "[sqllog]\npath = \"{logdir}\"\n[error]\nfile = \"{errlog}\"\n[logging]\nfile = \"{applog}\"\nlevel = \"warn\"\nretention_days = 1\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\n",
        logdir = dir.path().to_string_lossy().replace('\\', "/"),
        errlog = error_log.to_string_lossy().replace('\\', "/"),
        applog = app_log.to_string_lossy().replace('\\', "/"),
        csv = csv_path.to_string_lossy().replace('\\', "/"),
    );
    let cfg: Config = toml::from_str(&toml).unwrap();

    let result = handle_run(
        &cfg,
        None,
        false,
        true,
        &Arc::new(AtomicBool::new(false)),
        80,
        false,
        None,
        1,
        None,
    );
    assert!(
        result.is_ok(),
        "handle_run 应在无 template_analysis 配置时成功: {result:?}"
    );
}

#[test]
fn test_parallel_merge_consistent() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_line = "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT id FROM orders WHERE user_id = 42. EXECTIME: 5(ms) ROWCOUNT: 3(rows) EXEC_ID: 1.\n";
    for name in ["a.log", "b.log"] {
        std::fs::write(dir.path().join(name), log_line).unwrap();
    }
    let error_log = dir.path().join("errors.log");
    let app_log = dir.path().join("app.log");

    let make_cfg = |csv_file: &str| {
        let toml = format!(
            "[sqllog]\npath = \"{logdir}\"\n[error]\nfile = \"{errlog}\"\n[logging]\nfile = \"{applog}\"\nlevel = \"warn\"\nretention_days = 1\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\n[template]\nenable = true\n",
            logdir = dir.path().to_string_lossy().replace('\\', "/"),
            errlog = error_log.to_string_lossy().replace('\\', "/"),
            applog = app_log.to_string_lossy().replace('\\', "/"),
            csv = csv_file,
        );
        toml::from_str::<Config>(&toml).unwrap()
    };

    let csv_seq = dir
        .path()
        .join("out_seq.csv")
        .to_string_lossy()
        .replace('\\', "/");
    let cfg_seq = make_cfg(&csv_seq);
    let result_seq = handle_run(
        &cfg_seq,
        None,
        false,
        true,
        &Arc::new(AtomicBool::new(false)),
        80,
        false,
        None,
        1,
        None,
    );
    assert!(result_seq.is_ok(), "顺序路径应成功: {result_seq:?}");

    let csv_par = dir
        .path()
        .join("out_par.csv")
        .to_string_lossy()
        .replace('\\', "/");
    let cfg_par = make_cfg(&csv_par);
    let result_par = handle_run(
        &cfg_par,
        None,
        false,
        true,
        &Arc::new(AtomicBool::new(false)),
        80,
        false,
        None,
        4,
        None,
    );
    assert!(result_par.is_ok(), "并行路径应成功: {result_par:?}");

    let seq_lines = std::fs::read_to_string(&csv_seq).unwrap().lines().count();
    let par_lines = std::fs::read_to_string(&csv_par).unwrap().lines().count();
    assert_eq!(seq_lines, par_lines, "顺序与并行输出行数应一致");
}

#[test]
fn test_no_template_stats_when_disabled() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_path = dir.path().join("input.log");
    std::fs::write(
        &log_path,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT id FROM orders WHERE user_id = 42. EXECTIME: 5(ms) ROWCOUNT: 3(rows) EXEC_ID: 1.\n",
    )
    .unwrap();
    let csv_path = dir.path().join("out.csv");
    let companion_path = dir.path().join("out_templates.csv");
    let error_log = dir.path().join("errors.log");
    let app_log = dir.path().join("app.log");

    let toml = format!(
        "[sqllog]\npath = \"{logdir}\"\n[error]\nfile = \"{errlog}\"\n[logging]\nfile = \"{applog}\"\nlevel = \"warn\"\nretention_days = 1\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\n",
        logdir = dir.path().to_string_lossy().replace('\\', "/"),
        errlog = error_log.to_string_lossy().replace('\\', "/"),
        applog = app_log.to_string_lossy().replace('\\', "/"),
        csv = csv_path.to_string_lossy().replace('\\', "/"),
    );
    let cfg: Config = toml::from_str(&toml).unwrap();

    handle_run(
        &cfg,
        None,
        false,
        true,
        &Arc::new(AtomicBool::new(false)),
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();

    assert!(csv_path.exists(), "主 CSV 文件应存在");
    assert!(
        !companion_path.exists(),
        "disabled 状态下不应生成伴随文件 out_templates.csv"
    );
}

#[test]
fn test_template_stats_enabled_end_to_end_sequential() {
    let dir = tempfile::TempDir::new().unwrap();
    let log_path = dir.path().join("input.log");
    std::fs::write(
        &log_path,
        "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT id FROM orders WHERE user_id = 42. EXECTIME: 5(ms) ROWCOUNT: 3(rows) EXEC_ID: 1.\n",
    )
    .unwrap();
    let csv_path = dir.path().join("out.csv");
    let companion_path = dir.path().join("explicit_templates.csv");
    let error_log = dir.path().join("errors.log");
    let app_log = dir.path().join("app.log");

    let toml = format!(
        "[sqllog]\npath = \"{logdir}\"\n[error]\nfile = \"{errlog}\"\n[logging]\nfile = \"{applog}\"\nlevel = \"warn\"\nretention_days = 1\n[exporter.csv]\nfile = \"{csv}\"\noverwrite = true\nappend = false\n[template]\nenable = true\noutput_csv_path = \"{companion}\"\n",
        logdir = dir.path().to_string_lossy().replace('\\', "/"),
        errlog = error_log.to_string_lossy().replace('\\', "/"),
        applog = app_log.to_string_lossy().replace('\\', "/"),
        csv = csv_path.to_string_lossy().replace('\\', "/"),
        companion = companion_path.to_string_lossy().replace('\\', "/"),
    );
    let cfg: Config = toml::from_str(&toml).unwrap();

    handle_run(
        &cfg,
        None,
        false,
        true,
        &Arc::new(AtomicBool::new(false)),
        80,
        false,
        None,
        1,
        None,
    )
    .unwrap();

    assert!(csv_path.exists(), "主 CSV 文件应存在");
    assert!(
        std::fs::metadata(&csv_path).unwrap().len() > 0,
        "主 CSV 文件应非空"
    );

    assert!(
        companion_path.exists(),
        "enabled=true 时应在 output_csv_path 指定位置生成伴随文件"
    );

    let companion_content = std::fs::read_to_string(&companion_path).unwrap();
    let mut lines = companion_content.lines();
    let header = lines.next().unwrap();
    assert_eq!(
        header, "template_key,count,avg_us,min_us,max_us,p50_us,p95_us,p99_us,first_seen,last_seen",
        "伴随文件表头应完整匹配"
    );
    let data_lines = companion_content.lines().count();
    assert!(
        data_lines >= 2,
        "伴随文件应包含表头 + 至少 1 行数据，实际行数: {data_lines}"
    );
}
