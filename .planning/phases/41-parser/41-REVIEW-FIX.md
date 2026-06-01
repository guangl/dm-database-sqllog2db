---
phase: "41"
fixed_at: 2026-05-25T00:00:00Z
review_path: .planning/phases/41-parser/41-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 41: Code Review Fix Report

**Fixed at:** 2026-05-25T00:00:00Z
**Source review:** .planning/phases/41-parser/41-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5
- Fixed: 5
- Skipped: 0

## Fixed Issues

### WR-01: `scan_log_file_for_matches` silently discards file-open errors

**Files modified:** `src/cli/run/prescan.rs`
**Commit:** 2da5257
**Applied fix:** Replaced `let Ok(parser) = ... else { return Vec::new() }` with an explicit `match` that calls `log::warn!("Pre-scan: failed to open '{file_path}': {e}")` before returning the empty Vec. Format argument inlined per clippy `uninlined_format_args`.

---

### WR-02: `eprintln!` bypasses the `log` crate in orchestration code

**Files modified:** `src/cli/run/prescan.rs`
**Commit:** fa1e077
**Applied fix:** Replaced `eprintln!("Pre-scanning {} files...", log_files.len())` with `log::info!("Pre-scanning {} files...", log_files.len())` so the message respects RUST_LOG, --quiet, and the configured rolling-file log sink.

---

### WR-03: `to_string_lossy()` silently corrupts non-UTF8 file paths

**Files modified:** `src/cli/run/prescan.rs`
**Commit:** 11a6e27
**Applied fix:** Replaced `&file.to_string_lossy()` with `if let Some(path) = file.to_str()` pattern. When `to_str()` returns `None` (non-UTF8 path), emits `log::warn!("Pre-scan: skipping file with non-UTF8 path: {}", file.display())` and returns `Vec::new()`. Used `file.display()` instead of `{:?}` per clippy `unnecessary_debug_formatting`.

---

### IN-01: Cross-file merge produces duplicate `trxid` strings before `HashSet` dedup

**Files modified:** `src/cli/run/prescan.rs`
**Commit:** c8196fb
**Applied fix:** Changed the collection type inside `pool.install()` from `Vec<String>` to `std::collections::HashSet<String>`, then converted back to `Vec` via `.into_iter().collect()`. This deduplicates cross-file trxids in a single parallel pass, eliminating unnecessary String allocations for trxids that appear in multiple log files.

---

### IN-02: `recompile_meta_if_needed` doc/logic inconsistency for empty filters

**Files modified:** `src/cli/run/prescan.rs`
**Commit:** 50f7b4d
**Applied fix:** Added an early-return guard after the `filter.enable` check: if neither `filters.include.has_filters()` nor `filters.exclude.has_filters()`, the function returns `Ok(original)` instead of creating a `Some(CompiledMetaFilters { all fields: None })` that `build_pipeline` would immediately discard. Updated the doc comment to list "include/exclude 均为空" as an additional early-return case and removed the inaccurate claim about returning the original in all "no filters" scenarios.

---

_Fixed: 2026-05-25T00:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
