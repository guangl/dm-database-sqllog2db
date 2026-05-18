---
phase: 17-filter-nesting
fixed_at: 2026-05-17T08:30:00Z
review_path: .planning/phases/17-filter-nesting/17-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 17: Code Review Fix Report

**Fixed at:** 2026-05-17
**Source review:** .planning/phases/17-filter-nesting/17-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 4 (1 Critical + 3 Warning; Info findings excluded per default scope)
- Fixed: 4
- Skipped: 0

## Fixed Issues

### CR-01: Silent data-loss when old flat fields coexist with new nested sub-table

**Files modified:** `src/features/filters.rs`
**Commit:** 14ee652
**Applied fix:** Added `flat_include_present` and `flat_exclude_present` boolean checks in `From<RawFiltersFeature>` before calling `unwrap_or()`. When both `raw.include` (new sub-table) and legacy flat include fields are simultaneously present, `log::warn!()` is emitted to alert the user that the sub-table takes priority and flat fields are being silently dropped. Same symmetric guard for the exclude side. Added `test_mixed_format_new_format_wins_and_warns` test to assert new format wins and old flat field is not present in the result.

### WR-01: stats.rs rate calculation uses integer division

**Files modified:** `src/cli/stats.rs`
**Commit:** 9aaf106
**Applied fix:** Replaced `total_records / elapsed.as_secs().max(1)` with f64-based calculation: `(total_records as f64 / elapsed_secs) as u64` guarded by `if elapsed_secs > 0.0`. Uses the already-computed `elapsed_secs` variable. Added `#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]` to suppress expected clippy lints for the intentional cast.

### WR-02: show_config.rs silently omits most filter fields after nesting refactor

**Files modified:** `src/cli/show_config.rs`
**Commit:** 96a36cc
**Applied fix:** Added display blocks for all previously missing filter fields following the existing `kv()` pattern: `include.sessions`, `include.threads`, `include.statements`, `include.apps`, `include.tags`; all `exclude.*` sub-table fields (`users`, `ips`, `sessions`, `threads`, `statements`, `apps`, `tags`); `indicators.exec_ids`, `indicators.min_runtime_ms`, `indicators.min_row_count`; `sql.includes`, `sql.excludes`; `record_sql.includes`, `record_sql.excludes`.

### WR-03: stats.rs silently ignores regex compilation errors in filter path

**Files modified:** `src/cli/stats.rs`
**Commit:** 0b1f6b0
**Applied fix:** Moved `CompiledMetaFilters::try_from_include_exclude()` out of `process_file()` to before the per-file loop in `handle_stats()`. On regex compilation error, prints to stderr with `color::red("Error:")` prefix and returns early. Added `compiled_meta: Option<&'a CompiledMetaFilters>`, `start_ts: Option<&'a str>`, and `end_ts: Option<&'a str>` fields to `ProcessFileCtx` struct. Removed per-file recompilation from `process_file()` and updated all references to use `ctx.*` fields. Added `CompiledMetaFilters` to the top-level import.

## Skipped Issues

None — all in-scope findings (Critical + Warning) were successfully fixed.

---

_Fixed: 2026-05-17_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
