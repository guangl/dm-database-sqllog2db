/// Baseline benchmark: filter pipeline overhead.
///
/// Compares seven scenarios against the no-filter fast path:
///
/// | scenario                | what it measures                                         |
/// |-------------------------|----------------------------------------------------------|
/// | `no_pipeline`           | raw parse+export speed (fast path, zero overhead)        |
/// | `pipeline_passthrough`  | pipeline present but no record filtered out              |
/// | `trxid_small`           | exact trxid match against 10 IDs (`HashSet` O(1))        |
/// | `trxid_large`           | exact trxid match against 1 000 IDs (`HashSet` O(1))     |
/// | `indicator_prescan`     | two-pass: pre-scan by `min_runtime_ms` + main pass        |
/// | `exclude_passthrough`   | exclude config present but zero hits (pure overhead)     |
/// | `exclude_active`        | all records excluded by OR-veto (100% hit rate)          |
///
/// Run with: `cargo bench --bench bench_filters`
#[path = "bench_common.rs"]
mod bench_common;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use dm_database_sqllog2db::engine::run as handle_run;
use dm_database_sqllog2db::config::Config;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

const RECORDS: usize = 10_000;

fn base_toml(sqllog_dir: &Path, bench_dir: &Path) -> String {
    format!(
        r#"
[sqllog]
inputs = ["{sqllog}"]

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
    )
}

fn cfg_no_pipeline(sqllog_dir: &Path, bench_dir: &Path) -> Config {
    toml::from_str(&base_toml(sqllog_dir, bench_dir)).unwrap()
}

/// Filters enabled but `start_ts` is in the distant past → every record passes.
fn cfg_pipeline_passthrough(sqllog_dir: &Path, bench_dir: &Path) -> Config {
    let toml = format!(
        "{base}
[filter]
enable = true

[filter.include]
start_ts = \"2000-01-01\"
",
        base = base_toml(sqllog_dir, bench_dir)
    );
    toml::from_str(&toml).unwrap()
}

/// Exact trxid match against a small set (10 IDs).
/// Only records with trxid in [0..10] are kept.
fn cfg_trxid_small(sqllog_dir: &Path, bench_dir: &Path) -> Config {
    let ids: Vec<String> = (0..10).map(|i: usize| format!("\"{i}\"")).collect();
    let toml = format!(
        "{base}
[filter]
enable = true

[filter.include]
trxids = [{ids}]
",
        base = base_toml(sqllog_dir, bench_dir),
        ids = ids.join(", "),
    );
    toml::from_str(&toml).unwrap()
}

/// Exact trxid match against a large set (1 000 IDs) — validates `HashSet` O(1) benefit.
/// Matches the first 1 000 trxids out of `RECORDS`.
fn cfg_trxid_large(sqllog_dir: &Path, bench_dir: &Path) -> Config {
    let ids: Vec<String> = (0..1_000).map(|i: usize| format!("\"{i}\"")).collect();
    let toml = format!(
        "{base}
[filter]
enable = true

[filter.include]
trxids = [{ids}]
",
        base = base_toml(sqllog_dir, bench_dir),
        ids = ids.join(", "),
    );
    toml::from_str(&toml).unwrap()
}

/// Transaction-level filter using `min_runtime_ms` — triggers the two-pass pre-scan.
/// Records with `exec_time` >= 2000 ms pass (roughly 60% of the synthetic set).
fn cfg_indicator_prescan(sqllog_dir: &Path, bench_dir: &Path) -> Config {
    let toml = format!(
        "{base}
[filter]
enable = true

[filter.indicators]
min_runtime_ms = 2000
",
        base = base_toml(sqllog_dir, bench_dir)
    );
    toml::from_str(&toml).unwrap()
}

/// exclude 配置存在但无记录命中（纯排除过滤开销）。
/// `synthetic_log` 中 username 固定为 `BENCH`，
/// exclude 配置为 `["BENCH_EXCLUDE"]` → 零命中。
fn cfg_exclude_passthrough(sqllog_dir: &Path, bench_dir: &Path) -> Config {
    let toml = format!(
        "{base}
[filter]
enable = true

[filter.exclude]
users = [\"BENCH_EXCLUDE\"]
",
        base = base_toml(sqllog_dir, bench_dir)
    );
    toml::from_str(&toml).unwrap()
}

/// exclude 命中所有记录（100% hit rate）— OR-veto 极端压力场景。
/// exclude 配置为 `["BENCH"]`，`synthetic_log` 中所有记录 username = `BENCH`，全部被排除。
fn cfg_exclude_active(sqllog_dir: &Path, bench_dir: &Path) -> Config {
    let toml = format!(
        "{base}
[filter]
enable = true

[filter.exclude]
users = [\"BENCH\"]
",
        base = base_toml(sqllog_dir, bench_dir)
    );
    toml::from_str(&toml).unwrap()
}

fn bench_filters(c: &mut Criterion) {
    let bench_dir = bench_common::bench_target_dir("bench_filters");
    let sqllog_dir = bench_dir.join("sqllogs");
    fs::create_dir_all(&sqllog_dir).unwrap();
    fs::write(
        sqllog_dir.join("bench.log"),
        bench_common::synthetic_log(RECORDS),
    )
    .unwrap();

    let scenarios: &[(&str, Config)] = &[
        ("no_pipeline", cfg_no_pipeline(&sqllog_dir, &bench_dir)),
        (
            "pipeline_passthrough",
            cfg_pipeline_passthrough(&sqllog_dir, &bench_dir),
        ),
        ("trxid_small", cfg_trxid_small(&sqllog_dir, &bench_dir)),
        ("trxid_large", cfg_trxid_large(&sqllog_dir, &bench_dir)),
        (
            "indicator_prescan",
            cfg_indicator_prescan(&sqllog_dir, &bench_dir),
        ),
        (
            "exclude_passthrough",
            cfg_exclude_passthrough(&sqllog_dir, &bench_dir),
        ),
        (
            "exclude_active",
            cfg_exclude_active(&sqllog_dir, &bench_dir),
        ),
    ];

    let mut group = c.benchmark_group("filters");
    group.throughput(Throughput::Elements(RECORDS as u64));

    for (name, cfg) in scenarios {
        group.bench_with_input(BenchmarkId::from_parameter(name), cfg, |b, cfg| {
            b.iter(|| {
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(handle_run(
                        cfg,
                        true,
                        false,
                        &Arc::new(AtomicBool::new(false)),
                        None,
                    ))
                    .unwrap();
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_filters);
criterion_main!(benches);
