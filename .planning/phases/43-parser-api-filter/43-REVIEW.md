---
phase: "43"
status: has_findings
depth: standard
files_reviewed: 9
files_reviewed_list:
  - src/pipeline/filters/mod.rs
  - src/cli/run/prescan.rs
  - src/pipeline/mod.rs
  - src/cli/run/filter_processor.rs
  - src/cli/run/processor.rs
  - src/exporter/mod.rs
  - src/exporter/sqlite/write.rs
  - src/exporter/csv/writer.rs
  - src/pipeline/filters/compiled.rs
findings:
  critical: 1
  warning: 2
  info: 1
  total: 4
---

# Phase 43: Code Review Report

**Reviewed:** 2026-05-25T00:00:00Z
**Depth:** standard
**Files Reviewed:** 9
**Status:** has_findings

## Summary

Reviewed 9 source files implementing the filter pipeline, prescan orchestration, record processing loop, and CSV/SQLite exporters. The design is well-structured with clear separation between prescan (transaction-level) and main-pass (record-level) filtering, and the compiled filter approach avoids repeated regex compilation on the hot path.

Three concrete defects were found. The most serious is a silent data loss in the CSV exporter: `row_count` values are dropped when `exec_id == 0` and `exectime == 0.0`, while the SQLite exporter preserves them correctly. The other two are a behavioral divergence (fatal export errors do not short-circuit the inner processing loop) and a stat-reporting inaccuracy (failed exports are counted in the exported record total).

---

## Critical Issues

### CR-01: CSV exporter silently drops `row_count` when `exec_id=0` and `exectime=0.0`

**File:** `src/exporter/csv/writer.rs:74` and `src/exporter/csv/writer.rs:105`

**Issue:** The CSV writer uses `sqllog.exec_id != 0 || sqllog.exectime > 0.0` to decide whether to emit performance metric values. The SQLite writer at `src/exporter/sqlite/write.rs:14` uses `exec_id != 0 || exectime > 0.0 || sqllog.rowcount != 0`. The missing `rowcount` check in the CSV path means that any record with `rowcount > 0` but `exec_id == 0` and `exectime == 0.0` will produce three empty CSV columns instead of writing the real row count. This is a silent data loss — the row count from the log file is silently discarded, and the output differs between the two exporters for the same input.

**Fix:**
```rust
// writer.rs line 74 — ALL path
if sqllog.exec_id != 0 || sqllog.exectime > 0.0 || sqllog.rowcount != 0 {

// writer.rs line 105 — projected path
let has_metrics = sqllog.exec_id != 0 || sqllog.exectime > 0.0 || sqllog.rowcount != 0;
```

---

## Warnings

### WR-01: Fatal export error does not short-circuit the inner processing loop

**File:** `src/cli/run/processor.rs:107-121`

**Issue:** When `export_one_preparsed` returns a fatal error (e.g. `ExportError::DatabaseFailed`), the code logs the error and sets `file_stats.fatal_error`, but `map_or_else` returns `()` and execution falls through unconditionally to `records_in_file += 1` and continues to the next record. The outer caller in `mod.rs` only checks `file_stats.has_fatal()` after the entire file has been processed. For a broken SQLite connection, this means hundreds of thousands of further failed inserts are attempted before the error propagates. The loop should break immediately on fatal errors.

**Fix:**
```rust
// Replace the map_or_else block with a match that breaks on fatal errors
let export_result = exporter_manager
    .export_one_preparsed(&record, include_pm, ns);
match export_result {
    Err(e) if e.is_fatal() => {
        file_stats.set_fatal(e.to_string());
        eprintln!("[{}] {file_path}: {e}", e.severity());
        log::warn!("{file_path} | fatal export error: {e:?}");
        break 'outer;
    }
    Err(e) => {
        file_stats.add_export_error();
        eprintln!("[{}] {file_path}: {e}", e.severity());
        log::warn!("{file_path} | export error: {e:?}");
    }
    Ok(()) => {}
}
records_in_file += 1;
```

### WR-02: `records_in_file` increments unconditionally even when export fails

**File:** `src/cli/run/processor.rs:121`

**Issue:** `records_in_file += 1` runs unconditionally after `export_one_preparsed`, regardless of whether the export succeeded or failed. The comment on line 100 says "CR-02: avoid counting non-exported records" but the code does the opposite — failed exports are counted. This inflates the per-file record count returned to the caller, which is used both for the final summary message (`total_records`) and for the `pb.inc(1024)` progress bar. The WR-01 fix above (breaking on fatal) partially addresses this, but non-fatal export errors should also not increment the count.

**Fix:**
```rust
// Only increment on successful export
let export_result = exporter_manager.export_one_preparsed(&record, include_pm, ns);
if export_result.is_ok() {
    records_in_file += 1;
} else {
    // ... error handling ...
}
```

---

## Info

### IN-01: `scan_for_trxids_by_transaction_filters` thread pool error mapped to opaque `io::Error`

**File:** `src/cli/run/prescan.rs:70-71`

**Issue:** `rayon::ThreadPoolBuilder::build()` failure is mapped to `Error::Io(std::io::Error::other(e))`. The original `rayon::ThreadPoolBuildError` message is preserved inside the `io::Error`, but the outer error variant is `Error::Io` rather than a dedicated configuration or runtime error. If this error is ever displayed to the user, the `File error:` prefix (from `Error::Io` formatting) is misleading — the failure is not a file I/O problem. This is a minor diagnostic quality issue, not a correctness issue.

**Fix:** Either define a dedicated `Error::Runtime` variant or use a more descriptive context string:
```rust
.map_err(|e| Error::Io(std::io::Error::other(format!("rayon thread pool: {e}"))))
```

---

_Reviewed: 2026-05-25T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
