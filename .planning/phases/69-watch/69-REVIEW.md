---
phase: 69-watch
reviewed: 2026-06-06T00:00:00Z
depth: standard
files_reviewed: 8
files_reviewed_list:
  - Cargo.toml
  - src/cli/run/mod.rs
  - src/error.rs
  - src/cli/mod.rs
  - src/cli/opts.rs
  - src/cli/watch.rs
  - src/main.rs
  - tests/integration.rs
findings:
  critical: 1
  warning: 3
  info: 2
  total: 6
status: issues_found
---

# Phase 69: Code Review Report

**Reviewed:** 2026-06-06
**Depth:** standard
**Files Reviewed:** 8
**Status:** issues_found

## Summary

This phase added the `watch` subcommand, wiring a `notify`-based directory watcher through the existing `handle_run` pipeline. The overall structure is sound — the event loop is clean, canonicalization for macOS is correct, and the `ErrorStats.records_exported` field is properly accumulated in `ErrorStats::merge`. However one correctness bug was found in the core event handling path (duplicate processing on macOS), two warning-level design issues exist in error handling and testing, and two minor code quality items round out the findings.

## Critical Issues

### CR-01: macOS FSEvents double-processing — same file processed twice, duplicating CSV/SQLite rows

**File:** `src/cli/watch.rs:164-168`

**Issue:** `handle_event` accepts both `EventKind::Create(_)` and `EventKind::Modify(ModifyKind::Data(DataChange::Content))` as relevant events. On macOS with FSEvents, when a new `.log` file is written, the OS commonly fires **both** events in rapid succession for the same file — first `Create(File)` (file may be empty at this point) and shortly after `Modify(Data(Content))` (data is flushed). Because neither event handler deduplicates on path, both will independently call `handle_run` with the same file, exporting its records twice. This produces duplicate rows in the CSV/SQLite output and double-counts `total_stats.records_exported`.

The comment on line 162 acknowledges this sequence ("macOS FSEvents 有时先发 Create(File)… 再发 Modify(Data(Content))") but does not guard against double-processing — it reads as intent to handle both as a fallback, yet there is no mechanism to prevent both from firing on a single file creation.

**Fix:** Track recently-processed paths in a `HashMap<PathBuf, Instant>` or `HashSet<PathBuf>` with a short debounce window (e.g., 500 ms). Skip any path that was processed within the window. Alternatively, emit only on `Modify(Data(Content))` and drop the `Create` branch (if testing shows macOS always fires Modify after Create), or gate the Create path on file size > 0.

```rust
// In handle_event, add a recent-seen deduplication set:
fn handle_event(
    event: &notify::Event,
    ...
    seen: &mut HashMap<PathBuf, Instant>,   // new parameter
) {
    // ...
    for path in &event.paths {
        if path.extension().is_none_or(|ext| ext != "log") {
            continue;
        }
        let now = Instant::now();
        if let Some(&last) = seen.get(path) {
            if now.duration_since(last) < Duration::from_millis(500) {
                continue; // debounce: skip duplicate event within window
            }
        }
        seen.insert(path.clone(), now);
        // ... rest of processing
    }
}
```

---

## Warnings

### WR-01: `Err(Error::Interrupted)` from `handle_run` silently swallowed as a warning

**File:** `src/cli/watch.rs:191-193`

**Issue:** When Ctrl+C is pressed while a `handle_run` is in progress, `handle_run` returns `Err(Error::Interrupted)`. The `Err(e)` arm in `handle_event` treats this identically to any parse/export error: it logs `warn!("watch trigger error: Interrupted by user")`. This emits a misleading warning message into the log — "watch trigger error" — making normal graceful shutdown look like an error condition to operators reading log files. The interrupt path ultimately works correctly because the outer loop checks `interrupted.load()` after `handle_event` returns, but the spurious warning will confuse users.

**Fix:** Distinguish `Error::Interrupted` in the error arm and log nothing (or log at debug level):

```rust
Err(crate::error::Error::Interrupted) => {
    // Normal shutdown; outer loop will break on interrupted.load()
}
Err(e) => {
    warn!("watch trigger error: {e}");
}
```

### WR-02: `let _ = verbose` is dead code / misleading suppressor

**File:** `src/cli/watch.rs:81`

**Issue:** `verbose` is already used at line 59 (passed to `handle_event`, which passes it to `handle_run`). The `let _ = verbose` at line 81 is therefore redundant — it suppresses a compiler warning that should no longer exist. Dead suppressors signal stale cleanup and will cause clippy to emit `unused_variables` or related lint noise, potentially masking a real regression where `verbose` is removed from a call site and the suppressor masks that omission.

**Fix:** Remove line 81 entirely:

```rust
// Delete: let _ = verbose;
```

### WR-03: W4 integration test relies on timing without a hard bound — flaky on slow CI

**File:** `tests/integration.rs:2942-2966`

**Issue:** `test_watch_ignores_non_log_files` spawns a thread that writes a `.txt` file after 300 ms and then sets `interrupted` after 700 ms. The assertion is that the CSV file must not exist. This test depends on two timing assumptions: (a) the watcher starts and stabilizes within 300 ms, and (b) the event loop processes the file write, determines it is not a `.log` file, and completes before 700 ms. On slow CI agents (Windows runners, resource-constrained containers) the 700 ms window can be insufficient. A false negative is possible but not a false positive — if the test fails intermittently, it passes with `#[ignore]`. However the lack of a hard timeout means the test can also hang if the `handle_watch` call blocks for any reason.

**Fix:** Add `#[ignore = "timing-sensitive; use --include-ignored for smoke run"]` to document the known limitation, or restructure by injecting a ready-channel from the watcher so the test only waits as long as needed. If keeping timing-based: lower the first sleep and increase the interrupt sleep to give more headroom, and document the CI assumption explicitly.

---

## Info

### IN-01: `trigger_count` increments even when `handle_run` processes an empty file (zero records)

**File:** `src/cli/watch.rs:181`

**Issue:** `trigger_count` is incremented unconditionally on any `Ok(file_stats)` result, including the case where the file is valid but contains zero exportable records. On macOS, `Create(File)` is fired when the file descriptor is created, before any data is written. At that moment the file may be empty; `handle_run` succeeds with zero records, and `trigger_count` is bumped. This makes the status line show inflated trigger counts ("triggers: 2" when only one meaningful file was written). This is also evidence of the CR-01 issue — if both events fire, the count double-increments.

**Fix:** Conditionally increment only when `file_stats.records_exported > 0`:

```rust
if file_stats.records_exported > 0 {
    *trigger_count += 1;
}
total_stats.merge(&file_stats);
```

### IN-02: W3 (`test_watch_triggers_on_new_log_file`) permanently `#[ignore]`-ed without a tracking issue

**File:** `tests/integration.rs:2911`

**Issue:** The test is marked `#[ignore = "macOS FSEvents + test stdin-pipe block; fix in Phase 70 smoke test"]`. This is the most important watch behavior test (WATCH-02/05 — verifying that a new `.log` file actually triggers processing), and it is completely skipped in `cargo test`. The comment references "Phase 70" as the fix location, but without a linked issue or tracking mechanism, this test can remain permanently ignored. The consequence is that the core watch-to-process flow has no automated coverage in the standard test suite.

**Fix:** At minimum, add a TODO comment with an issue number. Preferably, redesign the test to avoid the stdin-pipe interaction problem by constructing the config with `overwrite: true` and using a dedicated temp dir that is cleanly separate from the test harness's stdin context. If the root cause is specifically cargo test's stdin pipe mode triggering `NoFilesFound` instead of the watch loop, the fix is to ensure the CSV exporter config is set with `append: true` or the file is pre-created to avoid the preflight stdin check.

---

_Reviewed: 2026-06-06_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
