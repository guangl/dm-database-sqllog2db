---
phase: 69-watch
reviewed: 2026-06-06T00:00:00Z
depth: standard
files_reviewed: 7
files_reviewed_list:
  - src/cli/mod.rs
  - src/cli/opts.rs
  - src/cli/run/mod.rs
  - src/cli/watch.rs
  - src/error.rs
  - src/main.rs
  - tests/integration.rs
findings:
  critical: 2
  warning: 4
  info: 3
  total: 9
status: issues_found
---

# Phase 69: Code Review Report

**Reviewed:** 2026-06-06T00:00:00Z
**Depth:** standard
**Files Reviewed:** 7
**Status:** issues_found

## Summary

Phase 69 adds the `watch` subcommand: a `notify`-based directory watcher that triggers `handle_run` on new or modified `.log` files. The core architecture is sound — debounce is correctly implemented via `should_trigger` with a 500ms `HashMap<PathBuf, Instant>` per-path window, and the `interrupted` `AtomicBool` is properly shared and checked. `ErrorStats::merge` accumulates `records_exported` correctly across triggers.

Two critical bugs were found. First, `handle_run` called from within watch mode can fall back to reading `/dev/stdin` when the triggered file no longer exists (race with log rotation) and the test process stdin is a pipe; this is the confirmed root cause of the `#[ignore]`'d integration test. Second, `Error::Interrupted` returned from `handle_run` mid-trigger is silently swallowed into a `warn!` rather than propagating exit, leaving a misleading log entry and preventing the loop from exiting cleanly in a multi-event burst.

Four warnings cover: a false-positive truncation footer in the error log writer, the `Interrupted` warn-log cosmetic issue, a multi-directory display regression in the active status line, and unhandled rename/create events from log rotation strategies. Three info items note dead code, the permanently-ignored test, and a quadratic deduplication pattern.

---

## Critical Issues

### CR-01: Triggered file deleted between event and processing causes silent stdin fallback and hang

**File:** `src/cli/watch.rs:286-288` / `src/cli/run/mod.rs:162-167`

**Issue:** `process_log_path` constructs `tmp_cfg.sqllog.inputs = vec![path.to_string_lossy()]` then calls `handle_run`. Inside `handle_run`, `resolve_input_files` calls `SqllogParser::expand_single`, which checks `path.exists()`. If the file was deleted between the `notify` event and the `expand_single` call (e.g. log rotation renamed it away), `expand_single` returns `Err(ParserError::PathNotFound)`. `resolve_input_files` returns an empty file list. On Unix, when `stdin` is not a terminal — which is always the case inside `cargo test` and is common in daemonised deployments — the code then silently falls back to:

```rust
let is_stdin_pipe = log_files.is_empty() && !std::io::stdin().is_terminal();
vec![PathBuf::from("/dev/stdin")]  // blocks indefinitely
```

This is the confirmed root cause of `test_watch_triggers_on_new_log_file` being marked `#[ignore = "macOS FSEvents + test stdin-pipe block"]`. The problem is not macOS-specific: it reproduces wherever the process stdin is a pipe.

**Fix:** Guard in `process_log_path` before calling `handle_run`:

```rust
fn process_log_path(path: &Path, ...) {
    if !path.exists() {
        warn!("watch: triggered path no longer exists, skipping: {}", path.display());
        return;
    }
    // ... rest of the function unchanged
}
```

A more complete fix also adds a `disable_stdin_fallback` flag to `handle_run`'s parameter list so that watch-mode callers can never accidentally fall back to stdin regardless of the process environment.

---

### CR-02: `Error::Interrupted` from inner `handle_run` is silently swallowed — misleading log and deferred loop exit

**File:** `src/cli/watch.rs:303-304`

**Issue:** When the user presses Ctrl+C while a file is being processed, `handle_run` detects `interrupted == true` and returns `Err(Error::Interrupted)`. The catch-all `Err(e) => warn!("watch trigger error: {e}")` arm logs this as a runtime failure. The watch loop does eventually exit because `interrupted` is re-checked at line 144 after `handle_event` returns, but:

1. If multiple events are queued in the same `handle_event` invocation (multiple paths in `event.paths`), subsequent paths are still processed in full after the interrupted trigger, delaying exit.
2. Operator log files will contain `WARN watch trigger error: Interrupted by user` for every interrupted run, making routine graceful shutdowns look like failures.

**Fix:**

```rust
// src/cli/watch.rs, in process_log_path match arm
Err(crate::error::Error::Interrupted) => {
    // Re-arm the flag to ensure outer loop exits at next check.
    interrupted.store(true, std::sync::atomic::Ordering::Relaxed);
    // Do not log as error — this is normal graceful shutdown.
}
Err(e) => warn!("watch trigger error: {e}"),
```

---

## Warnings

### WR-01: Error-log truncation footer fires for exactly 10,000 parse errors — false positive

**File:** `src/cli/run/mod.rs:441`

**Issue:** The truncation sentinel is:
```rust
let truncated = stats.parse_error_records.len() == 10_000;
```
If a run produces exactly 10,000 parse errors (matching the cap, no overflow), the condition is `true` and the footer `[truncated; showing first 10000 of 10000 total parse errors]` is written, falsely implying records were silently dropped.

**Fix:**
```rust
let truncated = stats.parse_errors > stats.parse_error_records.len();
```
This is `true` only when `parse_errors` (the raw counter incremented for every error) exceeds the collected sample size.

---

### WR-02: Active watch status line shows only the first watched directory after the first trigger

**File:** `src/cli/watch.rs:349-357`

**Issue:** `refresh_active_status` (called on the timeout branch) builds the status display with only `watch_dirs.first()`:
```rust
let dir_str = watch_dirs
    .first()
    .map(|p| p.display().to_string())
    .unwrap_or_default();
```
The initial spinner (set in `build_progress_bar`) correctly uses `format_paths_display(watch_dirs)`, which shows all directories or "N directories" for > 3. After the first timeout-based refresh, the status bar silently degrades to showing only one directory. The triggered-path display in `process_log_path` (line 293–296) also shows only the parent of the most recently triggered file rather than the full watched set.

**Fix:** Use `format_paths_display(watch_dirs)` in both `refresh_active_status` and `process_log_path`:
```rust
// refresh_active_status:
let dir_str = format_paths_display(watch_dirs);
```

---

### WR-03: Log-rotation rename events are silently ignored — appended rotated-file content triggers duplicate processing

**File:** `src/cli/watch.rs:244-250`

**Issue:** The event filter accepts only `Create(_)` and `Modify(Data(Content))`. Many log-rotation strategies rename the active file (`app.log` → `app.log.1`) then create a new empty `app.log`. The rename produces a `Modify(ModifyKind::Name(_))` event which is dropped. The subsequent `Create` of the new `app.log` is detected and processed correctly. However, if the rotation tool then continues writing to `app.log.1` (or if a daemon writes a closing flush), those writes generate `Modify(Data(Content))` events for `app.log.1`, which passes the `.log` extension filter and triggers `handle_run` on the already-processed content. In CSV-append or SQLite-append mode this produces duplicate rows.

**Fix:** Maintain a `HashSet<PathBuf>` of already-processed file paths. Alternatively, document the limitation and add a test for append-mode deduplication behaviour during rotation.

---

### WR-04: `Ordering::Relaxed` used for both `ctrlc` signal write and `interrupted` reads — correct but fragile

**File:** `src/main.rs:173,218` / `src/cli/watch.rs:144` / `src/cli/run/mod.rs:148,307`

**Issue:** `Relaxed` ordering is used throughout for both stores (in the ctrlc handler) and loads (in the processing loops). For a single-bit cancellation flag that only transitions `false → true`, `Relaxed` is technically sufficient on all tier-1 Rust targets because the compiler and CPU cannot invent writes. However, the guard at `src/cli/run/mod.rs:307` checks `interrupted` inside a hot loop between individual record exports. On architectures with weak memory models (e.g. ARM), a `Relaxed` load is not guaranteed to observe the write until an arbitrarily long time after the store. In practice, the loop will exit "soon enough", but the exact latency is not bounded. For a safety-critical stop flag, `Acquire`/`Release` is the idiomatic Rust pairing.

This is a warning rather than a critical because no data loss results and the practical latency is sub-millisecond on all supported platforms.

**Fix:** Use `Ordering::Release` on the store (ctrlc handler) and `Ordering::Acquire` on all loads.

---

## Info

### IN-01: Dead `let _ = verbose;` statement at end of `handle_watch`

**File:** `src/cli/watch.rs:61`

**Issue:** `verbose` is consumed at line 48 when passed to `run_watch_loop`. The `let _ = verbose;` on line 61 is dead — the value has already been used by copy. In Rust, `let _ = copy_type;` silences an "unused variable" warning but `verbose` is not unused here. The statement misleads readers into thinking `verbose` might not be forwarded.

**Fix:** Remove line 61. If removing it exposes a compiler warning, that indicates a regression in the forwarding chain.

---

### IN-02: `test_watch_triggers_on_new_log_file` permanently ignored without issue number

**File:** `tests/integration.rs:2911`

**Issue:** The most important happy-path watch test is marked:
```rust
#[ignore = "macOS FSEvents + test stdin-pipe block; fix in Phase 70 smoke test"]
```
This covers WATCH-02/05 (primary trigger behaviour). Fixing CR-01 above removes the root cause and makes this test un-ignorable. Without a linked issue number the "Phase 70" deferral has no accountability mechanism.

**Fix:** Apply CR-01 fix, then remove `#[ignore]` and verify the test passes.

---

### IN-03: `collect_watch_dirs` uses linear `Vec::contains` for deduplication — O(n²) on large input lists

**File:** `src/cli/watch.rs:193, 204, 210`

**Issue:** Deduplication inside the loop uses `!dirs.contains(&dir)` three times, giving O(n²) overall for large `inputs` slices. Typical use involves a handful of directories, so this is not a correctness issue, but the function is `pub` and its contract does not bound input size.

**Fix:** Track seen paths with a `HashSet<PathBuf>` alongside the output `Vec`, or sort and `dedup` afterward.

---

_Reviewed: 2026-06-06T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
