/// Baseline benchmark: parser throughput.
///
/// Measures the raw parsing speed of dm-database-parser-sqllog:
/// mmap file read + log line parsing → Sqllog records.
/// Excludes any exporter overhead (CSV / `SQLite`).
/// Run with: `cargo bench --bench bench_parser`
#[path = "bench_common.rs"]
mod bench_common;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use dm_database_parser_sqllog::LogParserBuilder;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;

fn bench_parser_throughput(c: &mut Criterion) {
    let bench_dir = PathBuf::from("target/bench_parser");
    let sqllog_dir = bench_dir.join("sqllogs");
    fs::create_dir_all(&sqllog_dir).unwrap();

    let mut group = c.benchmark_group("parser_throughput");

    for &n in &[1_000usize, 10_000, 50_000] {
        let log_path = sqllog_dir.join("bench.log");
        fs::write(&log_path, bench_common::synthetic_log(n)).unwrap();

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
