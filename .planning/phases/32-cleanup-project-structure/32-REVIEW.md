---
phase: 32-cleanup-project-structure
reviewed: 2026-05-20T10:00:00Z
depth: standard
files_reviewed: 6
files_reviewed_list:
  - src/config/mod.rs
  - src/config/validate.rs
  - tests/integration.rs
  - Cargo.toml
  - src/cli/run/tests.rs
  - src/cli/run/processor.rs
findings:
  critical: 0
  warning: 5
  info: 1
  total: 6
status: issues_found
---

# Phase 32: Code Review Report

**Reviewed:** 2026-05-20T10:00:00Z
**Depth:** standard
**Files Reviewed:** 6
**Status:** issues_found

## Summary

Reviewed 6 source files for residual references to removed features, dead code, security issues, and code quality problems. No security vulnerabilities or data-loss risks were found. The main issues are: a misleading package description (mentions non-existent JSONL exporter), a test that silently accepts an unknown config section (`[template]`), duplicated filter compilation logic, a test name referencing a removed "aggregator" feature, and dead code in the compiled filters module.

---

## Warnings

### WR-01: Package description mentions non-existent JSONL exporter

**File:** `Cargo.toml:7`
**Issue:** The package description reads `"高性能 CLI 工具：流式解析达梦数据库 SQL 日志并导出到 CSV/JSONL/SQLite"` but there is no JSONL exporter in the codebase. Only CSV and SQLite exporters exist (`CsvExporterConfig`, `SqliteExporterConfig` in `src/config/mod.rs`). A user evaluating this tool for JSONL output would be misled.

**Fix:** Remove "JSONL/" from the description or implement the JSONL exporter:
```toml
description = "高性能 CLI 工具：流式解析达梦数据库 SQL 日志并导出到 CSV/SQLite"
```

---

### WR-02: Test silently accepts unrecognized `[template]` config section

**File:** `src/config/validate.rs:468-484`
**Issue:** The test `test_validate_new_top_level_format_passes` on line 468 includes a `[template]` TOML section (`[template]\nenable = true`), but the `Config` struct in `src/config/mod.rs` defines no `template` field. Because serde is not configured with `deny_unknown_fields`, the `[template]` section is silently dropped without any warning or error. This means a user migrating from a legacy config that contained `[template]` would not be notified that the section is ignored. Either the Config struct should explicitly reject unknown top-level sections, or the test should not include this phantom section.

**Fix:** Either add a `#[serde(deny_unknown_fields)]` attribute (or a dedicated deprecation detection like `pipeline_deprecated`), or remove the `[template]` section from the test:
```rust
// Remove the [template] section from the test TOML
// Or add legacy detection:
#[doc(hidden)]
#[serde(rename = "template", default)]
pub template_deprecated: Option<toml::Value>,
```

---

### WR-03: Duplicated filter compilation logic between `validate_filter()` and `validate_and_compile()`

**File:** `src/config/validate.rs:47-55` and `src/config/validate.rs:67-80`
**Issue:** Both `validate_and_compile()` (lines 47-55) and `validate_filter()` (lines 67-80) independently compile the same regex filters from `self.filter` using `CompiledMetaFilters::try_from_include_exclude()` and `CompiledSqlFilters::try_from_sql_filters()`. The `validate()` method calls `validate_filter()` (line 16), which compiles and discards the results. Meanwhile, `validate_and_compile()` repeats the identical compilation. This is a DRY violation that doubles compilation and creates risk of the two paths diverging.

**Fix:** Refactor `validate_filter()` to delegate to a shared helper, or have `validate()` call `validate_and_compile()` and discard the output:
```rust
fn validate(&self) -> Result<()> {
    if self.pipeline_deprecated.is_some() { /* ... */ }
    self.logging.validate()?;
    self.exporter.validate()?;
    self.sqllog.validate()?;
    // Reuse validate_and_compile for filter validation, discard result
    self.validate_and_compile()?;
    Ok(())
}
```
(This would also require adjusting the method signature, since `validate_filter()` does not need compilation in the `validate()` path when `validate()` is called directly.)

---

### WR-04: Test name references removed "aggregator" feature

**File:** `src/cli/run/tests.rs:56`
**Issue:** The test function is named `test_aggregator_disabled_none_path`, but no "aggregator" feature exists in the current codebase. The test body only verifies that `handle_run` succeeds with a default configuration (no aggregator-specific setup or assertions). This is a residual reference from a removed feature that should have been cleaned up during the feature removal phase.

**Fix:** Rename the test to reflect what it actually tests:
```rust
fn test_handle_run_default_config_succeeds() {
```

---

### WR-05: `CompiledSqlFilters::has_filters()` is dead code

**File:** `src/pipeline/filters/compiled.rs:214-218`
**Issue:** The method `has_filters()` on `CompiledSqlFilters` is annotated with `#[allow(dead_code)]` and is never called by any production code. The `matches()` method on the same struct is the one used in the hot loop (called at `src/cli/run/processor.rs:103`). Code kept alive solely by `#[allow(dead_code)]` adds maintenance burden and confusion.

**Fix:** Remove the dead method and its `#[allow(dead_code)]` suppression:
```rust
// Remove the entire has_filters() method from CompiledSqlFilters
```

---

## Info

### IN-01: `normalize_template` is dead code from removed fingerprint/template-analysis feature

> **Note:** This function is defined in `src/pipeline/normalizer.rs:462-463`, which is outside the reviewed file list. It is noted here because it is a residual reference discovered during cross-referencing.

The function `normalize_template` has `#[allow(dead_code)]` and `#[must_use]` but is only called in test code (within `normalizer.rs`). The comment on line 460 explicitly states it was migrated from the removed `fingerprint.rs` module: "原位于 `fingerprint.rs`，为保留模板管道功能迁移至此。" If the "template analysis" feature is permanently removed, this function (36 lines of body plus the internal `scan_sql_bytes` helper) and its associated tests should be removed.

---

_Reviewed: 2026-05-20T10:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
