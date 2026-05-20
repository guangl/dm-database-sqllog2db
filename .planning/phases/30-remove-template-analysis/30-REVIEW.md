---
phase: 30-remove-template-analysis
reviewed: 2026-05-20T10:30:00Z
depth: standard
files_reviewed: 28
files_reviewed_list:
  - benches/bench_csv.rs
  - benches/bench_filters.rs
  - benches/bench_sqlite.rs
  - Cargo.toml
  - docs/architecture.md
  - README.md
  - src/cli/init.rs
  - src/cli/opts.rs
  - src/cli/run/filter_processor.rs
  - src/cli/run/mod.rs
  - src/cli/run/parallel.rs
  - src/cli/run/prescan.rs
  - src/cli/run/processor.rs
  - src/cli/run/tests.rs
  - src/cli/show_config.rs
  - src/config/apply_one.rs
  - src/config/exporter.rs
  - src/config/logging.rs
  - src/config/mod.rs
  - src/config/sqllog.rs
  - src/config/validate.rs
  - src/error.rs
  - src/exporter/csv/mod.rs
  - src/exporter/csv/tests.rs
  - src/exporter/mod.rs
  - src/exporter/sqlite/mod.rs
  - src/exporter/sqlite/tests.rs
  - src/exporter/tests.rs
  - src/lang.rs
  - src/lib.rs
  - src/main.rs
  - src/pipeline/filters/compiled.rs
  - src/pipeline/filters/mod.rs
  - src/pipeline/filters/types.rs
  - src/pipeline/mod.rs
  - src/pipeline/normalizer.rs
  - tests/integration.rs
findings:
  critical: 1
  warning: 6
  info: 3
  total: 10
status: issues_found
---

# Phase 30: Code Review Report — Remove Template Analysis

**Reviewed:** 2026-05-20T10:30:00Z  
**Depth:** standard  
**Files Reviewed:** 28  
**Status:** issues_found

## Summary

Phase 30 removes template analysis (aggregator, template_reporter, companion modules) and the hdrhistogram dependency. The core removal work in the module structure is clean. However, there are significant issues with stale references, dead code that was missed, and — most critically — an invalid benchmark file that tests nothing relevant since Phase 30's config migration.

## Critical Issues

### CR-01: bench_filters.rs benchmarks are completely invalid — config sections silently ignored

**File:** `benches/bench_filters.rs:44-158`

**Issue:** All seven benchmark scenarios (`no_pipeline`, `pipeline_passthrough`, `trxid_small`, `trxid_large`, `indicator_prescan`, `exclude_passthrough`, `exclude_active`) use the old `[features.filters]` config section format to configure filters. However, `Config` has no `features` field — filters are now configured under the top-level `[filter]` section. Serde silently drops unknown fields/structures because `Config` does not use `#[serde(deny_unknown_fields)]`. This means every benchmark in this file tests the **same** no-filter fast path, regardless of the intended filter scenario.

Additionally, the base config template (`base_toml`, line 44) includes an `[error]` section that similarly has no corresponding struct field in `Config` and is silently dropped.

The benchmarks pass at runtime and produce numbers, but those numbers are meaningless — they all measure the same no-pipeline fast path. No filter logic is exercised, no pre-scan is triggered, and no regex compilation occurs.

The `[features.filters]` was likely the format in a previous version (v1.3 or earlier) but was never updated when the config was migrated to the top-level `[filter]` format.

**Fix:** Rewrite all benchmark scenario configs to use the current top-level `[filter]` format:
```rust
fn cfg_pipeline_passthrough(sqllog_dir: &Path, bench_dir: &Path) -> Config {
    let toml = format!(
        "{base}
[filter]
enable = true
[filter.include]
start_ts = \"2000-01-01\"
",
        base = base_toml(sqllog_dir, bench_dir)
    );
    toml::from_str(&toml).unwrap()
}
```

Similarly for `cfg_trxid_small`, `cfg_trxid_large`, `cfg_indicator_prescan`, `cfg_exclude_passthrough`, and `cfg_exclude_active` — all must use the `[filter]` / `[filter.include]` / `[filter.exclude]` format.

## Warnings

### WR-01: normalize_template and all associated functions are dead code

**File:** `src/pipeline/normalizer.rs:462`

**Issue:** The `normalize_template` function (line 462) is explicitly marked `#[allow(dead_code)]`, confirming it was used only by the removed template analysis feature. The following entire dependency chain is now dead code:

- `normalize_template` (line 462) — the public entry point, `#[allow(dead_code)]`
- `scan_sql_bytes` (line 467) — private, only called by `normalize_template`
- `dispatch_byte` (line 494) — called only by `scan_sql_bytes`
- `handle_quote` (line 518) — called only by `dispatch_byte`
- `handle_line_comment` (line 540) — called only by `dispatch_byte`
- `handle_block_comment` (line 551) — called only by `dispatch_byte`
- `handle_word` (line 565) — called only by `dispatch_byte`
- `try_fold_in_list` (line 593) — called only by `handle_word`
- `skip_quoted` (line 628) — called only by `try_fold_in_list` and `is_subquery`
- `is_subquery` (line 644) — called only by `try_fold_in_list`
- `is_keyword` (line 667) — called only by `handle_word`
- `is_ident_byte` (line 718) — called only by `handle_word`, `prev_is_ident_byte`
- `prev_is_ident_byte` (line 723) — called only by `handle_word`
- `const NEEDS_SPECIAL_NORM` (line 438) — called only by `scan_sql_bytes`

Additionally, the associated proptest and unit tests (lines 946-1021) are also dead weight.

**Fix:** Remove `normalize_template` function, all its private helpers, and the associated test block. The remaining normalizer.rs code (`compute_normalized`, `parse_params`, `ParamBuffer`, `ParamValue`, `count_placeholders`, `apply_params_into`) is still needed for the `replace_parameters` feature and should be kept.

### WR-02: SqliteExporter::open_connection_only is dead code

**File:** `src/exporter/sqlite/mod.rs:132`

**Issue:** `open_connection_only` is marked `#[allow(dead_code)]`. The doc comment (line 129-131) states: "Used for parallel CSV path to write template statistics." Since template statistics were removed, this method has no callers.

**Fix:** Remove the `open_connection_only` method and its `#[allow(dead_code)]` attribute.

### WR-03: docs/architecture.md contains stale template analysis references

**File:** `docs/architecture.md`

**Issue:** Multiple sections reference the removed template analysis feature:

1. Line 67: `normalize_template` mentioned under Pipeline features — but `normalize_template` is now dead code.
2. Lines 122-125: Full `TemplateAggregator` section describes hdrhistogram, template frequency accumulation, and latency distribution — all removed.
3. Line 66: Comment about "模板（template）聚合" in the Pipeline description.
4. Line 116: Pipeline description mentions "filter/template" together.

**Fix:** Remove the "TemplateAggregator" subsection (lines 122-125). Update all references to template analysis or note that it was removed. Fix the data flow diagram if it references template processing.

### WR-04: README.md contains stale "模板分析与图表" section

**File:** `README.md:34-40`

**Issue:** The entire "模板分析与图表" section describes the removed template analysis feature, including:

- `normalize_template` (dead code)
- TemplateAggregator and hdrhistogram (removed)
- SVG charts (frequency bar, latency histogram, trend line, user pie — removed)
- `[template]` and `[charts]` TOML configuration (no longer supported)
- "双路统计输出" for template CSV + SQLite (removed)

The "零开销快速路径" description (line 45) also mentions "无模板" as a condition for the fast path.

**Fix:** Remove the "模板分析与图表" section. Update the fast path description to remove "template" references. The section should be replaced with a note about the `replace_parameters` feature (the only remaining pipeline function).

### WR-05: Test function name references removed aggregator

**File:** `src/cli/run/tests.rs:56`

**Issue:** Test function `test_aggregator_disabled_none_path` includes "aggregator" in its name, which was part of the removed template analysis. The test body simply verifies that `handle_run` succeeds with default configuration — it has nothing to do with aggregation.

**Fix:** Rename to `test_handle_run_default_config_succeeds` or similar.

### WR-06: validate test has stale [template] section silently ignored

**File:** `src/config/validate.rs:468-484`

**Issue:** The test `test_validate_new_top_level_format_passes` includes `[template]` and `[filter]` sections in the TOML config, asserting `cfg.validate().is_ok()`. Since `[template]` has no corresponding field in `Config`, serde silently drops it. The test passes vacuously — it does not verify any template handling.

This creates a support risk: users who still have `[template]` in their config files after upgrading will have the template section silently dropped with no warning or error. This is especially dangerous if they expected template/chart output that no longer exists.

**Fix:** Either:
1. Remove the `[template]` line from the test, or
2. If backward compatibility is important, add a `template_deprecated` field (similar to `pipeline_deprecated`) that catches the `[template]` section and returns a clear migration error.

## Info

### IN-01: FileError::ReadFailed variant is dead code

**File:** `src/error.rs:59`

**Issue:** `FileError::ReadFailed` variant is marked `#[allow(dead_code)]` with a TODO comment "Phase 32 统一清理". It was likely kept for Phase 32 cleanup. Worth noting since Phase 30 is a good opportunity to reduce dead code.

**Fix:** Remove the variant and its `#[allow(dead_code)]` attribute, or address in Phase 32 as planned.

### IN-02: CompiledSqlFilters::has_filters is unused

**File:** `src/pipeline/filters/compiled.rs:217`

**Issue:** `CompiledSqlFilters::has_filters()` is marked `#[allow(dead_code)]`. The code in `cli/run/mod.rs:82-87` checks `f.record_sql.has_filters()` on `SqlFilters` (the un-compiled type) before building `CompiledSqlFilters`, so the compiled version's `has_filters()` is never called.

**Fix:** Either remove the method, or consider using it in `cli/run/mod.rs` after compilation instead of checking the un-compiled `SqlFilters::has_filters()`.

### IN-03: bench_csv.rs and bench_sqlite.rs use [error] section not in Config

**File:** `benches/bench_csv.rs:38-39`, `benches/bench_sqlite.rs:43-44`

**Issue:** Both benchmark files include an `[error]` section in their TOML config templates. There is no `error` field in `Config`. This is pre-existing (not introduced by Phase 30) but the section is silently dropped by serde.

**Fix:** Remove the `[error]` section from benchmark TOML templates since it has no effect.

## Hot Path Regression Analysis

The pipeline hot path (when `pipeline.is_empty()` is true) was reviewed for regression risk:

- **processor.rs:71-77**: The fast path correctly checks `pipeline.is_empty()` and sets `passes=true`. No regression.
- **processor.rs:81**: `needs_pm` is always `true` when `passes=true`. This is correct — records must always pass through the exporter path.
- **processor.rs:84**: `parse_meta()` is called even in the fast path, but this is the same cost as before (the exporter would call it internally). No net overhead increase.
- **processor.rs:89-98**: PerformanceMetrics construction with synthetic values when `include_pm=false`. No regression.
- **processor.rs:107-120**: `compute_normalized` is only called when `do_normalize` is enabled. In the default config (`do_normalize=false`), this is skipped entirely. No regression.

**Conclusion:** No hot path regressions detected. The fast path maintains its zero-overhead property.

---

_Reviewed: 2026-05-20T10:30:00Z_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
