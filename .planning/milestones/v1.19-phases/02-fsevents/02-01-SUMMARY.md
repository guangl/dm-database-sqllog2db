---
phase: 02-fsevents
plan: 01
subsystem: testing
tags: [watch, integration-test, csv-append, error-log, exit-code, rust]

# Dependency graph
requires:
  - phase: 01-watch
    provides: trigger_full_file, handle_watch, force_append_for_watch_trigger, WatchLoopState

provides:
  - WATCH-07/08/09 integration tests in tests/watch_incremental.rs
  - build_csv_config helper (CSV-only Config construction)
  - INVALID_LOG_LINE constant for parse-error triggering

affects: [02-fsevents]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Integration test pattern: TempDir + direct trigger_full_file calls + file content assertion"
    - "Error log verification: filter lines starting with [ERROR] from accumulated append output"

key-files:
  created: []
  modified:
    - tests/watch_incremental.rs

key-decisions:
  - "Extend existing watch_incremental.rs (not new file) per D-08"
  - "build_csv_config sets append=false initially; force_append_for_watch_trigger injects true per Pitfall 3"
  - "WATCH-09 uses handle_watch with pre-set interrupted=true to bypass watcher loop entirely"
  - "FSEvents #[ignore] in tests/integration.rs:2917 preserved per D-01"

patterns-established:
  - "build_csv_config(log_path, csv_path) -> Config: reusable CSV-only test config factory"
  - "trigger_full_file twice pattern: first creates file, second appends; header appears once"

requirements-completed: [QUAL-02, QUAL-03]

# Metrics
duration: 12min
completed: 2026-06-07
---

# Phase 02 Plan 01: WATCH-07/08/09 Integration Tests Summary

**Three integration tests (CSV append, error log append, interrupted exit) added to tests/watch_incremental.rs with build_csv_config helper and INVALID_LOG_LINE constant**

## Performance

- **Duration:** 12 min
- **Started:** 2026-06-07T02:00:00Z
- **Completed:** 2026-06-07T02:12:00Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- Added `test_watch_07_csv_append`: verifies two `trigger_full_file` calls produce 1 header + 6 data rows, header appears exactly once (CSV append mode working correctly)
- Added `test_watch_08_error_log_append`: verifies error log accumulates at least 2 `[ERROR]` lines across two triggers with invalid log lines
- Added `test_watch_09_exit_code_130`: verifies `handle_watch` returns `Err(Error::Interrupted)` when `interrupted=true` is pre-set (maps to exit 130 in main.rs)
- Added `build_csv_config` helper for CSV-only Config construction following existing `build_sqlite_config` pattern
- Added `INVALID_LOG_LINE` constant for triggering parse errors in WATCH-08

## Task Commits

1. **Task 1: WATCH-07/08/09 集成测试 + helper + 常量** - `8ffecc4` (feat)

## Files Created/Modified

- `tests/watch_incremental.rs` - Added 3 integration tests, build_csv_config helper, INVALID_LOG_LINE constant, extended use imports

## Decisions Made

- Used direct `trigger_full_file` calls (not notify watcher) per existing WATCH-03/04 pattern
- `build_csv_config` sets `append=false, overwrite=true` as initial values; `force_append_for_watch_trigger` injects `append=true` at trigger time (Pitfall 3)
- WATCH-09 passes `tmp.path()` as log dir to `handle_watch`; function checks `interrupted` immediately and returns before processing files
- D-01 preserved: `tests/integration.rs:2917` `#[ignore]` annotation unchanged

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed doc comment markdown lint warning**
- **Found during:** Task 1 (clippy check)
- **Issue:** `watch/mod.rs::DM_LOG_LINE_GARBAGE` in doc comment lacked backticks, triggering `clippy::doc_markdown`
- **Fix:** Wrapped identifier in backticks per clippy suggestion
- **Files modified:** tests/watch_incremental.rs
- **Verification:** `cargo clippy --all-targets -- -D warnings` passed with 0 warnings
- **Committed in:** `8ffecc4` (part of task commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - minor doc lint)
**Impact on plan:** No scope creep. Fix was a one-character-class change required for clippy compliance.

## Issues Encountered

- `cargo fmt` reformatted `trigger_full_file` call arguments from single-line to multi-line style; applied automatically before final commit.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- WATCH-07/08/09 integration tests complete and passing (7 tests total in watch_incremental.rs)
- `tests/integration.rs:2917` FSEvents `#[ignore]` preserved with book-recorded rationale (D-01/D-02)
- Ready for Plan 02 (collector.rs coverage, QUAL-02 gap closure)

## Self-Check: PASSED

- `tests/watch_incremental.rs` exists and contains 3 new test functions
- Commit `8ffecc4` exists
- `cargo test --test watch_incremental`: 7 passed, 0 failed
- `cargo clippy --all-targets -- -D warnings`: 0 warnings
- `cargo fmt --check`: passed
- `tests/integration.rs:2917` #[ignore] preserved

---
*Phase: 02-fsevents*
*Completed: 2026-06-07*
