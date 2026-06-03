---
phase: 59-cli-run-exporter-pipeline
reviewed: 2026-06-03T12:00:00Z
depth: standard
files_reviewed: 6
files_reviewed_list:
  - src/cli/run/collector.rs
  - src/cli/run/filter_processor.rs
  - src/cli/run/mod.rs
  - src/cli/run/parallel.rs
  - src/cli/run/processor.rs
  - src/cli/run/sqlite_parallel.rs
findings:
  critical: 1
  warning: 3
  info: 2
  total: 6
status: issues_found
---

# Phase 59: Code Review Report

**Reviewed:** 2026-06-03T12:00:00Z
**Depth:** standard
**Files Reviewed:** 6
**Status:** issues_found

## Summary

Reviewed six files covering the parallel and sequential CLI run orchestration, filter processing, record collection, and output finalization. The overall decomposition is sound — `collector.rs` correctly mirrors `processor.rs` PARAMS-buffer logic, and the two-phase prescan design is coherent.

One blocker was found: `concat_csv_parts` deletes a part file while its `BufReader` is still in scope. On Windows this causes `ERROR_SHARING_VIOLATION` (`os error 32`), making the entire parallel CSV export path fail at the cleanup step. Since the codebase already has Windows-specific guards in `mod.rs`, Windows is an intended platform.

Three warnings cover: a missing defensive guard inside `normalize_and_export` (contract mismatch with its doc comment), a fatal-error classification error in `run_file_loop` (wrong error variant, wrong path), and unresponsive interrupt handling for filtered workloads.

Two info items cover a dead-parameter pattern in the parallel function signatures and a merge-before-return ordering oddity in `run_file_loop`.

---

## Critical Issues

### CR-01: `concat_csv_parts` deletes a temp file while its reader is still open — fails on Windows

**File:** `src/cli/run/parallel.rs:59`

**Issue:** Inside the `for` loop, `reader` (`BufReader<File>`) is created at line 47, used by `std::io::copy` at line 58, and is still live (not dropped) when `std::fs::remove_file(part_path)?` executes at line 59. On POSIX systems this is harmless (unlink semantics). On Windows, `remove_file` on an open file handle returns `os error 32` (`ERROR_SHARING_VIOLATION`), which propagates via `?` and aborts the concat.

The result: on Windows, the parallel CSV export always fails during the finalization step after all records have been successfully parsed and written to part files. The output CSV is left partially written (only the first part is concatenated before the error occurs on `remove_file`). The subsequent cleanup in `finalize_concat` attempts to delete the partial output file with `let _ = std::fs::remove_file(output_path)`, which may silently leave a truncated CSV behind.

This is confirmed as an intended cross-platform scenario: `mod.rs` already contains `#[cfg(target_os = "windows")]` guards for stdin-pipe detection, demonstrating that Windows is a supported target.

**Fix:** Drop the reader explicitly before removing the file, or collect paths for deferred removal after `writer.flush()`:

```rust
// Option A: drop reader before remove_file
std::io::copy(&mut reader, &mut writer)?;
drop(reader);                          // close the file handle before unlinking
std::fs::remove_file(part_path)?;

// Option B: defer all removals until after flush (also fixes the flush-before-remove ordering)
let mut parts_to_remove: Vec<&Path> = Vec::new();
for (idx, (part_path, _)) in parts.iter().enumerate() {
    let part_file = std::fs::File::open(part_path)?;
    let mut reader = BufReader::new(part_file);
    let skip_header = idx > 0 || append_to_existing;
    if skip_header {
        let mut discard = Vec::with_capacity(256);
        std::io::BufRead::read_until(&mut reader, b'\n', &mut discard)?;
    }
    std::io::copy(&mut reader, &mut writer)?;
    parts_to_remove.push(part_path);
}
writer.flush()?;
for p in parts_to_remove {
    if let Err(e) = std::fs::remove_file(p) {
        log::warn!("failed to remove temp part {}: {e}", p.display());
    }
}
```

Option B is preferred as it also ensures flush completes before any cleanup, preventing a truncated output file from masquerading as complete.

---

## Warnings

### WR-01: `normalize_and_export` unconditionally calls `update_params_buffer_only` when `!passes`, without guarding on `do_normalize`

**File:** `src/cli/run/processor.rs:59-62`

**Issue:** The function's own doc comment states: "仅在 `passes==false && do_normalize && record.tag.is_none()` 时更新 `params_buffer`". The implementation does not enforce the `do_normalize` guard:

```rust
if !passes {
    update_params_buffer_only(record, params_buffer, placeholder_override, ns_scratch);
    //                        ^ no do_normalize check here
    return ExportAction::Continue;
}
```

The caller (`process_log_file`, line 195) prevents reaching this branch when `do_normalize=false` via the `needs_processing` guard, so there is no current runtime bug. However:

- The function is not self-defensive: any future caller that invokes `normalize_and_export` directly with `do_normalize=false` (a plausible pattern if the function gains new call sites) will silently corrupt `params_buffer` by inserting PARAMS into it when normalization is disabled, causing wrong parameter substitutions in later DML records within the same session.
- `collector.rs::process_record` (the parallel-path equivalent) correctly applies the guard in its else-branch (line 92-99). The two implementations now have divergent contracts, making the codebase harder to reason about.

**Fix:** Add the missing guards inside `normalize_and_export`:

```rust
if !passes {
    if do_normalize && record.tag.is_none() {
        update_params_buffer_only(record, params_buffer, placeholder_override, ns_scratch);
    }
    return ExportAction::Continue;
}
```

---

### WR-02: Fatal export error in `run_file_loop` is re-wrapped as the wrong variant with the wrong path

**File:** `src/cli/run/mod.rs:374-379`

**Issue:** When a fatal export error occurs (SQLite `DatabaseFailed`), `normalize_and_export` stores the error message via `file_stats.set_fatal(e.to_string())` and returns `BreakFatal`. `run_file_loop` then reconstructs the error as:

```rust
return Err(Error::Export(crate::error::ExportError::WriteFailed {
    path: log_file.into(),          // input log file — wrong path
    reason: file_stats.fatal_error.unwrap_or_default(),
}));
```

Two concrete defects:

1. **Wrong variant.** `ExportError::WriteFailed` has `is_fatal() == false` (only `ExportError::DatabaseFailed` is fatal, per `error.rs:101`). The reconstructed error's severity downgrades from `Critical` to `Error`, and `suggestion()` gives generic CSV advice ("Check disk space") instead of the SQLite suggestion ("Verify the SQLite database file is accessible").

2. **Wrong path.** The `path` field is set to the input `.log` file being processed at the time of the fatal error, not the SQLite database path. This misattributes the error in log output and in any upstream error handler that extracts the path.

**Fix:** Preserve the original error type. Since only `DatabaseFailed` is currently fatal for export errors, use that variant directly:

```rust
if file_stats.has_fatal() {
    return Err(Error::Export(crate::error::ExportError::DatabaseFailed {
        reason: file_stats.fatal_error.unwrap_or_default(),
    }));
}
```

For a more general solution, store the original `Error` in `ErrorStats` rather than a lossy `String`, so the variant and path survive the round-trip.

---

### WR-03: Interrupt check in `process_log_file` is gated on `passes`, leaving Ctrl+C unresponsive during heavily-filtered single-file runs

**File:** `src/cli/run/processor.rs:204`

**Issue:**

```rust
ExportAction::Continue if passes && tick_progress(pb, records_in_file, interrupted) => break 'outer,
```

`tick_progress` — and therefore the `interrupted` flag check — is only evaluated when `passes == true`. For records that are filtered out (`passes == false`), the interrupt path is entirely skipped. In the sequential path, the only other interrupt check in this function is at the start of each file (in `run_file_loop` line 352). If a user is processing a single large file where most or all records are filtered out, `Ctrl+C` will not be honored until the file finishes parsing, which may take minutes on a 1 GB+ log file.

The parallel path (`collector.rs:39`) correctly checks the interrupt flag for every record (`if interrupted.load(...) { break; }`), making the parallel path more responsive.

**Fix:** Add a separate interrupt check in the filtered path, independent of `tick_progress`:

```rust
match action {
    ExportAction::BreakQuota | ExportAction::BreakFatal => break 'outer,
    ExportAction::Continue if passes && tick_progress(pb, records_in_file, interrupted) => break 'outer,
    ExportAction::Continue => {}
}
// Check interrupt for filtered records (passes=false) on the same schedule
if !passes && records_in_file.trailing_zeros() >= 10
    && interrupted.load(Ordering::Relaxed)
{
    break 'outer;
}
```

Or, simpler: move the interrupt check out of `tick_progress` and evaluate it unconditionally at some cadence (e.g., every 1024 total records processed, not just exported records).

---

## Info

### IN-01: `show_progress` / `_show_progress` parameter is accepted by both parallel inner functions but silently discarded

**File:** `src/cli/run/parallel.rs:268`, `src/cli/run/sqlite_parallel.rs:109`

**Issue:** `process_csv_parallel` receives `show_progress: bool` and immediately discards it with `let _ = show_progress;`. `process_sqlite_parallel` uses `_show_progress: bool` (underscore-prefixed). Neither function implements any progress output. The `verbose` parameter that the outer wrappers (`run_csv_parallel`, `run_sqlite_parallel`) consume is not forwarded to the inner functions either.

This leaves parallel mode with no progress or per-file verbose output, while the sequential path provides both. Dead parameters in function signatures add noise and mislead maintainers into thinking progress output is implemented.

**Fix:** Either implement progress in parallel mode or remove the dead parameters from both inner-function signatures and document that parallel mode does not currently support progress. The outer wrappers can retain verbose logging of the "N files, M jobs" summary line.

---

### IN-02: `run_file_loop` merges `file_stats` into `run_stats` before checking for fatal error, making the merge dead on the error path

**File:** `src/cli/run/mod.rs:373-379`

**Issue:**

```rust
run_stats.merge(&file_stats);      // line 373 — merged unconditionally
if file_stats.has_fatal() {        // line 374
    return Err(...);               // run_stats is dropped here; merge was wasted
}
```

When a fatal error occurs, `run_stats` is discarded because the function returns `Err`. The `merge` on the fatal path is dead code — the statistics it accumulates are never seen by any caller.

This does not cause incorrect behavior today, but it is misleading to a reader: the merge appears to accumulate data that will be surfaced, when in fact it is silently dropped.

**Fix:** Check for fatal before merging, so the merge only runs on paths where `run_stats` will be returned:

```rust
if file_stats.has_fatal() {
    return Err(Error::Export(crate::error::ExportError::DatabaseFailed {
        reason: file_stats.fatal_error.unwrap_or_default(),
    }));
}
run_stats.merge(&file_stats);
```

---

_Reviewed: 2026-06-03T12:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
