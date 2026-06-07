---
phase: 01-watch
reviewed: 2026-06-07T00:00:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - src/config/mod.rs
  - src/cli/run/mod.rs
  - src/cli/watch/mod.rs
  - src/cli/run/tests.rs
  - tests/integration.rs
findings:
  critical: 0
  warning: 3
  info: 2
  total: 5
status: issues_found
---

# Phase 01-watch: Code Review Report

**Reviewed:** 2026-06-07
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

This phase introduces the `watch` subcommand (`src/cli/watch/mod.rs`), the `Config.append_error_log` field, the `force_append_for_watch_trigger` helper, and the WATCH-09 change making `handle_watch` return `Err(Error::Interrupted)` on SIGINT rather than `Ok(())`.

The core logic is correct: `force_append_for_watch_trigger` correctly sets both CSV and SQLite exporters to append mode (lines 521–533), debounce logic is sound and well-tested, offset persistence via `_watch_offsets` table is properly isolated from the `SqliteExporter` EXCLUSIVE lock, and the incremental path via temporary file is functionally correct.

Three stale comments/assertions from the pre-WATCH-09 state were not cleaned up alongside the behavior change, creating a misleading picture for readers. Two INFO items note missing test coverage.

---

## Warnings

### WR-01: Ignored W3 integration test panics if un-ignored — `.unwrap()` on `Err(Interrupted)`

**File:** `tests/integration.rs:2941`

**Issue:** `test_watch_triggers_on_new_log_file` is `#[ignore]`d, but its body ends with `handle_watch(...).unwrap()`. The spawned thread sets `interrupted = true` before the test ends, and `handle_watch` returns `Err(Error::Interrupted)` when the flag is set (WATCH-09). If this test is ever un-ignored, `.unwrap()` will panic and the test will fail with a misleading message rather than a useful assertion failure.

**Fix:**
```rust
// Replace line 2941:
handle_watch(&cfg, true, false, &interrupted).unwrap();
// With:
let result = handle_watch(&cfg, true, false, &interrupted);
assert!(
    result.is_ok() || matches!(result, Err(dm_database_sqllog2db::error::Error::Interrupted)),
    "handle_watch should succeed or return Interrupted, got: {result:?}"
);
```

### WR-02: W2 test doc comment documents the old `Ok(())` return value

**File:** `tests/integration.rs:2892`

**Issue:** The doc-comment on `test_watch_exits_when_interrupted` reads:
> "W2: interrupted=true 预置时 `handle_watch` 立即返回 Ok(())（WATCH-06 优雅退出）"

The assertion in the body correctly checks for `Err(Error::Interrupted)` (the WATCH-09 contract), so the test passes. But the comment actively misleads any reader about the expected return value. Anyone relying on this comment for the behavioral specification sees the wrong contract.

**Fix:**
```rust
/// W2: interrupted=true 预置时 `handle_watch` 返回 Err(Error::Interrupted)（WATCH-09 exit 130）。
```

### WR-03: Unit test `test_interrupted_flag_exits_immediately` has stale inline comment

**File:** `src/cli/watch/mod.rs:631`

**Issue:** The comment reads:
> "但 interrupted=true 时如果目录存在则立即跳出 loop 返回 Ok"

After WATCH-09, `handle_watch` returns `Err(Error::Interrupted)` when the loop exits due to the flag, not `Ok(())`. The test itself is harmless (it discards the result with `let _ = result`), but the comment incorrectly describes the post-WATCH-09 control flow for anyone tracing the code.

**Fix:**
```rust
// 但 interrupted=true 时如果目录存在则跳出 loop 并返回 Err(Interrupted)（WATCH-09）
```

---

## Info

### IN-01: No direct unit test for `write_error_log` with `append_error_log = true`

**File:** `src/cli/run/tests.rs`

**Issue:** `test_write_error_log_run_still_truncates` (line 535) tests the `append_error_log = false` (truncate/overwrite) path. The `append_error_log = true` watch append path is exercised only indirectly through `test_watch_error_log_append` in `src/cli/watch/mod.rs`, which goes through the full `trigger_full_file` stack. A direct unit test calling `write_error_log` with `append_error_log = true` and asserting pre-existing content is preserved would make the contract explicit and catch regressions without requiring the full watch machinery.

**Fix:** Add a mirror test to `src/cli/run/tests.rs`:
```rust
#[test]
fn test_write_error_log_watch_appends() {
    use crate::config::ErrorLogConfig;
    use crate::error::{ErrorKind, ErrorStats, ParseErrorRecord};

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_string_lossy().into_owned();
    std::fs::write(&path, b"EXISTING\n").unwrap();

    let cfg = Config {
        error: Some(ErrorLogConfig { file: path.clone() }),
        append_error_log: true,
        ..Config::default()
    };
    let stats = ErrorStats {
        parse_errors: 1,
        total_errors: 1,
        parse_error_records: vec![ParseErrorRecord {
            line_number: 1,
            raw_truncated: "bad".to_string(),
            kind: ErrorKind::ParseFailed,
        }],
        ..ErrorStats::default()
    };

    write_error_log(&cfg, &stats);

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("EXISTING"), "旧内容应被保留（追加模式）");
    assert!(content.contains("[ERROR] line "), "新错误行应追加到文件末尾");
}
```

### IN-02: `Ordering::Relaxed` store in test threads paired with `Ordering::Acquire` load in production code

**File:** `tests/integration.rs:2939, 2973`

**Issue:** Both `test_watch_triggers_on_new_log_file` and `test_watch_ignores_non_log_files` use `interrupted_clone.store(true, Ordering::Relaxed)` from a spawned thread. The production `run_watch_loop` reads the flag with `Ordering::Acquire`. On x86/x86_64 hardware this is harmless because all stores are at least release-ordered by the memory model, but on weakly-ordered targets (ARM, RISC-V), a `Relaxed` store paired with an `Acquire` load is technically unsound — the reading thread could spin indefinitely.

**Fix:** Use `Ordering::Release` for the stores in both tests:
```rust
// line 2939 and 2973:
interrupted_clone.store(true, Ordering::Release);
```

---

_Reviewed: 2026-06-07_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
