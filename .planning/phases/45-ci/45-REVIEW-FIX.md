---
phase: "45"
fixed_at: 2026-05-25T00:00:00Z
review_path: .planning/phases/45-ci/45-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 45: Code Review Fix Report

**Fixed at:** 2026-05-25
**Source review:** .planning/phases/45-ci/45-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 3
- Fixed: 3
- Skipped: 0

## Fixed Issues

### WR-01: Parse errors silently dropped in SQLite parallel collect path

**Files modified:** `src/cli/run/sqlite_parallel.rs`
**Commit:** da03ac8
**Applied fix:** Replaced the bare `let Ok(record) = result else { continue }` in
`collect_log_file` with a `match` arm that increments a `parse_errors: usize` counter
and emits `log::warn!("{} | parse error: {e:?}", file.display())`.  The per-file count
is propagated back through `parallel_collect` (return type changed from `(collected,
skipped)` to `(collected, skipped, total_parse_errors)`) and a summary `log::warn!` is
emitted in `process_sqlite_parallel` when `total_parse_errors > 0`.  Behaviour now
mirrors `processor.rs` lines 146-150.

### WR-02: `test_parallel_merge_consistent` does not test sequential mode

**Files modified:** `src/cli/run/tests.rs`
**Commit:** 1356bb5
**Applied fix:** Restructured the test to place the "sequential" run in a dedicated
`seq/` subdirectory containing only one log file (`only.log`), so `log_files.len() == 1`
and the parallel branch is never triggered regardless of `available_parallelism()`.  The
"parallel" run uses a separate `par/` subdirectory with two files (`a.log`, `b.log`).
The assertion was updated to the correct invariant: the parallel run should produce one
more data row than the sequential run (two files vs one file).  The helper closure was
renamed `make_cfg_dir` to match the pattern in `test_sqlite_parallel_matches_sequential`.

### IN-01: Three parameters accepted but unconditionally discarded in `process_sqlite_parallel`

**Files modified:** `src/cli/run/sqlite_parallel.rs`
**Commit:** bb58f17
**Applied fix:** Renamed the three unused parameters to `_show_progress`, `_field_mask`,
and `_ordered_indices` so the compiler enforces the intentional non-use without a `let _`
suppression.  Expanded the doc comment to explain each parameter's status:
`_show_progress` — progress display not yet implemented for the `SQLite` parallel path;
`_field_mask` / `_ordered_indices` — already re-derived from `cfg` inside
`ExporterManager::from_config`, so forwarding them here is redundant.  The
`let _ = (show_progress, field_mask, ordered_indices)` line was removed entirely.

---

_Fixed: 2026-05-25_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
