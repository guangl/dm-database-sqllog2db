---
phase: 66-compat
reviewed: 2026-06-04T00:00:00Z
depth: standard
files_reviewed: 1
files_reviewed_list:
  - tests/integration.rs
findings:
  critical: 0
  warning: 3
  info: 2
  total: 5
status: issues_found
---

# Phase 66: Code Review Report

**Reviewed:** 2026-06-04
**Depth:** standard
**Files Reviewed:** 1
**Status:** issues_found

## Summary

`tests/integration.rs` is a large (2396-line) integration test file covering CLI handlers (`handle_run`, `handle_init`, `handle_validate`), E2E pipeline, boundary conditions, stats subcommand, and new Phase 66 compatibility tests (`COMPAT-01/02/03`). The new COMPAT tests themselves (lines 2160–2396) are structurally sound. The issues found are spread across pre-existing tests and affect test reliability rather than production code. No security issues, data-loss risks, or correctness bugs in production logic were found via the test file.

## Warnings

### WR-01: Three filter-pipeline tests have no output assertions — silent filter regressions go undetected

**File:** `tests/integration.rs:411`, `tests/integration.rs:434`, `tests/integration.rs:459`
**Issue:** `test_handle_run_with_filters_builds_pipeline`, `test_handle_run_with_transaction_filters_prescans`, and `test_handle_run_with_min_runtime_filter` each call `handle_run` with a filter configuration but never read or assert on the produced CSV output. A filter bug that causes zero records to be written (or all records to pass when they should be rejected) would not be caught by any of these three tests. The tests only verify that `handle_run` returns `Ok(...)`.

- `test_handle_run_with_filters_builds_pipeline` (line 411): `include.users = ["TESTUSER"]`, all 20 records match → should produce 21 lines (header + 20). Not checked.
- `test_handle_run_with_transaction_filters_prescans` (line 434): `exec_ids = [0, 1, 2]`, 30 records → should produce 4 lines (header + 3). Not checked.
- `test_handle_run_with_min_runtime_filter` (line 459): `min_runtime_ms = 1`, 20 records, EXECTIME varies → expected filtered count never verified.

**Fix:** Add a CSV read-back assertion after each `handle_run` call, for example:
```rust
// test_handle_run_with_filters_builds_pipeline
handle_run(&cfg, true, false, &interrupted).unwrap();
let content = std::fs::read_to_string(&csv_file).unwrap();
assert_eq!(content.lines().count(), 21, "expected header + 20 matching records");

// test_handle_run_with_transaction_filters_prescans
handle_run(&cfg, true, false, &interrupted).unwrap();
let content = std::fs::read_to_string(&csv_file).unwrap();
assert_eq!(content.lines().count(), 4, "expected header + 3 records matching exec_ids [0,1,2]");
```

### WR-02: `test_stats_csv_top_5_limits_rows` uses `<=` assertion — accepts zero rows silently

**File:** `tests/integration.rs:1601-1614`
**Issue:** The test feeds 8 records with `--top 5` and asserts `slow_data <= 5` and `freq_data <= 5`. The `<=` predicate passes if 0 rows are emitted (e.g., if the stats exporter silently drops all records or the file is never created). A broken `--top` implementation that produces an empty stats file would still satisfy this assertion.

**Fix:** Use an exact equality or bounded-range assertion:
```rust
assert!(
    slow_data >= 1 && slow_data <= 5,
    "slow_sql.csv data rows should be 1..=5, got {slow_data}"
);
assert!(
    freq_data >= 1 && freq_data <= 5,
    "frequent_sql.csv data rows should be 1..=5, got {freq_data}"
);
```

### WR-03: Misleading test comment creates false expectations about file layout

**File:** `tests/integration.rs:791`
**Issue:** The comment at line 791 reads:
```
// Arrange: 2 条正常行 + 1 条无效行 + 2 条正常行 = 4 条正常记录
```
The actual code writes 1 invalid line followed by 4 valid lines (a loop `for i in 0..4`). There are not "2+2" valid lines around a bad line — the invalid line is only at the top, and there are 4 contiguous valid lines after it. The assertion (5 total = header + 4) is correct, but the comment creates a false picture of the test layout that could mislead future maintainers.

**Fix:** Correct the comment to match the actual structure:
```rust
// Arrange: 1 条无效行（文件开头）+ 4 条正常行 = 4 条正常记录
```

## Info

### IN-01: `content.lines().count()` called twice in several `assert_eq!` error messages

**File:** `tests/integration.rs:103-108`, `tests/integration.rs:641-646`
**Issue:** Several assertions call `content.lines().count()` as both the left-hand side of `assert_eq!` and again inside the error message string. `assert_eq!` already prints both actual and expected values on failure, making the redundant `content.lines().count()` in the format string dead computation. Example at line 103:
```rust
assert_eq!(
    content.lines().count(),
    11,
    "expected header + 10 data rows, got {}",
    content.lines().count()   // ← redundant, assert_eq already shows this
);
```

**Fix:** Remove the redundant count from the format string argument:
```rust
assert_eq!(content.lines().count(), 11, "expected header + 10 data rows");
```

### IN-02: `write_test_log` generates duplicate record values across files — weakens COMPAT-02 deduplication detection

**File:** `tests/integration.rs:2174-2176` (used in `test_parallel_csv_content_matches_sequential` and `test_parallel_csv_filter_matches_sequential`)
**Issue:** `write_test_log` generates records using `i in 0..count`. Both `file_a` and `file_b` are written with the same 20 records (identical `trxid`, `EXEC_ID`, `sess` values). When sorted, each data line from `file_a` is an exact duplicate of the corresponding line from `file_b`. If the parallel path silently deduplicated records (e.g., due to a `UNIQUE` constraint or hash-set insertion), the sorted output would still have 40 matching lines (20 unique × 2 each). The test would pass even if a deduplication bug removed half the records, as long as both halves are removed symmetrically. Using distinct record ranges per file (e.g., `write_test_log(&file_a, 0..20)` and `write_test_log(&file_b, 20..40)` if the helper supported an offset) would make accidental deduplication detectable.

**Fix (recommended):** Add a second helper or parameter to `write_test_log` that accepts a start offset, then use non-overlapping ranges:
```rust
write_test_log_offset(&file_a, 20, 0);   // records 0..19
write_test_log_offset(&file_b, 20, 20);  // records 20..39
```

---

_Reviewed: 2026-06-04_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
