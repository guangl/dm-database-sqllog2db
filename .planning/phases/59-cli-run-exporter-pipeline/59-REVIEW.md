---
phase: 59-cli-run-exporter-pipeline
reviewed: 2026-06-03T14:00:00Z
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
  info: 2
  total: 7
status: issues_found
---

# Phase 59: Code Review Report

**Reviewed:** 2026-06-03T14:00:00Z
**Depth:** standard
**Files Reviewed:** 6
**Status:** issues_found

## Summary

Reviewed six files covering sequential and parallel CLI run orchestration, filter processing, record collection (`collector.rs`), per-record processing (`processor.rs`), CSV parallel concat (`parallel.rs`), and SQLite parallel write (`sqlite_parallel.rs`).

The overall decomposition is sound. The two-phase PARAMS-buffer logic is correctly mirrored between `collector.rs` (parallel path) and `processor.rs` (sequential path), and the pipeline/prescan design is coherent.

Two blockers were found: (1) `concat_csv_parts` deletes each temp file before flushing the BufWriter, creating a data-loss scenario on any mid-concat I/O failure and a hard crash on Windows due to open-file deletion semantics; (2) `run_sequential` does not call `exporter_manager.finalize()` on the error path, silently discarding buffered CSV/SQLite data when a fatal export error occurs.

Three warnings cover: a defensive-guard mismatch in `normalize_and_export` vs its doc comment, a fatal error re-wrapping with the wrong variant and wrong path in `run_file_loop`, and unresponsive `Ctrl+C` handling during heavily-filtered single-file runs in the sequential path.

Two info items cover dead progress parameters in parallel function signatures and a merge-before-return dead-code ordering in `run_file_loop`.

---

## Critical Issues

### CR-01: `concat_csv_parts` deletes temp files before flush — data loss on I/O failure; hard crash on Windows

**File:** `src/cli/run/parallel.rs:58-59`

**Issue:** Inside the concat loop, each temp part file is deleted immediately after `std::io::copy` but before `writer.flush()`:

```rust
std::io::copy(&mut reader, &mut writer)?;   // data goes into BufWriter's 2 MB in-memory buffer
std::fs::remove_file(part_path)?;           // source deleted while data is still in RAM
```

The `writer.flush()` on line 63 is outside the loop. This creates two failure modes:

**A — Data loss on any subsequent I/O failure.** If a later part's `copy` fails (disk full, I/O error), the `?` propagates, `flush()` is never called, and the BufWriter is dropped — its Drop silently discards the unflushed buffer per Rust's documented BufWriter behavior. The earlier parts that were already removed are now gone: their processed data exists neither on disk in temp storage nor in the output file. `finalize_concat` then deletes the partial output file, so the user must re-run the entire job from scratch with no indication of which files were lost.

**B — Hard failure on Windows.** On Windows, `remove_file` on an open file handle returns `os error 32` (`ERROR_SHARING_VIOLATION`). The `reader` (`BufReader<File>`) is still live in scope when `remove_file` is called at line 59. Since `mod.rs` already contains `#[cfg(target_os = "windows")]` guards showing Windows is a supported target, this means the parallel CSV export path always fails on Windows during the cleanup step after all records have been successfully parsed.

**Fix:** Collect paths for deferred removal and flush before any deletion:

```rust
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
    // reader dropped here — file handle closed before removal
    parts_to_remove.push(part_path);
}
writer.flush()?;   // flush BEFORE any deletion
for p in parts_to_remove {
    if let Err(e) = std::fs::remove_file(p) {
        log::warn!("failed to remove temp part {}: {e}", p.display());
    }
}
```

This ensures: (a) all data is safely on disk before any source is deleted, (b) the file handle is closed before calling `remove_file` on Windows, and (c) cleanup errors are logged but not fatal (the important data is already written).

---

### CR-02: `run_sequential` does not finalize the exporter on fatal error — BufWriter data silently discarded

**File:** `src/cli/run/mod.rs:316-329`

**Issue:** `run_sequential` wraps `run_file_loop` with a `?` operator and only calls `exporter_manager.finalize()` on the success path:

```rust
let (per_file_counts, run_stats) = run_file_loop(...)?;  // line 316: any Err skips line 327
exporter_manager.finalize()?;                            // line 327: never called on error
```

`run_file_loop` returns `Err` in two cases:
1. `process_log_file` returns `Err` (e.g., `crate::scanner::build_parser` fails to open a file).
2. `file_stats.has_fatal()` is true — an export error was flagged fatal (e.g., SQLite `DatabaseFailed`).

In both cases, `ExporterManager::finalize()` is skipped. For the CSV exporter, `finalize()` calls `BufWriter::flush()`. Without it, the `BufWriter<File>` is dropped when `run_sequential` unwinds. Rust's `BufWriter::drop` does call `flush()` internally, but only a best-effort attempt — it silently ignores errors. If the flush fails (e.g., disk full, which may be the root cause of the fatal error), records buffered in the 16 MB BufWriter are permanently lost without any error surfaced to the user.

The result: the output CSV file appears to exist and have content, but is silently truncated by up to 16 MB of records that were accepted by the exporter (returning `Ok(())`) but never written to disk.

**Fix:** Ensure `finalize()` is always called, even on the error path. Use a guard pattern or drop finalizer:

```rust
fn run_sequential(...) -> Result<(Vec<(PathBuf, usize)>, ErrorStats)> {
    let mut exporter_manager = ExporterManager::from_config(final_cfg)?;
    exporter_manager.initialize()?;
    info!("Parsing and exporting SQL logs...");
    let loop_result = run_file_loop(
        log_files, &mut exporter_manager, pipeline,
        do_normalize, placeholder_override, verbose, show_progress, pb, interrupted,
    );
    // Finalize regardless of loop outcome; preserve the loop error if finalize also fails
    let finalize_result = exporter_manager.finalize();
    if !quiet {
        exporter_manager.log_stats();
    }
    let (per_file_counts, run_stats) = loop_result?;
    finalize_result?;
    Ok((per_file_counts, run_stats))
}
```

---

## Warnings

### WR-01: `normalize_and_export` calls `update_params_buffer_only` unconditionally in the `!passes` branch, ignoring `do_normalize`

**File:** `src/cli/run/processor.rs:59-62`

**Issue:** The function's own doc comment states: "仅在 `passes==false && do_normalize && record.tag.is_none()` 时更新 `params_buffer`". The implementation does not enforce the `do_normalize` guard:

```rust
if !passes {
    update_params_buffer_only(record, params_buffer, placeholder_override, ns_scratch);
    // ^ called for ALL !passes records; do_normalize and tag.is_none() not checked
    return ExportAction::Continue;
}
```

There is currently no runtime bug because the caller (`process_log_file` line 195) only calls `normalize_and_export` on records that pass `needs_processing = passes || (do_normalize && record.tag.is_none())`. So in the `!passes` branch, `do_normalize=true` and `record.tag.is_none()` are guaranteed by the call site. However:

- The function's contract does not match its implementation. Any future call site that passes `do_normalize=false` will silently mutate `params_buffer` with PARAMS entries, causing incorrect SQL parameter substitution in later DML records for the same session.
- The parallel equivalent in `collector.rs` correctly applies the guard (lines 92-100), creating divergent contracts between the two code paths that mirror each other.

**Fix:** Make the function self-defensive by adding the missing guard:

```rust
if !passes {
    if do_normalize && record.tag.is_none() {
        update_params_buffer_only(record, params_buffer, placeholder_override, ns_scratch);
    }
    return ExportAction::Continue;
}
```

---

### WR-02: Fatal export error in `run_file_loop` re-wrapped as wrong variant with wrong path

**File:** `src/cli/run/mod.rs:374-379`

**Issue:** When `file_stats.has_fatal()` is true (e.g., SQLite `DatabaseFailed`), `run_file_loop` reconstructs the error as:

```rust
return Err(Error::Export(crate::error::ExportError::WriteFailed {
    path: log_file.into(),                           // input .log file — wrong path
    reason: file_stats.fatal_error.unwrap_or_default(),
}));
```

Two concrete defects:

1. **Wrong variant.** `ExportError::WriteFailed` has `is_fatal() == false` per `error.rs:101` (only `ExportError::DatabaseFailed` is fatal). The reconstructed error's classification is silently downgraded from Critical to Error severity. `suggestion()` returns generic disk-space advice instead of the SQLite-specific message.

2. **Wrong path.** `path` is set to the input `.log` file being processed at the time of the error, not the SQLite database path or CSV output path. Error messages and any upstream handlers that extract the path will misattribute the failure.

**Fix:**

```rust
if file_stats.has_fatal() {
    // DatabaseFailed preserves the fatal classification; reason already contains
    // the full error message from the original error via set_fatal(e.to_string())
    return Err(Error::Export(crate::error::ExportError::DatabaseFailed {
        reason: file_stats.fatal_error.unwrap_or_default(),
    }));
}
```

For a more general fix, store the original `Error` in `ErrorStats` (rather than a lossy `String`) so the variant and embedded paths survive the round-trip.

---

### WR-03: Interrupt check in `process_log_file` is gated on `passes`, leaving Ctrl+C unresponsive during heavily-filtered runs

**File:** `src/cli/run/processor.rs:204`

**Issue:**

```rust
ExportAction::Continue if passes && tick_progress(pb, records_in_file, interrupted) => break 'outer,
```

`tick_progress` — and therefore the `interrupted.load()` check — executes only when `passes == true`. For filtered records (`passes == false`), no interrupt check occurs inside the inner loop. The only other interrupt check in the sequential path is at the start of each file in `run_file_loop` (line 352).

For a single large file where most or all records fail the filter (e.g., filtering by a username that rarely appears in a 1 GB log), a user pressing Ctrl+C will not be acknowledged until the entire file finishes parsing. This can take minutes.

The parallel path (`collector.rs:39`) correctly checks the interrupt flag for every record regardless of filter outcome, making the two paths inconsistent in their responsiveness.

**Fix:** Add an unconditional interrupt check on the filtered path at the same cadence (every 1024 total records parsed, not just exported):

```rust
match action {
    ExportAction::BreakQuota | ExportAction::BreakFatal => break 'outer,
    ExportAction::Continue if passes && tick_progress(pb, records_in_file, interrupted) => break 'outer,
    ExportAction::Continue => {}
}
// Honor Ctrl+C for filtered records too, on the same 1024-record cadence
if !passes {
    let total_parsed = records_in_file + errors_in_file; // approximate
    if total_parsed.trailing_zeros() >= 10 && interrupted.load(Ordering::Relaxed) {
        break 'outer;
    }
}
```

Alternatively, move the interrupt poll out of `tick_progress` into a shared counter that increments for every processed record (whether exported or filtered), and check it once per 1024 iterations.

---

## Info

### IN-01: `show_progress` / `_show_progress` is a dead parameter in both parallel inner functions

**File:** `src/cli/run/parallel.rs:268`, `src/cli/run/sqlite_parallel.rs:109`

**Issue:** `process_csv_parallel` receives `show_progress: bool` and immediately discards it with `let _ = show_progress;` on line 268. `process_sqlite_parallel` renames it `_show_progress: bool` (underscore prefix). Neither function implements any progress output. The outer wrappers (`run_csv_parallel`, `run_sqlite_parallel`) in `mod.rs` also consume `verbose` without forwarding it to the inner functions.

Dead parameters in public function signatures mislead maintainers into assuming progress is implemented in parallel mode, and they pollute callsites with arguments that have no effect.

**Fix:** Either implement progress output in parallel mode (e.g., using `indicatif`'s multi-progress support) or remove the dead parameters from both function signatures and add a comment documenting that parallel mode does not currently support per-file progress. The outer wrapper's "N files, M jobs" `eprintln!` log line is sufficient for the verbose path.

---

### IN-02: `run_stats.merge` runs before the fatal-error check in `run_file_loop` — dead code on the error path

**File:** `src/cli/run/mod.rs:373-379`

**Issue:**

```rust
per_file_counts.push((log_file.clone(), processed));
run_stats.merge(&file_stats);      // line 373 — always executed
if file_stats.has_fatal() {        // line 374
    return Err(...);               // run_stats dropped here; the merge was wasted
}
```

When a fatal error occurs, `run_stats` is never returned — the function exits via `Err`. The `merge` on the fatal path does not affect any observable behavior and is silently discarded. This misleads a code reader into thinking the merged statistics will be surfaced to the caller.

**Fix:** Swap the order: check for fatal before merging:

```rust
if file_stats.has_fatal() {
    return Err(Error::Export(crate::error::ExportError::DatabaseFailed {
        reason: file_stats.fatal_error.unwrap_or_default(),
    }));
}
per_file_counts.push((log_file.clone(), processed));
run_stats.merge(&file_stats);
```

---

_Reviewed: 2026-06-03T14:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
