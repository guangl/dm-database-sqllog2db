/// Baseline benchmark: parser throughput.
///
/// Measures the raw parsing speed of dm-database-parser-sqllog:
/// mmap file read + log line parsing → Sqllog records.
/// Excludes any exporter overhead (CSV / `SQLite`).
/// Run with: `cargo bench --bench bench_parser`
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use dm_database_parser_sqllog::LogParserBuilder;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;

/// Build N synthetic `DaMeng` SQL log lines.
fn synthetic_log(record_count: usize) -> String {
    use std::fmt::Write as _;
    let mut buf = String::with_capacity(record_count * 170);
    for i in 0..record_count {
        writeln!(
            buf,
            "2025-01-15 10:30:28.001 (EP[0] sess:0x{i:04x} user:BENCH trxid:{i} stmt:0x1 appname:BenchApp ip:10.0.0.{ip}) [SEL] SELECT col1, col2 FROM bench_table WHERE id={i} AND status='active'. EXECTIME: {exec}(ms) ROWCOUNT: {rows}(rows) EXEC_ID: {i}.",
            ip   = i % 256,
            exec = (i * 13) % 5000,
            rows = i % 1000,
        )
        .unwrap();
    }
    buf
}

fn bench_parser_throughput(c: &mut Criterion) {
    let bench_dir = PathBuf::from("target/bench_parser");
    let sqllog_dir = bench_dir.join("sqllogs");
    fs::create_dir_all(&sqllog_dir).unwrap();

    let mut group = c.benchmark_group("parser_throughput");

    for &n in &[1_000usize, 10_000, 50_000] {
        let log_path = sqllog_dir.join("bench.log");
        fs::write(&log_path, synthetic_log(n)).unwrap();

        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &log_path, |b, path| {
            b.iter(|| {
                let parser = LogParserBuilder::new(black_box(path.to_str().unwrap()))
                    .build()
                    .unwrap();
                black_box(parser.iter().filter_map(std::result::Result::ok).count())
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_parser_throughput);
criterion_main!(benches);
