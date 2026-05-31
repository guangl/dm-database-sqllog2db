/// Baseline benchmark: CSV export throughput.
///
/// Measures the full pipeline: log-file parsing → CSV serialization → write to /dev/null.
/// Run with: `cargo bench --bench bench_csv --features csv`
#[path = "bench_common.rs"]
mod bench_common;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use dm_database_sqllog2db::cli::run::handle_run;
use dm_database_sqllog2db::config::Config;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

fn make_config(sqllog_dir: &Path, bench_dir: &Path) -> Config {
    let toml = format!(
        r#"
[sqllog]
directory = "{sqllog}"

[logging]
file = "{dir}/app.log"
level = "warn"
retention_days = 1

[exporter.csv]
file = "/dev/null"
overwrite = true
append = false
"#,
        sqllog = sqllog_dir.to_string_lossy().replace('\\', "/"),
        dir = bench_dir.to_string_lossy().replace('\\', "/"),
    );
    toml::from_str(&toml).unwrap()
}

fn bench_csv_export(c: &mut Criterion) {
    let bench_dir = bench_common::bench_target_dir("bench_csv");
    let sqllog_dir = bench_dir.join("sqllogs");
    fs::create_dir_all(&sqllog_dir).unwrap();

    let mut group = c.benchmark_group("csv_export");

    for &n in &[1_000usize, 10_000, 50_000] {
        fs::write(sqllog_dir.join("bench.log"), bench_common::synthetic_log(n)).unwrap();
        let cfg = make_config(&sqllog_dir, &bench_dir);

        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &cfg, |b, cfg| {
            b.iter(|| {
                handle_run(
                    cfg,
                    true,
                    false,
                    &Arc::new(AtomicBool::new(false)),
                    None, // compiled_filters
                )
                .unwrap();
            });
        });
    }

    group.finish();
}

fn bench_csv_real_file(c: &mut Criterion) {
    let real_dir = PathBuf::from("sqllogs");
    if !real_dir.exists() {
        eprintln!("sqllogs/ not found, skipping csv_export_real benchmark");
        return;
    }

    let bench_dir = bench_common::bench_target_dir("bench_csv_real");
    fs::create_dir_all(&bench_dir).unwrap();
    let cfg = make_config(&real_dir, &bench_dir);

    let mut group = c.benchmark_group("csv_export_real");
    // 真实文件慢，减少采样次数；measurement_time 给足单次测量窗口
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(60));
    // 记录数未预扫描，省略 Throughput::Elements，仅记录绝对时间
    group.bench_function("real_file", |b| {
        b.iter(|| {
            handle_run(
                &cfg,
                true,
                false,
                &Arc::new(AtomicBool::new(false)),
                None, // compiled_filters
            )
            .unwrap();
        });
    });
    group.finish();
}

/// Micro-benchmark：隔离 CSV 格式化层净开销（不含 `parse_meta`/`parse_performance_metrics`）。
///
/// 输入采用硬编码典型记录（D-03）：包含 ts, ep, sess, trxid, stmt, appname, ip, sql,
/// `EXECTIME`, `ROWCOUNT`, `EXEC_ID`。10000 条相同记录，与 `csv_export/10000` group 对齐，
/// 方便对比格式化层在总开销中的占比。
///
/// 注意：本 group 无 v1.0 baseline。**不要**用 `--baseline v1.0` 对比此 group。
fn bench_csv_format_only(c: &mut Criterion) {
    use dm_database_parser_sqllog::LogParserBuilder;
    use dm_database_sqllog2db::exporter::Exporter;
    use dm_database_sqllog2db::exporter::csv::CsvExporter;

    // D-03：硬编码典型记录（中等长度 SQL）
    const LOG_LINE: &str = "2024-01-01 00:00:00.000 (EP[1234] sess:0x0001 user:BENCHUSER trxid:TID001 stmt:0x1 appname:App ip:10.0.0.1) [SEL] SELECT * FROM t WHERE id = 1. EXECTIME: 10(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n";
    const N: usize = 10_000;

    let bench_dir = bench_common::bench_target_dir("bench_csv_format_only");
    fs::create_dir_all(&bench_dir).unwrap();
    let log_path = bench_dir.join("fmt.log");
    let content: String = LOG_LINE.repeat(N);
    fs::write(&log_path, &content).unwrap();

    // 一次性解析全部 N 条记录到 Vec，benchmark 内只跑格式化
    let parser = LogParserBuilder::new(log_path.to_str().unwrap())
        .build()
        .unwrap();
    let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();
    assert_eq!(
        records.len(),
        N,
        "expected {N} parsed records, got {}",
        records.len()
    );

    // v1.1.0: 所有字段已在 Sqllog 上物化，无需预解析 meta/pm
    let parsed: Vec<_> = records.iter().collect();

    let out_path = bench_dir.join("out.csv");

    let mut group = c.benchmark_group("csv_format_only");
    group.throughput(Throughput::Elements(N as u64));
    group.bench_function(BenchmarkId::from_parameter(N), |b| {
        b.iter(|| {
            let mut exporter = CsvExporter::new(&out_path);
            exporter.initialize().unwrap();
            for sqllog in &parsed {
                exporter.export_one_preparsed(sqllog, true, None).unwrap();
            }
            exporter.finalize().unwrap();
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_csv_export,
    bench_csv_real_file,
    bench_csv_format_only
);
criterion_main!(benches);
