---
phase: 32-cleanup-project-structure
reviewed: 2026-05-20T11:00:00Z
depth: standard
files_reviewed: 6
files_reviewed_list:
  - Cargo.toml
  - src/cli/run/processor.rs
  - src/cli/run/tests.rs
  - src/config/mod.rs
  - src/config/validate.rs
  - tests/integration.rs
findings:
  critical: 0
  warning: 2
  info: 3
  total: 5
status: issues_found
---

# Phase 32: Code Review Report

**Reviewed:** 2026-05-20T11:00:00Z
**Depth:** standard
**Files Reviewed:** 6
**Status:** issues_found

## Summary

Reviewed 6 source files at standard depth for Phase 32 (cleanup project structure after feature removals). The code compiles with zero clippy warnings, all dependencies are used, and there are no security vulnerabilities. However, several stale references from removed features were found: (1) the package description and an error comment still mention a non-existent JSONL exporter, (2) dead code `FileError::ReadFailed` carries an explicit Phase 32 TODO that was not addressed, (3) a test name references a removed "aggregator" feature, and (4) a test fixture includes a `[template]` section that is not a valid config field, creating an inconsistency with the `[pipeline]` deprecation guard.

---

## Warnings

### WR-01: Dead code `FileError::ReadFailed` with explicit Phase 32 TODO not cleaned up

**File:** `src/error.rs:59-60`
**Issue:** The `FileError::ReadFailed` variant has `#[allow(dead_code)]` and `// TODO: Phase 32 统一清理`, but remains in the codebase after Phase 32. It is never constructed anywhere -- only `FileError::AlreadyExists`, `FileError::CreateDirectoryFailed`, and `FileError::WriteFailed` are used. The explicit TODO marking it for Phase 32 cleanup was not fulfilled.

**Fix:** Remove the variant and its `#[allow(dead_code)]` annotation:

```rust
// Remove these three lines (58-60):
//     #[error("Failed to read file {path}: {reason}")]
//     #[allow(dead_code)] // TODO: Phase 32 统一清理
//     ReadFailed { path: PathBuf, reason: String },
```

---

### WR-02: Package description and error comment mention non-existent JSONL exporter

**File:** `Cargo.toml:7`, `src/error.rs:83`
**Issue:** The package description reads `"高性能 CLI 工具：流式解析达梦数据库 SQL 日志并导出到 CSV/JSONL/SQLite"` but JSONL is not a supported exporter. The `ExporterConfig` only contains `csv` and `sqlite` fields (`src/config/exporter.rs:6-7`). The doc comment on `ExportError::WriteFailed` at `src/error.rs:83` also mentions JSONL (`"文件写入失败（CSV、JSONL、错误日志等所有文件型导出器通用）"`). These are stale references from a removed JSONL feature.

**Fix:** Update the package description and error comment:

```toml
# Cargo.toml:7
description = "高性能 CLI 工具：流式解析达梦数据库 SQL 日志并导出到 CSV/SQLite"
```

```rust
// src/error.rs:83
/// 文件写入失败（CSV、错误日志等所有文件型导出器通用）
```

---

## Info

### IN-01: Test name references removed "aggregator" feature

**File:** `src/cli/run/tests.rs:56`
**Issue:** The test `test_aggregator_disabled_none_path` references an "aggregator" feature that no longer exists. The test body only verifies that `handle_run` succeeds with a default configuration (no aggregator-specific setup or assertions). Originally the aggregator config required export paths to be explicitly set, and this test verified the override path; now the test reduces to a default config smoke test.

**Fix:** Rename the test to reflect what it actually tests:

```rust
fn test_handle_run_default_config_succeeds() {
```

---

### IN-02: `[template]` config section silently accepted in test fixture

**File:** `src/config/validate.rs:468-484`
**Issue:** The test `test_validate_new_top_level_format_passes` includes a `[template]` TOML section with `enable = true`. The `Config` struct has no `template` field, and serde silently drops unknown sections. This contrasts with `[pipeline]`, which IS explicitly captured via `pipeline_deprecated: Option<toml::Value>` and rejected during validation. While unknown fields are standard serde behavior, including `[template]` in a test named "new_top_level_format_passes" is misleading -- it implies `[template]` is a valid section when it is not.

**Fix:** Replace `[template]` with a clearly inert key like `[unknown_section]` to avoid implying it is a valid config section:

```rust
fn test_validate_new_top_level_format_passes() {
    let toml = r#"
[sqllog]
path = "sqllogs"
[output]
fields = ["ts", "sql", "username"]
[filter]
enable = false
[exporter.csv]
file = "out.csv"
"#;
```

---

### IN-03: Stale JSONL reference in dev-dependency explanations

**File:** `Cargo.toml:17`
**Issue:** The `exclude` list in Cargo.toml includes `/export/*`. This was historically relevant for the JSONL and template-analysis output feature. It has no functional impact but is a stale reference that could be cleaned during a structure review.

No code change needed; purely informational.

---

_Reviewed: 2026-05-20T11:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
