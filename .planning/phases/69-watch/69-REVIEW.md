---
phase: 69-watch
reviewed: 2026-06-06T00:00:00Z
depth: standard
files_reviewed: 7
files_reviewed_list:
  - src/cli/watch.rs
  - src/error.rs
  - src/cli/run/mod.rs
  - src/cli/opts.rs
  - src/cli/mod.rs
  - src/main.rs
  - tests/integration.rs
findings:
  critical: 0
  warning: 3
  info: 2
  total: 5
status: issues_found
---

# Phase 69: Code Review Report

**Reviewed:** 2026-06-06T00:00:00Z
**Depth:** standard
**Files Reviewed:** 7
**Status:** issues_found

## Summary

Phase 69 adds the `watch` subcommand: a `notify`-based directory watcher that triggers `handle_run` on new or modified `.log` files. The core architecture is sound. Debounce is correctly implemented via `should_trigger` with a 500ms `HashMap<PathBuf, Instant>` per-path window, preventing duplicate processing when macOS FSEvents fires both `Create` and `Modify` for the same file. The `interrupted` `AtomicBool` is properly shared and checked. `ErrorStats::merge` accumulates `records_exported` correctly across watch triggers. No security vulnerabilities or data-loss bugs found.

Three issues degrade operator experience or code quality: a false-positive truncation footer in the error log writer, a misleading log warn on normal user interruption, and a multi-directory display regression in the active status line. Two informational items are noted around dead code and test coverage gaps.

## Warnings

### WR-01: Error-log truncation footer fires for exactly 10,000 parse errors — false positive

**File:** `src/cli/run/mod.rs:441`

**Issue:** The truncation check is:
```rust
let truncated = stats.parse_error_records.len() == 10_000;
```
This assumes the vector length is 10,000 **only** when truncation occurred. But if a file produces **exactly** 10,000 parse errors (total matches the cap), the vector fills to exactly 10,000 entries with no truncation, and this condition is still `true`. The error log will incorrectly append:
```
[truncated; showing first 10000 of 10000 total parse errors]
```
This misleads operators into thinking records were silently dropped when they were not.

**Fix:**
```rust
// src/cli/run/mod.rs:441 — compare counts instead of checking exact equality
let truncated = stats.parse_errors > stats.parse_error_records.len();
```

---

### WR-02: `Error::Interrupted` from `handle_run` logged as "watch trigger error" — misleading

**File:** `src/cli/watch.rs:304`

**Issue:** When the user presses Ctrl+C during an active `handle_run` invocation, `handle_run` returns `Err(Error::Interrupted)`. The catch-all arm in `process_log_path` logs it as:
```
WARN watch trigger error: Interrupted by user
```
This looks like a runtime failure in the application logs. The watch session terminates correctly (the AtomicBool is checked at the top of the event loop after `handle_event` returns), but operators reviewing log files will see a misleading "error" for normal graceful shutdown.

**Fix:**
```rust
// src/cli/watch.rs:303-305
Err(crate::error::Error::Interrupted) => {
    // Normal shutdown; outer loop will break on interrupted.load()
}
Err(e) => warn!("watch trigger error: {e}"),
```

---

### WR-03: Active watch status line shows only the first watched directory after the first trigger

**File:** `src/cli/watch.rs:349-357`

**Issue:** `refresh_active_status` builds the display string with only `watch_dirs.first()`:
```rust
let dir_str = watch_dirs
    .first()
    .map(|p| p.display().to_string())
    .unwrap_or_default();
```
The initial spinner (set in `build_progress_bar`) correctly uses `format_paths_display(watch_dirs)` which shows all directories (or "N directories" for > 3). After the first successful trigger the status bar silently degrades to showing only one path, even when multiple directories are being watched. Users who configure multiple `inputs` entries will be confused.

**Fix:**
```rust
// src/cli/watch.rs:351-352 — reuse format_paths_display instead of first()
let dir_str = format_paths_display(watch_dirs);
pb.set_message(render_active_status(
    &dir_str,
    trigger_count,
    rows,
    triggered_at.elapsed(),
));
```
`format_paths_display` is already defined in the same module at line 220.

---

## Info

### IN-01: Dead `let _ = verbose;` statement at end of `handle_watch`

**File:** `src/cli/watch.rs:61`

**Issue:** `verbose` is consumed by `run_watch_loop` at line 46 (passed through the call chain down to `handle_run`). The `let _ = verbose;` on line 61 is therefore dead code — `verbose` has already been used before this point. This pattern typically appears to suppress an "unused variable" compiler warning, but since the variable is actually used, the statement is redundant and confuses readers into thinking `verbose` might not be threaded through.

**Fix:** Remove line 61. If the compiler emits a warning after removal, that would signal a real regression where `verbose` is no longer being passed to the run path.

---

### IN-02: `test_watch_triggers_on_new_log_file` permanently `#[ignore]`'d — no tracking mechanism

**File:** `tests/integration.rs:2911`

**Issue:**
```rust
#[ignore = "macOS FSEvents + test stdin-pipe block; fix in Phase 70 smoke test"]
```
This is the most important watch behavioral test (WATCH-02/05: verifying that a new `.log` file triggers processing and produces output). It is completely skipped in `cargo test`. The "Phase 70" reference has no linked issue number, meaning it can remain permanently ignored. The existing active watch tests (W2: pre-interrupted exit, W4: non-.log file ignored) only cover degenerate paths and do not prove the happy path works end-to-end.

**Fix:** File a tracking issue with an issue number and update the ignore message. Consider an alternative test design: construct the config with a pre-existing CSV exporter output path that has `append: true`, and use `handle_watch` directly (not via subprocess) to avoid the stdin-pipe interaction. If the root cause is cargo test injecting a non-tty stdin that triggers the stdin-pipe fallback in `handle_run`, a minimal fix is to set `log_files` from a non-empty list (guaranteed by a pre-written `.log` file) so `resolve_input_files` never reaches the stdin path.

---

_Reviewed: 2026-06-06T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
