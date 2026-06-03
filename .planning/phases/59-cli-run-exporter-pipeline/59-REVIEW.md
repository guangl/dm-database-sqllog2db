---
phase: 59-cli-run-exporter-pipeline
reviewed: 2026-06-03T00:00:00Z
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
  critical: 2
  warning: 3
  info: 1
  total: 6
status: issues_found
---

# Phase 59: Code Review Report

**Reviewed:** 2026-06-03T00:00:00Z
**Depth:** standard
**Files Reviewed:** 6
**Status:** issues_found

## Summary

This phase refactored `cli/run` by splitting oversized functions and extracting a shared `collector.rs`. The high-level structure is sound, but two correctness defects were introduced or surfaced during the refactor. The first is a semantic divergence between `collector.rs::process_record` and `processor.rs::normalize_and_export` in the filtered-PARAMS path that produces divergent `params_buf` state under certain conditions. The second is a fatal-error re-wrapping in `run_file_loop` that misclassifies a database-fatal error as a non-fatal `WriteFailed` variant, misattributing it to the wrong file path and downgrading the severity label printed to stderr.

Three warnings cover a latent guard omission in `normalize_and_export`, a temp-file partial-write hazard in `concat_csv_parts`, and a dead `verbose` parameter in the parallel wrappers.

---

## Critical Issues

### CR-01: Fatal database error re-wrapped as non-fatal `ExportError::WriteFailed` in `run_file_loop`

**File:** `src/cli/run/mod.rs:374-378`

**Issue:** When a fatal export error occurs (e.g., SQLite `DatabaseFailed`), `normalize_and_export` calls `file_stats.set_fatal(e.to_string())` and breaks the loop. `run_file_loop` then detects `file_stats.has_fatal()` and reconstructs the error as `Error::Export(ExportError::WriteFailed { path: log_file, reason: ... })`.

Two concrete defects result:

1. **Wrong error variant.** `ExportError::WriteFailed` has `is_fatal() == false` (only `ExportError::DatabaseFailed` is fatal per `error.rs:101`). The reconstructed error's severity is `ErrorSeverity::Error` instead of `Critical`, so stderr prints `[ERROR]` where `[CRITICAL]` is expected.

2. **Wrong path in the error.** The `path` field is set to `log_file` (the input log file being processed), not the SQLite database path. A database write failure is attributed to the wrong file in the error message.

**Fix:**
```rust
if file_stats.has_fatal() {
    // Preserve the original fatal classification. Database failures must
    // propagate as DatabaseFailed so is_fatal() and severity() stay correct.
    return Err(Error::Export(crate::error::ExportError::DatabaseFailed {
        reason: file_stats.fatal_error.unwrap_or_default(),
    }));
}
```

If there are non-database fatal export errors in future, consider storing the original `Error` in `ErrorStats` rather than a bare `String`, so the correct variant survives the round-trip.

---

### CR-02: `normalize_and_export` unconditionally calls `compute_normalized` when `passes == false`, diverging from the documented contract and from `collector.rs`

**File:** `src/cli/run/processor.rs:41-50`

**Issue:** When `passes == false`, the function unconditionally calls `compute_normalized` without checking `do_normalize`:

```rust
if !passes {
    crate::pipeline::compute_normalized(   // no do_normalize guard
        record,
        &record.sql,
        params_buffer,
        placeholder_override,
        ns_scratch,
    );
    return ExportAction::Continue;
}
```

The function's own doc comment says: "仅在 `passes==false && do_normalize && record.tag.is_none()` 时更新 `params_buffer`". The `do_normalize` guard is missing.

The caller at `process_log_file:184-185` currently prevents a `do_normalize=false` PARAMS record from reaching this function (`needs_processing = passes || (do_normalize && record.tag.is_none())`), so there is no immediate crash. However:

- The function is not self-consistent: the contract it documents does not match the code it runs.
- `collector.rs::process_record` (the parallel-path equivalent) correctly guards the else-branch — if `process_record` is the reference implementation, `normalize_and_export` has silently diverged.
- Any future caller that passes `do_normalize=false` directly to `normalize_and_export` (bypassing the `needs_processing` guard) will silently pollute `params_buffer`, causing wrong parameter substitutions in subsequent DML records of the same session.

**Fix:**
```rust
if !passes {
    // Filtered PARAMS record: update params_buffer only when normalization
    // is active, so downstream DML records can substitute parameters.
    if do_normalize && record.tag.is_none() {
        crate::pipeline::compute_normalized(
            record,
            &record.sql,
            params_buffer,
            placeholder_override,
            ns_scratch,
        );
    }
    return ExportAction::Continue;
}
```

---

## Warnings

### WR-01: `remove_file` inside `concat_csv_parts` write loop propagates error mid-concat, leaving output file in a corrupt state

**File:** `src/cli/run/parallel.rs:59`

**Issue:** After copying a part file into the output writer, the code removes the part with `std::fs::remove_file(part_path)?`. If `remove_file` fails (permissions, NFS, antivirus lock), the `?` propagates an error immediately. At that point:

- `writer` has not been flushed (flush is at line 63, after the loop).
- Subsequent parts have not been written.
- The output file is partially written and will not be cleaned up (the cleanup at `finalize_concat:236-237` only runs on `concat_result.is_err() && !append_to_existing`, but the file deletion there uses `let _ =` so it is best-effort anyway).

The result is a silently truncated CSV output file that appears valid (has a header) but is missing records.

**Fix:** Collect part paths to remove and delete them in a separate pass after `writer.flush()` succeeds, or downgrade the `?` to a warning log so the concat completes:

```rust
std::io::copy(&mut reader, &mut writer)?;
// Defer removal: only delete after flush succeeds to avoid partial output.
parts_to_remove.push(part_path.clone());
```

Then after `writer.flush()?`:
```rust
for p in parts_to_remove {
    if let Err(e) = std::fs::remove_file(&p) {
        log::warn!("failed to remove temp part {}: {e}", p.display());
    }
}
```

---

### WR-02: `verbose` parameter accepted but silently discarded in `run_csv_parallel` and `run_sqlite_parallel`; the wrappers are no-ops that duplicate a single `eprintln!`

**File:** `src/cli/run/mod.rs:222-295`

**Issue:** Both `run_csv_parallel` (line 233) and `run_sqlite_parallel` (line 272) accept a `verbose: bool` parameter and use it only to guard an `eprintln!` at the top of each function. They then call `process_csv_parallel` / `process_sqlite_parallel` without forwarding `verbose`. The wrappers add no logic beyond logging a "Processing N files in parallel" line — they do not forward `verbose` into the inner functions.

`process_csv_parallel` (in `parallel.rs`) also accepts `show_progress: bool` but immediately discards it with `let _ = show_progress;` (line 268). `process_sqlite_parallel` uses `_show_progress` (underscore-prefixed, line 88). Neither parallel path does anything with progress output.

This means:
- When `verbose=true`, users see the "Processing N files in parallel" line but no per-file detail in parallel mode (unlike sequential mode).
- The `show_progress` parameter passed to both parallel inner functions is silently ignored, so parallel mode never shows progress regardless of `--quiet` / `--verbose` flags.

**Fix:** Either implement per-file verbose output inside the parallel paths, or remove `show_progress` and `verbose` from `process_csv_parallel` / `process_sqlite_parallel` signatures and document that parallel mode does not support progress output. Keeping dead parameters in public-facing function signatures causes confusion for future maintainers.

---

### WR-03: `sqlite_parallel.rs::process_sqlite_parallel` hard-codes `include_pm = true` instead of querying the exporter

**File:** `src/cli/run/sqlite_parallel.rs:123`

**Issue:**
```rust
exporter_manager.export_one_preparsed(&record, true, normalized.as_deref())?;
```

The `include_pm` argument is hard-coded to `true`. In the sequential path (`processor.rs:172`) and CSV parallel path (`parallel.rs:108`), the value is read from `exporter_manager.csv_include_performance_metrics()`. For the SQLite exporter, `csv_include_performance_metrics()` always returns `true` (see `exporter/mod.rs:64-66`), so there is currently no observable difference. However:

- The hard-coded `true` breaks the established pattern and makes the code fragile: if `ExporterKind::csv_include_performance_metrics` is ever changed for SQLite (e.g., to support field projection at the exporter level), this call site will silently pass stale data.
- It also diverges from the way the sequential and CSV-parallel paths work, making the code harder to audit.

**Fix:**
```rust
let include_pm = exporter_manager.csv_include_performance_metrics();
for (file_path, file_rows) in collected {
    let count = file_rows.len();
    for (record, normalized) in file_rows {
        exporter_manager.export_one_preparsed(&record, include_pm, normalized.as_deref())?;
    }
    per_file_counts.push((file_path, count));
}
```

---

## Info

### IN-01: `run_file_loop` calls `run_stats.merge(&file_stats)` before the fatal-error early-return check

**File:** `src/cli/run/mod.rs:373-378`

**Issue:**
```rust
run_stats.merge(&file_stats);          // line 373
if file_stats.has_fatal() {            // line 374
    return Err(...);                   // line 375-378
}
```

When a fatal error occurs, `file_stats` (including the fatal flag) is merged into `run_stats` before the function returns an `Err`. Since the function returns `Err`, the caller never sees `run_stats`, so the merged data is effectively discarded. The merge here is dead code on the fatal path.

This does not cause a bug today but is misleading: a reader of the code may assume `run_stats` is returned somehow. Consider merging only on non-fatal paths, or add a comment clarifying the discard:

```rust
if file_stats.has_fatal() {
    // run_stats is discarded because we return Err; skip the merge.
    return Err(...);
}
run_stats.merge(&file_stats);
```

---

_Reviewed: 2026-06-03T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
