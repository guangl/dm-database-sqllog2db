---
phase: "42"
fixed_at: 2026-05-25T00:00:00Z
review_path: .planning/phases/42-criterion/42-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 42: Code Review Fix Report

**Fixed at:** 2026-05-25
**Source review:** .planning/phases/42-criterion/42-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 4
- Fixed: 4
- Skipped: 0

## Fixed Issues

### WR-01: Missing `criterion::black_box` — benchmark result may be optimized away

**Files modified:** `benches/bench_parser.rs`
**Commit:** ed7b93e
**Applied fix:** Added `use std::hint::black_box` (using `std::hint` instead of the deprecated `criterion::black_box`) and wrapped both the path input and `.count()` return value in `black_box(...)` to prevent LLVM from eliding the hot loop as dead code.

### WR-02: Synthetic log format inconsistency with sibling benchmarks

**Files modified:** `benches/bench_parser.rs`
**Commit:** f2d088e
**Applied fix:** Removed the extra `AND status='active'` predicate from the SQL template in `bench_parser.rs`, aligning it with `bench_filters.rs`. This makes records-per-second figures comparable across benchmarks.

### IN-01: `synthetic_log` function duplicated across benchmark files

**Files modified:** `benches/bench_common.rs` (new), `benches/bench_parser.rs`, `benches/bench_csv.rs`, `benches/bench_filters.rs`, `benches/bench_sqlite.rs`
**Commit:** 63b98c6
**Applied fix:** Created `benches/bench_common.rs` with a single canonical `synthetic_log` function (no `AND status='active'`). All four benchmark files now import via `#[path = "bench_common.rs"] mod bench_common` and call `bench_common::synthetic_log(n)`. This also fixed `bench_csv.rs` and `bench_sqlite.rs` which still had the extra clause.

### IN-02: Relative `target/bench_parser` path depends on CWD

**Files modified:** `benches/bench_common.rs`, `benches/bench_parser.rs`, `benches/bench_csv.rs`, `benches/bench_filters.rs`, `benches/bench_sqlite.rs`
**Commit:** f94f3e6
**Applied fix:** Added `bench_target_dir(name: &str) -> PathBuf` helper to `bench_common.rs` that reads `$CARGO_TARGET_DIR` (falling back to `"target"`) for resolving benchmark output directories. Updated all four bench files to use `bench_common::bench_target_dir("bench_name")` instead of `PathBuf::from("target/bench_name")`. Removed now-unused `use std::path::PathBuf` imports from files that no longer reference `PathBuf` by name.

---

_Fixed: 2026-05-25_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
