#![cfg(not(target_os = "windows"))]
//! jemalloc 峰值堆分配基线测试
//!
//! 目的：在 Wave 0 采集 v1.10（优化前）的堆分配基线数值，供 Wave 1 优化后对比（PERF-02）。
//!
//! 隔离说明：
//! - 本文件为 integration test，仅在 `cargo test` 时编译（tests/ 目录下文件始终处于 test 环境）。
//! - `#[global_allocator]` 放在测试文件中只替换测试进程的 allocator，不影响 release binary（D-03）。
//! - `tikv-jemallocator` 是 dev-dependency，这才是真正防止其进入 release 编译图的原因（非 cfg(test)）。

// SAFETY: 仅在测试环境替换全局 allocator，不影响生产 binary（D-03）。
// 注意：此处无需 #[cfg(test)]，tests/ 中的文件本身即为 test-only 编译单元。
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use dm_database_sqllog2db::engine::run as handle_run;
use dm_database_sqllog2db::config::Config;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tempfile::TempDir;

/// 测量闭包执行期间的 jemalloc 内存使用变化。
///
/// 同时返回 `(allocated_delta, resident_delta)`：
/// - `allocated`：当前活跃分配字节数。完整操作执行后临时内存释放，delta 可能为 0。
/// - `resident`：物理映射字节数。`jemalloc` 延迟归还内存给 OS，更能反映峰值内存压力。
///
/// 每次 `read` 前调用 `epoch.advance()` 刷新统计，防止读到陈旧数据（Pitfall 5 防护）。
fn measure_alloc_delta(f: impl FnOnce()) -> (usize, usize) {
    use tikv_jemalloc_ctl::{epoch, stats};
    let epoch_mib = epoch::mib().unwrap();
    let alloc_mib = stats::allocated::mib().unwrap();
    let resident_mib = stats::resident::mib().unwrap();
    epoch_mib.advance().unwrap();
    let before_alloc = alloc_mib.read().unwrap();
    let before_resident = resident_mib.read().unwrap();
    f();
    epoch_mib.advance().unwrap();
    let after_alloc = alloc_mib.read().unwrap();
    let after_resident = resident_mib.read().unwrap();
    (
        after_alloc.saturating_sub(before_alloc),
        after_resident.saturating_sub(before_resident),
    )
}

/// 生成 N 条合成 `DaMeng` SQL 日志行（与 `bench_csv.rs` 的 `synthetic_log` 字节级一致）。
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

/// 构建指向 `sqllog_dir` 的 `Config`（CSV 导出到 `/dev/null`）。
fn make_config(sqllog_dir: &std::path::Path, bench_dir: &std::path::Path) -> Config {
    let toml = format!(
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
    );
    toml::from_str(&toml).unwrap()
}

/// PERF-02 baseline measurement — Wave 0
///
/// 使用 jemalloc 统计测量处理 `10_000` 条合成日志记录的内存使用 delta，
/// 输出 v1.10（优化前）基线数值供 Wave 1 对比。
///
/// 注意：`stats.allocated` 是当前活跃分配字节数（非累计值）。完整的 `handle_run` 执行后
/// 临时内存已释放，allocated delta 可能为 0。同时输出 `resident` 值作为补充指标。
///
/// 运行方式：`cargo test test_jemalloc_peak_baseline -- --nocapture`
#[tokio::test(flavor = "multi_thread")]
async fn test_jemalloc_peak_baseline() {
    use tikv_jemalloc_ctl::{epoch, stats};

    // Verify jemalloc is active and stats are accessible
    let epoch_mib = epoch::mib().expect("jemalloc epoch::mib() failed — is stats feature enabled?");
    let alloc_mib = stats::allocated::mib()
        .expect("jemalloc stats::allocated::mib() failed — is stats feature enabled?");
    let resident_mib = stats::resident::mib()
        .expect("jemalloc stats::resident::mib() failed — is stats feature enabled?");

    // Read initial state before measurement
    epoch_mib.advance().unwrap();
    let initial_allocated = alloc_mib.read().unwrap();
    let initial_resident = resident_mib.read().unwrap();

    let tmp = TempDir::new().unwrap();
    let sqllog_dir = tmp.path().join("sqllogs");
    std::fs::create_dir_all(&sqllog_dir).unwrap();
    std::fs::write(sqllog_dir.join("bench.log"), synthetic_log(10_000)).unwrap();

    let cfg = make_config(&sqllog_dir, tmp.path());
    let interrupted = Arc::new(AtomicBool::new(false));

    let (allocated_delta, resident_delta) = measure_alloc_delta(|| {
        // block_in_place is required because measure_alloc_delta takes a synchronous
        // closure, so we cannot .await handle_run directly. block_in_place yields the
        // tokio worker thread to the blocking pool, allowing block_on to drive the
        // async handle_run without nesting runtimes.
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(handle_run(
                &cfg,
                true,
                false,
                &interrupted,
                None,
            ))
        })
        .unwrap();
    });

    // Final state for absolute reporting
    epoch_mib.advance().unwrap();
    let final_allocated = alloc_mib.read().unwrap();
    let final_resident = resident_mib.read().unwrap();

    // Note: stats.allocated is current active allocations (not cumulative).
    // handle_run frees temporary memory on completion, so allocated_delta may be 0.
    // We use resident_delta as the primary heap pressure indicator for PERF-02,
    // since jemalloc retains physical pages after deallocation (lazy return to OS).
    let heap_pressure = if allocated_delta > 0 {
        allocated_delta
    } else {
        resident_delta
    };
    println!("[phase44-before] jemalloc allocated delta (10000 records) = {heap_pressure} bytes");
    println!(
        "[phase44-before] allocated_delta = {allocated_delta} bytes (active allocs freed after run)"
    );
    println!("[phase44-before] resident_delta  = {resident_delta} bytes (retained physical pages)");
    println!(
        "[phase44-before] absolute: allocated {initial_allocated} -> {final_allocated} bytes, resident {initial_resident} -> {final_resident} bytes"
    );

    // jemalloc must be active: resident > 0 confirms allocator is working
    assert!(
        final_resident > 0,
        "jemalloc resident must be positive — allocator may not be active"
    );
    // Note: heap_pressure (delta-based) is intentionally not asserted here.
    // On warm runs both allocated_delta and resident_delta can be 0 because:
    //   - allocated_delta: handle_run frees temporary memory before returning
    //   - resident_delta: jemalloc reuses already-mapped pages (lazy OS return)
    // This test is a *measurement* baseline (PERF-02), not a correctness assertion.
    // The liveness check above (final_resident > 0) is the only reliable guard.
    let _ = heap_pressure; // consumed by println above; no assertion on delta values
}
