---
phase: 67-prog-diag
reviewed: 2026-06-06T00:00:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - src/cli/run/mod.rs
  - src/cli/run/processor.rs
  - src/cli/run/tests.rs
  - src/config/mod.rs
  - src/error.rs
findings:
  critical: 0
  warning: 4
  info: 3
  total: 7
status: issues_found
---

# Phase 67: Code Review Report

**Reviewed:** 2026-06-06T00:00:00Z
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

Reviewed the phase 67 (prog-diag) implementation covering progress tracking, diagnostics, error statistics, and the run orchestration pipeline. The core correctness logic is sound: the two-pass prescan design, normalize-and-export hot path, interrupt handling, and `ErrorStats` merge semantics all appear correct.

Four warnings were identified. The most significant is a functional regression in the parallel execution paths: both the CSV-parallel and SQLite-parallel paths fail to populate `parse_error_records` and `by_type`, causing the error log file to never be written and hint messages to never appear for parallel runs, even when parse errors occur. This silently degrades a diagnostic feature for the most common production configuration (multi-file with multiple CPUs).

Three info-level findings cover a `progress bar` display ordering issue, an infallible `expect()` in parallel code, and a test that doesn't actually verify parallel-path execution.

---

## Warnings

### WR-01: Error log file never written on parallel runs — parse_error_records not populated

**File:** `src/cli/run/collector.rs:38–59` (used by `src/cli/run/parallel.rs:163` and `src/cli/run/sqlite_parallel.rs:34`)

**Issue:** `collector::collect_log_file` only increments a `parse_errors: usize` counter for each parse failure; it never creates `ParseErrorRecord` objects and never calls `classify_error_kind`. As a result, when the parallel CSV or SQLite paths are active (`jobs > 1 && log_files.len() > 1`), `run_stats.parse_error_records` remains empty after the full run. `write_error_log` in `mod.rs:477` guards on `stats.parse_error_records.is_empty()` and returns immediately — the error log file is never created. Simultaneously, `by_type` in `ErrorStats` is also never populated, so the two hint messages in `print_run_summary` (encoding hint and field-missing hint) are never shown for parallel runs regardless of how many parse errors occurred.

The sequential path in `processor.rs:241–247` does populate `parse_error_records` and `classify_error_kind` correctly. This is a behavioral difference between the two execution paths with no user-visible indication.

**Fix:** In `collector.rs`, change the parse error branch to capture `ParseErrorRecord` with kind classification the same way `processor.rs` does:

```rust
Err(e) => {
    parse_errors += 1;
    let (line_number, raw_ref) = match &e {
        ParseError::InvalidFormat { raw, line_number } => (*line_number, raw.as_str()),
        _ => (0u64, ""),
    };
    let kind = crate::error::classify_error_kind(raw_ref);
    // Accumulate into a local Vec<ParseErrorRecord> (capped at 10_000),
    // return alongside parse_errors count for merging into ErrorStats.
    log::warn!("{} | parse error: {e:?}", file.display());
    continue;
}
```

The function signature should return `(Vec<(Sqllog, Option<String>)>, usize, Vec<ParseErrorRecord>)` or accept a mutable `ErrorStats` to accumulate directly, mirroring the sequential path.

---

### WR-02: `finalize` error silently dropped when the file-processing loop also fails

**File:** `src/cli/run/mod.rs:327–332`

**Issue:** `run_sequential` calls `exporter_manager.finalize()` unconditionally (correct — ensures the `BufWriter` flush attempt is made even on loop failure). However, the result is bound and checked *after* `loop_result?`:

```rust
let finalize_result = exporter_manager.finalize();
(!quiet).then(|| exporter_manager.log_stats());
let (per_file_counts, run_stats) = loop_result?;   // early-returns on loop error
finalize_result?;                                   // never reached if loop failed
```

If `loop_result` is `Err`, the `finalize_result?` line is unreachable. A `BufWriter` flush failure (e.g., disk full after processing 10 GB of records) is silently lost — the caller receives only the loop error and has no indication the output file is truncated or corrupt.

**Fix:** When `loop_result` is `Err`, log `finalize_result` as a warning before returning:

```rust
let (per_file_counts, run_stats) = match loop_result {
    Ok(v) => v,
    Err(loop_err) => {
        if let Err(fin_err) = finalize_result {
            log::warn!("finalize failed during loop error cleanup: {fin_err}");
        }
        return Err(loop_err);
    }
};
finalize_result?;
Ok((per_file_counts, run_stats))
```

---

### WR-03: `eprintln!` in `merge_trxid_prescan` ignores `--quiet` flag

**File:** `src/cli/run/mod.rs:188–191`

**Issue:** When stdin-pipe mode is combined with transaction-level filters, the degradation warning is emitted via a bare `eprintln!`:

```rust
eprintln!(
    "[WARN] Transaction-level filters with stdin: pre-scan disabled, \
     degrading to per-record matching."
);
```

This call is not guarded by `!quiet`, so it writes to stderr even when the user runs with `--quiet`. The `quiet` flag is not in scope inside `merge_trxid_prescan`, but it is available at the call site in `handle_run`. Breaking the quiet contract makes piping workflows that parse stderr unreliable.

**Fix:** Pass `quiet` into `merge_trxid_prescan` (or return a diagnostic string and let `handle_run` decide whether to print it):

```rust
fn merge_trxid_prescan(
    cfg: &Config,
    log_files: &[PathBuf],
    jobs: usize,
    is_stdin_pipe: bool,
    quiet: bool,        // added
) -> Result<Option<Config>> {
    ...
    if is_stdin_pipe {
        warn!("...");
        if !quiet {
            eprintln!("[WARN] Transaction-level filters with stdin: ...");
        }
        return Ok(None);
    }
```

---

### WR-04: Progress bar printed over by summary — `finish_and_clear` called after `print_run_summary`

**File:** `src/cli/run/mod.rs:118–130`

**Issue:** `print_run_summary` is called at line 118 while the progress bar's steady-tick thread is still writing to stderr. `pb.finish_and_clear()` is called only at line 130, *after* the summary output. Because both `eprintln!` and `indicatif`'s tick thread write to the same stderr file descriptor, the multi-line summary (records, errors, hints) can be interleaved with or obscured by the spinner. The canonical fix in `indicatif` is to call `finish_and_clear()` first, or to use `pb.println()` for summary lines while the bar is active.

**Fix:** Reorder to clear the progress bar before printing the summary:

```rust
// In handle_run, after collecting total_records:
if let Some(pb) = &pb {
    pb.finish_and_clear();   // moved up
}
print_run_summary(...);
write_error_log(final_cfg, &run_stats);
// pb already cleared above — remove the second finish_and_clear
```

---

## Info

### IN-01: `expect()` in parallel CSV path — logically infallible but not idiomatic

**File:** `src/cli/run/parallel.rs:286`

**Issue:** `process_csv_parallel` uses `.expect("parallel CSV requires CSV exporter")` to unwrap the CSV config. The call site in `mod.rs` guarantees `exporter.csv.is_some()` before calling this function, so it cannot panic in practice. However, `expect()` in non-test production code is a code smell per Rust convention and the project's error-handling style (all other callers use `?` propagation). If the invariant is ever violated through a future refactor, the process panics rather than returning a typed error.

**Fix:** Convert to a typed error:

```rust
let csv_cfg = cfg.exporter.csv.as_ref().ok_or_else(|| {
    Error::Config(crate::error::ConfigError::NoExporters)
})?;
```

---

### IN-02: Progress bar created but never incremented for parallel paths

**File:** `src/cli/run/mod.rs:62–68`

**Issue:** `make_progress_bar(show_progress, log_files.len())` is called unconditionally before the parallel/sequential branch. When `use_parallel` is true, the progress bar is passed only to `run_sequential`. The parallel functions (`run_csv_parallel`, `run_sqlite_parallel`) do not receive the progress bar, so `{pos}/{len}` remains `0/N` for the entire run. Users see a spinning cursor but no count advancement. The bar is also created with `enable_steady_tick(80ms)`, keeping a background thread running unnecessarily throughout the parallel run.

**Fix:** Either pass the progress bar into the parallel helpers and call `pb.inc(1)` per completed file (must be done with `Arc<ProgressBar>` in rayon tasks), or suppress the progress bar entirely for parallel runs:

```rust
let show_progress = !quiet && !verbose && !use_parallel;
let pb = make_progress_bar(show_progress, log_files.len());
```

Note: `use_parallel` is not yet computed at that point in `handle_run`; the check would need to be restructured or deferred.

---

### IN-03: `test_parallel_merge_consistent` does not guarantee parallel path execution

**File:** `src/cli/run/tests.rs:103–170`

**Issue:** The test calls `handle_run` with `jobs_override: None`, so `jobs = available_parallelism()`. On a single-core machine (or CI runner with 1 vCPU), `jobs = 1`, `use_csv_parallel = false`, and both the "sequential" and "parallel" configurations run through the sequential path. The assertion `par_lines == seq_lines + 1` would still pass because it only validates record counts, not which execution path was taken. The test name implies parallel behavior but cannot enforce it.

**Fix:** Use `jobs_override: Some(2)` to force the parallel path regardless of host CPU count:

```rust
handle_run(&cfg_par, true, false, &Arc::new(AtomicBool::new(false)), Some(2)).unwrap();
```

This ensures the test always exercises `process_csv_parallel` on all machines.

---

_Reviewed: 2026-06-06T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
