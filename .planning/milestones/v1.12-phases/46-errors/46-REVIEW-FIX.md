---
phase: 46-errors
fixed_at: 2026-06-01T00:00:00Z
review_path: .planning/phases/46-errors/46-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 46: Code Review Fix Report

**Fixed at:** 2026-06-01
**Source review:** .planning/phases/46-errors/46-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 4
- Fixed: 4
- Skipped: 0

## Fixed Issues

### WR-01: `Validate` command duplicates hint-formatting logic, bypassing `format_error_output`

**Files modified:** `src/main.rs`
**Commit:** 62d3fe4
**Applied fix:** Extracted `format_validate_error` helper function that uses `[FAIL]` label (distinct from severity-based labels) and shares the hint-formatting logic. Replaced the 6-line inline `if/else` in the `Validate` command arm with a single `eprintln!("{}", format_validate_error(&e))` call. Added a doc comment explaining that `[FAIL]` is intentionally distinct from `[CRITICAL]/[ERROR]` to signal config validation failure vs a fatal runtime error.

### WR-02: `suggestion()` return value for `ConfigError::ParseFailed` is not unit-tested

**Files modified:** `src/main.rs`
**Commit:** 82c8861
**Applied fix:** Added `test_error_suggestion_for_config_parse_failed` unit test in `src/main.rs` alongside the existing `test_error_suggestion_for_config_not_found`. The test constructs `Error::Config(ConfigError::ParseFailed { .. })`, asserts the suggestion is non-empty, and asserts it contains "TOML" or "syntax". This guards the E2E hint assertion from silent regression.

### IN-01: `_verbose` parameters silently unused in `init_simple_logging` and `apply_verbosity_to_config`

**Files modified:** `src/main.rs`
**Commit:** e25cbd9
**Applied fix:** Removed the `_verbose: bool` parameter from both `init_simple_logging` and `apply_verbosity_to_config`. Added doc comments explaining that `--verbose` is intentionally a no-op for non-Run commands (non-Run commands use simple logging; debug verbosity requires the full logging stack initialized in the Run path). Updated all call sites (lines ~120, ~133) and the three related tests. Renamed `test_apply_verbosity_verbose` and `test_apply_verbosity_neither` to `test_apply_verbosity_quiet` and `test_apply_verbosity_not_quiet` to reflect the single `quiet` parameter.

### IN-02: `test_error_print_format_uses_hint_prefix` only tests one variant; no test for `Interrupted` guard or `ConfigError::ParseFailed` path

**Files modified:** `src/main.rs`
**Commit:** 0ed564d
**Applied fix:** Added two new unit tests:
- `test_interrupted_matches_guard_is_true`: Verifies that `matches!(e, Error::Interrupted)` is true for the `Interrupted` variant (documenting the guard in `main()`), and shows that if `format_error_output` were called it would produce a `[CRITICAL]` hint line — confirming the guard is necessary to suppress it.
- `test_format_error_output_config_parse_failed_is_critical`: Verifies that `ConfigError::ParseFailed` through `format_error_output` produces `[CRITICAL]` prefix and includes a `hint:` line, covering the exact variant exercised by the E2E test.

---

_Fixed: 2026-06-01_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
