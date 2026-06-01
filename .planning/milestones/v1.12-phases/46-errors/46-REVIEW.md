---
phase: 46-errors
reviewed: 2026-06-01T00:00:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - src/main.rs
  - tests/integration.rs
findings:
  critical: 0
  warning: 2
  info: 2
  total: 4
status: issues_found
---

# Phase 46: Code Review Report

**Reviewed:** 2026-06-01
**Depth:** standard
**Files Reviewed:** 2
**Status:** issues_found

## Summary

Phase 46 optimized error output by replacing a `Suggestion:` prefix with `  hint: ` and extracted a `format_error_output` helper in `src/main.rs`. The new E2E test in `tests/integration.rs` correctly exercises the hint path via the `test_cli_error_uses_hint_prefix` test.

The logic of `format_error_output` is correct, and the hint prefix change is consistently applied in the `Err(e)` path at `main()`. However, two substantive issues were found:

1. The `Validate` command (lines 141-154) duplicates the hint-formatting logic inline instead of calling `format_error_output`, producing a divergent `[FAIL]` label that bypasses the shared function. This is a maintenance defect and a semantic inconsistency.
2. The E2E test `test_cli_error_uses_hint_prefix` requires a pre-built binary via `env!("CARGO_BIN_EXE_sqllog2db")`. This macro resolves at compile time and is correct for integration tests built by Cargo, but the test has no guard ensuring the config error actually triggers a hint (it relies entirely on `ConfigError::ParseFailed` having a non-empty `suggestion()`). If that suggestion is ever made empty the test silently regresses — the assertion on `hint:` would fail but there is no test that `suggestion()` for `ParseFailed` is non-empty in unit tests of the error module itself.

---

## Warnings

### WR-01: `Validate` command duplicates hint-formatting logic, bypassing `format_error_output`

**File:** `src/main.rs:143-149`

**Issue:** The `Validate` command arm re-implements the same `if hint.is_empty() / else` pattern inline rather than calling the newly extracted `format_error_output`. This means `Validate` errors use a hard-coded `[FAIL]` label that never varies with `error.severity()`, while fatal errors from all other paths use `[{severity}]` (e.g. `[CRITICAL]`, `[ERROR]`). The two code paths can diverge silently if either is modified. The `test_cli_validate_invalid_config_outputs_fail_prefix` test even asserts `!stderr.contains("[CRITICAL]")` and `!stderr.contains("[ERROR]")`, cementing this inconsistency.

```rust
// Current (lines 143-149): inline duplication, fixed [FAIL] label
if let Err(e) = cfg.validate() {
    let hint = e.suggestion();
    if hint.is_empty() {
        eprintln!("[FAIL] {e}");
    } else {
        eprintln!("[FAIL] {e}\n  hint: {hint}");
    }
    std::process::exit(EXIT_FATAL);
}

// Fix option A: extract a format_validate_error helper that uses [FAIL]:
fn format_validate_error(error: &Error) -> String {
    let hint = error.suggestion();
    if hint.is_empty() {
        format!("[FAIL] {error}")
    } else {
        format!("[FAIL] {error}\n  hint: {hint}")
    }
}

// Fix option B: if [FAIL] vs [CRITICAL]/[ERROR] distinction is intentional,
// document it in a comment and add a test that [FAIL] is NOT replaced by severity.
// Regardless, the inline duplication of the is_empty/hint logic should be removed.
```

### WR-02: `suggestion()` return value for `ConfigError::ParseFailed` is not unit-tested; E2E hint assertion has a hidden dependency

**File:** `tests/integration.rs:906-913`

**Issue:** `test_cli_error_uses_hint_prefix` asserts `stderr.contains("  hint: ")`, which depends on `Error::Config(ConfigError::ParseFailed { .. }).suggestion()` returning a non-empty string. There is no dedicated unit test in `src/error.rs` (or in `src/main.rs`'s inline tests) that asserts `ConfigError::ParseFailed` specifically produces a non-empty suggestion. If the suggestion string for `ParseFailed` is ever set to `""`, the E2E test becomes the only guard — but it will fail with a confusing message about missing `"  hint: "` rather than a clear assertion about the suggestion value.

A unit test should be added alongside the existing `test_error_suggestion_for_config_not_found`:

```rust
#[test]
fn test_error_suggestion_for_config_parse_failed() {
    let e = Error::Config(ConfigError::ParseFailed {
        path: "/tmp/bad.toml".into(),
        reason: "unexpected EOF".into(),
    });
    let s = e.suggestion();
    assert!(
        !s.is_empty(),
        "ParseFailed should have a non-empty suggestion, got empty"
    );
    assert!(
        s.contains("TOML") || s.contains("syntax"),
        "ParseFailed suggestion should mention TOML syntax; got: {s}"
    );
}
```

---

## Info

### IN-01: `_verbose` parameters in `init_simple_logging` and `apply_verbosity_to_config` are silently unused

**File:** `src/main.rs:27, 41`

**Issue:** Both functions accept `_verbose: bool` but never read it. The leading underscore suppresses the compiler warning, but the parameter is dead code. If `--verbose` should affect simple logging (e.g., setting `LevelFilter::Debug` for non-Run commands), this is a latent bug. If verbose is intentionally ignored for non-Run commands, a comment should explain why.

**Fix:** If `--verbose` is intentionally no-op for `init`/`validate`, remove the parameter from both functions and their call sites, or add a `// verbose flag intentionally ignored for non-Run commands` comment. Do not use underscore-prefixed parameters to silently swallow live arguments.

### IN-02: `test_error_print_format_uses_hint_prefix` only tests `ExportError::WriteFailed`; other hint-bearing error variants not covered

**File:** `src/main.rs:326-344`

**Issue:** The new unit test for `format_error_output` exercises a single variant (`ExportError::WriteFailed`) but `format_error_output` is also called for `ConfigError`, `FileError`, and `IoError` variants. In particular, `Error::Interrupted` has `suggestion()` return `"Run was interrupted by user."` which is non-empty, so it would produce a hint line — but `Interrupted` is excluded from `format_error_output` by the `matches!(e, Error::Interrupted)` guard in `main()`. This is correct behavior, but it is not tested: there is no test that `Interrupted` does NOT go through `format_error_output`.

**Fix:** Add a test that `Error::Interrupted` exits with `EXIT_INTERRUPTED` (130) without printing a hint line. Also add a test for `ConfigError::ParseFailed` through `format_error_output` to verify `[CRITICAL]` prefix, since that is the exact variant exercised by the E2E test.

---

_Reviewed: 2026-06-01_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
