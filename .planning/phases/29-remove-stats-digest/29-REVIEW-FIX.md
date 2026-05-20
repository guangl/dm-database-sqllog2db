---
phase: 29-remove-stats-digest
fixed_at: 2026-05-20T13:30:00Z
review_path: .planning/phases/29-remove-stats-digest/29-REVIEW.md
iteration: 1
findings_in_scope: 9
fixed: 9
skipped: 0
status: all_fixed
---

# Phase 29: Code Review Fix Report

**Fixed at:** Wed May 20 13:30:00 CST 2026
**Source review:** .planning/phases/29-remove-stats-digest/29-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 9
- Fixed: 9
- Skipped: 0

## Fixed Issues

### CR-01: bench_filters.rs uses `[features.filters]` instead of `[filter]`

**Files modified:** `benches/bench_filters.rs`
**Commit:** 128f2b0
**Applied fix:** Replaced all 6 occurrences of `[features.filters]` with `[filter]` in config builder functions (cfg_pipeline_passthrough, cfg_trxid_small, cfg_trxid_large, cfg_indicator_prescan, cfg_exclude_passthrough, cfg_exclude_active). The old section name was silently ignored by serde since Config has no `features` field, causing all benchmarks to measure the no-filter fast path.

### WR-01: README.md contains stale references to removed `stats` and `digest` commands

**Files modified:** `README.md`
**Commit:** ff4a952
**Applied fix:** Removed stats/digest from the CLI commands bullet list; replaced stats/digest usage examples with `sqllog2db validate` and `sqllog2db show-config`; removed paragraph referencing `sqllog2db stats --chart` for SVG generation, replaced with reference to `[charts]` config section.

### WR-02: docs/architecture.md contains stale references to removed modules

**Files modified:** `docs/architecture.md`
**Commit:** 51bff68
**Applied fix:** Removed paragraph about stats/digest commands reading exported output files; removed stats.rs and digest.rs bullet points from CLI module listing; updated handler function pattern example from handle_stats to handle_validate.

### WR-03: SqliteExporter::open_connection_only() is dead code

**Files modified:** `src/exporter/sqlite/mod.rs`
**Commit:** df8cf34
**Applied fix:** Removed the `open_connection_only()` method and its `#[allow(dead_code)]` annotation. The function was part of the removed stats/digest template analysis path.

### WR-04: normalize_template() is dead code in normalizer.rs

**Files modified:** `src/pipeline/normalizer.rs`
**Commit:** b492145
**Applied fix:** Added TODO comment documenting the intended re-integration point. The function was preserved (not deleted) because the template pipeline (TemplateAggregator, charts) is still active; it awaits re-integration into the Pipeline. Also removed migration comment referencing deleted `fingerprint.rs`.

### WR-05: No test coverage for pre-compiled filter path

**Files modified:** `src/cli/run/tests.rs`
**Commit:** 6a40733
**Applied fix:** Added `test_precompiled_filter_path` test that calls `cfg.validate_and_compile()`, passes the result to `handle_run`, and verifies the output contains expected records.

### IN-01: FileError::ReadFailed dead code

**Files modified:** `src/error.rs`
**Commit:** a57a5e6 (Phase 32 fix, already applied before this fix round)
**Applied fix:** The `FileError::ReadFailed` variant was already removed by Phase 32 cleanup. No additional action needed.

### IN-02: Migration comments reference removed fingerprint.rs

**Files modified:** `src/pipeline/normalizer.rs`
**Commit:** b492145 (included in WR-04 commit)
**Applied fix:** Updated the test section comment to remove reference to the deleted `fingerprint.rs`.

### IN-03: Inconsistent import paths in validate.rs

**Files modified:** `src/config/validate.rs`
**Commit:** 41f3eb5
**Applied fix:** Changed `crate::pipeline::filters::CompiledMetaFilters` and `crate::pipeline::filters::CompiledSqlFilters` to the shorter re-exported paths (`crate::pipeline::CompiledMetaFilters` and `crate::pipeline::CompiledSqlFilters`) for consistency with the usage in `validate_and_compile`.

---

_Fixed: 2026-05-20T13:30:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
