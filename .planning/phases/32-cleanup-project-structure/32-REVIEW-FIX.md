---
phase: 32-cleanup-project-structure
fixed_at: 2026-05-20T11:25:00Z
review_path: .planning/phases/32-cleanup-project-structure/32-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 4
skipped: 1
status: partial
---

# Phase 32: Code Review Fix Report

**Fixed at:** 2026-05-20T11:25:00Z
**Source review:** .planning/phases/32-cleanup-project-structure/32-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5
- Fixed: 4
- Skipped: 1

## Fixed Issues

### WR-01: Dead code `FileError::ReadFailed` with explicit Phase 32 TODO not cleaned up

**Files modified:** `src/error.rs`
**Commit:** a57a5e6
**Applied fix:** Removed the `ReadFailed` variant and its `#[allow(dead_code)]` annotation from the `FileError` enum. The variant was never constructed anywhere -- only `AlreadyExists`, `CreateDirectoryFailed`, and `WriteFailed` are used.

### WR-02: Package description and error comment mention non-existent JSONL exporter

**Files modified:** `Cargo.toml`, `src/error.rs`
**Commit:** 6207aca
**Applied fix:** Removed "JSONL" from the package description (`CSV/JSONL/SQLite` -> `CSV/SQLite`) and from the `ExportError::WriteFailed` doc comment (`CSV、JSONL、错误日志` -> `CSV、错误日志`).

### IN-01: Test name references removed "aggregator" feature

**Files modified:** `src/cli/run/tests.rs`
**Commit:** 79f2d83
**Applied fix:** Renamed `test_aggregator_disabled_none_path` to `test_handle_run_default_config_succeeds` to reflect what the test actually validates.

### IN-02: `[template]` config section silently accepted in test fixture

**Files modified:** `src/config/validate.rs`
**Commit:** 4f1eeff
**Applied fix:** Removed the `[template]` / `enable = true` section from the `test_validate_new_top_level_format_passes` test fixture. The `[template]` section is not a valid config field and misled readers into thinking it was supported.

## Skipped Issues

### IN-03: Stale JSONL reference in dev-dependency explanations

**File:** `Cargo.toml:17`
**Reason:** Reviewer indicated "No code change needed; purely informational." The `/export/*` exclude entry has no functional impact and no actionable fix was suggested.
**Original issue:** `/export/*` in the Cargo.toml exclude list was historically relevant for a removed JSONL/export feature. Purely informational finding.

---

_Fixed: 2026-05-20T11:25:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
