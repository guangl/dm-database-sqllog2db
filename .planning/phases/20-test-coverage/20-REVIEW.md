---
phase: 20-test-coverage
reviewed: 2026-05-18T19:30:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - tests/integration.rs
  - src/pipeline/fingerprint.rs
findings:
  critical: 1
  warning: 5
  info: 3
  total: 9
status: issues_found
---

# Phase 20: Code Review Report

**Reviewed:** 2026-05-18T19:30:00Z
**Depth:** standard
**Files Reviewed:** 2
**Status:** issues_found

## Summary

Reviewed `tests/integration.rs` (1686 lines) and `src/pipeline/fingerprint.rs` (459 lines). The integration tests provide broad coverage across CLI handlers, resume, parallel export, E2E pipeline, boundary conditions, and performance baselines. The fingerprint/normalize SQL parser is well-structured with a two-mode byte scanner.

One critical bug was found in `handle_line_comment` — the space-insertion guard present in `handle_block_comment` is absent from the line comment handler, leading to token merging when `--` is at end-of-line. A similar token-merging weakness exists in `handle_quote` for fingerprint mode. On the test side, several tests have weak or absent assertions that could mask regressions.

---

## Critical Issues

### CR-01: handle_line_comment does not insert space after comment removal causing token merging

**File:** `src/pipeline/fingerprint.rs:141-146`
**Issue:** `handle_line_comment` skips the `--` comment entirely but never adds a whitespace separator between the preceding token and whatever follows the newline. When a line comment ends at end-of-line and the next line starts immediately with a keyword (e.g., `SELECT 1--comment\nFROM t`), the output becomes `SELECT 1FROM` instead of `SELECT 1 FROM`.

Compare with `handle_block_comment` (line 156-158) which correctly guards against this:
```rust
if !matches!(out.last(), Some(&b' ')) {
    out.push(b' ');
}
```

`handle_line_comment` has no such guard. This affects both `fingerprint()` and `normalize_template()` because `handle_line_comment` is always called from `dispatch_byte` (inside Normalize-mode dispatch), and the space loss propagates end-to-end.

**Fix:**
```rust
fn handle_line_comment(bytes: &[u8], i: usize, out: &mut Vec<u8>) -> usize {
    if !matches!(out.last(), Some(&b' ')) {
        out.push(b' ');
    }
    match memchr::memchr(b'\n', &bytes[i..]) {
        Some(rel) => i + rel + 1,
        None => bytes.len(),
    }
}
```

Note: the signature must be updated to accept `out: &mut Vec<u8>`, and the call site at line 78 must pass it: `handle_line_comment(bytes, i, out)`.

---

## Warnings

### WR-01: handle_quote in fingerprint mode may merge `?` with next token

**File:** `src/pipeline/fingerprint.rs:115-138`
**Issue:** When a string literal has no trailing whitespace before the next token (e.g., `WHERE name = 'alice'AND id = 1`), `handle_quote` in fingerprint mode replaces the literal with `?` but does not ensure a space follows. Output becomes `WHERE name = ?AND id = ?` instead of `WHERE name = ? AND id = ?`. While rare in real SQL logs, this causes structurally identical queries with/without spacing to produce different fingerprints, violating the fingerprint's aggregation contract.

**Fix:**
```rust
fn handle_quote(bytes: &[u8], i: usize, out: &mut Vec<u8>, keep_literal: bool) -> usize {
    let literal_start = i;
    if !keep_literal {
        out.push(b'?');
    }
    let mut j = i + 1;
    let len = bytes.len();
    loop {
        let Some(rel) = memchr::memchr(b'\'', &bytes[j..]) else {
            j = len;
            break;
        };
        j += rel + 1;
        if j < len && bytes[j] == b'\'' {
            j += 1;
        } else {
            break;
        }
    }
    if keep_literal {
        out.extend_from_slice(&bytes[literal_start..j]);
    } else if j < len && !bytes[j].is_ascii_whitespace() && !matches!(out.last(), Some(&b' ')) {
        out.push(b' ');
    }
    j
}
```

### WR-02: test_handle_run_interrupted discards result with no assertion

**File:** `tests/integration.rs:193-207`
**Issue:** The test sets `interrupted = true` and calls `handle_run`, then does `let _ = result;` — entirely discarding the return value. The comment says "Either Ok (no files processed) or Err(Interrupted) depending on timing," meaning both outcomes pass. The test effectively checks only "doesn't panic," which is too weak for critical interruption logic. A regression that silently returns `Ok(())` without checking the flag would not be caught.

**Fix:** Since the flag is pre-set to true before any processing, the function should observe it early and return early (`Ok(())` is the expected behavior for early exit). Add:
```rust
let result = handle_run(...);
assert!(result.is_ok(), "handle_run should not panic on pre-set interrupt: {result:?}");
```

### WR-03: Many "no panic" tests have no return value assertions

**Files:**
- `tests/integration.rs:337-349` (`test_handle_stats_empty_dir`)
- `tests/integration.rs:367-377` (`test_handle_stats_nonexistent_dir`)
- `tests/integration.rs:404-449` (multiple `handle_stats` group tests)
- `tests/integration.rs:574-643` (multiple `handle_digest` tests)
- `tests/integration.rs:988-992` (`test_handle_show_config_integration`)

**Issue:** Several tests call functions that return `Result` (e.g., `handle_digest`) and discard the result. If these functions start silently returning errors instead of producing expected output, the tests still pass.

**Fix:** Assert `Ok(())` on return values where applicable. For example:
```rust
handle_digest(&cfg, true, None, SortBy::Count, 1, false, None)
    .expect("handle_digest should succeed even with empty dir");
```

For `handle_stats` and `handle_show_config` which have no return value (unit), consider creating a richer test harness, or at minimum document that these are smoke tests.

### WR-04: Imprecise assertions in CSV output tests

**Files:**
- `tests/integration.rs:174` — `assert!(content.lines().count() >= 10)`
- `tests/integration.rs:241` — `assert!(rows_first >= 10, ...)`

**Issue:** These assertions use `>=` instead of exact equality. With 10 records, expected lines = 11 (header + 10 data), and `11 >= 10` passes. But if the actual count drops to 10 (header + 9 data = missing one record), the assertion still passes. This hides off-by-one regressions.

**Fix:**
```rust
assert_eq!(content.lines().count(), 11, "expected header + 10 data rows");
```

### WR-05: Inconsistent compiled_filters passing pattern in filter tests

**File:** `tests/integration.rs:852-983` (lines 852, 915, 951)
**Issue:** `test_handle_run_with_filters_builds_pipeline` (line 869) explicitly calls `cfg.validate_and_compile().unwrap()` and passes the compiled filters to `handle_run`. However, `test_handle_run_with_transaction_filters_prescans` (line 935) and `test_handle_run_with_min_runtime_filter` (line 970) configure the exact same type of filters but pass `None` for `compiled_filters`, relying on `handle_run`'s internal `recompile_meta_if_needed` path. This makes it unclear which filter-compilation path each test exercises and whether the explicit compilation path is actually tested in all filter configurations.

**Fix:** Either consistently pass explicitly compiled filters across all filter tests, or add a comment in each test explaining which compilation path it covers.

---

## Info

### IN-01: CSV field counting uses naive `split(',')` which is fragile for quoted fields

**Files:**
- `tests/integration.rs:1440` — `data_line.split(',').count()`
- `tests/integration.rs:1498` — `line.split(',').count()`

**Issue:** These count CSV fields by splitting on comma. If the SQL text contains a comma inside a quoted CSV field (e.g., after a future change to test SQL or a different SQL parser), the count would overcount. The tests acknowledge this limitation in comments but the fragility remains.

**Fix:** Use the `csv` crate to properly parse CSV lines and count fields, or explicitly document why the current SQL format avoids commas in quoted values.

### IN-02: test_handle_stats_nonexistent_dir uses Unix-specific path

**File:** `tests/integration.rs:373`
**Issue:** The path `/no/such/directory/at/all` is Unix-specific and will fail on Windows.

**Fix:** For cross-platform tests, detect OS or use a path that is invalid on all platforms.

### IN-03: test_handle_init_creates_config_file uses weak assertion

**File:** `tests/integration.rs:672-674`
**Issue:** `assert!(!content.is_empty())` — an empty file would pass this check. The test should verify actual config content.

**Fix:**
```rust
let content = std::fs::read_to_string(&config_path).unwrap();
assert!(content.contains("[sqllog]"), "init template should contain [sqllog] section");
```

---

_Reviewed: 2026-05-18T19:30:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
