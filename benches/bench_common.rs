// Shared helpers for all benchmarks.
//
// Import in each bench file:
//   `#[path = "bench_common.rs"] mod bench_common;`
// then call `bench_common::synthetic_log(n)` and `bench_common::bench_target_dir("name")`.

use std::path::PathBuf;

/// Build N synthetic `DaMeng` SQL log lines.
///
/// Each line has a unique session id, trxid, and `exec_time` derived from
/// the loop index so the parser sees a realistic variety of values.
/// Capacity is pre-allocated at 170 bytes per record (measured on the
/// canonical template below).
#[must_use]
pub fn synthetic_log(record_count: usize) -> String {
    use std::fmt::Write as _;
    let mut buf = String::with_capacity(record_count * 170);
    for i in 0..record_count {
        writeln!(
            buf,
            "2025-01-15 10:30:28.001 (EP[0] sess:0x{i:04x} user:BENCH trxid:{i} stmt:0x1 appname:BenchApp ip:10.0.0.{ip}) [SEL] SELECT col1, col2 FROM bench_table WHERE id={i}. EXECTIME: {exec}(ms) ROWCOUNT: {rows}(rows) EXEC_ID: {i}.",
            ip   = i % 256,
            exec = (i * 13) % 5000,
            rows = i % 1000,
        )
        .unwrap();
    }
    buf
}

/// Resolve a benchmark output directory under the Cargo target directory.
///
/// Uses `CARGO_TARGET_DIR` when set (e.g. in CI), falling back to `"target"`
/// so that `cargo bench` invoked from the project root still works without
/// any environment variable.
#[must_use]
pub fn bench_target_dir(name: &str) -> PathBuf {
    let target_root = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string());
    PathBuf::from(target_root).join(name)
}
