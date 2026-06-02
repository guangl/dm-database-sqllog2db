---
phase: 57-e2e
reviewed: 2026-06-02T00:00:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - src/stats/config.rs
  - tests/integration.rs
findings:
  critical: 1
  warning: 3
  info: 2
  total: 6
status: issues_found
---

# Phase 57: Code Review Report

**Reviewed:** 2026-06-02
**Depth:** standard
**Files Reviewed:** 2
**Status:** issues_found

## Summary

Phase 57 introduces `validate_stats_time_range` — a cross-field `from ≤ to` validation function — and a suite of CLI-level e2e integration tests covering `run` CSV/SQLite output, `init` file creation, and `stats` time-range rejection.

The core logic in `src/stats/config.rs` has one confirmed correctness bug: the lexicographic comparison used for the `from ≤ to` guard is broken when `from` and `to` use different format widths (date vs datetime). The integration tests in `tests/integration.rs` are generally sound but contain one empty test body, a test-label collision, and a hardcoded contract string that will silently diverge if `FIELD_NAMES` changes.

---

## Critical Issues

### CR-01: Mixed-format from/to comparison produces false rejection in `validate_stats_time_range`

**File:** `src/stats/config.rs:38`

**Issue:** The ordering guard uses a plain string comparison:

```rust
if from.as_str() > to.as_str() {
```

The inline comment (line 37) asserts this is legal because `"YYYY-MM-DD 字典序 == 日期序"`, but that equivalence only holds when both strings share the same format width. When `from` is a 19-char datetime and `to` is a 10-char date, the comparison is broken.

Concrete example:
- `from = "2024-01-15 00:00:00"` (19 bytes), `to = "2024-01-15"` (10 bytes)
- Lexicographic comparison: after the first 10 matching bytes, `from` has more bytes, so `"2024-01-15 00:00:00" > "2024-01-15"` → **true**.
- Result: the validator rejects this pair with `"stats.from must be <= stats.to"`.
- Semantically this is a valid range: `from` is the start of the same day named by `to`.

The aggregate filter (`src/stats/aggregate.rs:114–129`) uses prefix-truncation to compare timestamps, which correctly handles mixed-format bounds. The validator must apply the same logic or normalize both sides to the same length before comparing.

**Fix:** Normalize both sides to the same prefix length before comparing:

```rust
if let (Some(from), Some(to)) = (&stats.from, &stats.to) {
    let cmp_len = from.len().min(to.len());
    // Only compare the common prefix so a date "2024-01-15" and datetime
    // "2024-01-15 00:00:00" are treated as equivalent at the day boundary.
    if from.as_bytes()[..cmp_len] > to.as_bytes()[..cmp_len] {
        return Err(Error::Config(ConfigError::InvalidValue {
            field: "stats.from".to_string(),
            value: from.clone(),
            reason: format!("stats.from ({from}) must be <= stats.to ({to})"),
        }));
    }
}
```

Add a unit test to `src/stats/config.rs`:

```rust
#[test]
fn test_validate_stats_time_range_accepts_datetime_from_with_date_to() {
    let cfg = StatsConfig {
        from: Some("2024-01-15 00:00:00".to_string()),
        to:   Some("2024-01-15".to_string()),
        top:  None,
    };
    assert!(
        validate_stats_time_range(&cfg).is_ok(),
        "datetime from at start of to-date should be accepted"
    );
}
```

---

## Warnings

### WR-01: Empty test body `test_handle_run_empty_dir_unix_behavior` provides no coverage

**File:** `tests/integration.rs:68-75`

**Issue:** The test function is registered and counted as passing, but its body is empty — it asserts nothing. It inflates the test count while providing zero actual coverage or contract guarantee. The comment explains the rationale but does not convert into assertions.

**Fix:** Either delete the function entirely, or replace it with a concrete assertion using `make_run_config` to confirm the Unix no-tty / stdin-fallback behavior. If the test is genuinely untestable in CI due to tty interference, use `#[ignore]` with an explanatory comment so it does not falsely represent coverage:

```rust
#[test]
#[cfg(not(target_os = "windows"))]
#[ignore = "stdin tty behavior is non-deterministic in CI; covered indirectly by C3"]
fn test_handle_run_empty_dir_unix_behavior() {}
```

---

### WR-02: TEST-03 label collision between Phase 57 test and existing boundary test section

**File:** `tests/integration.rs:672` and `tests/integration.rs:1942`

**Issue:** The comment block at line 672 declares `// ── Boundary tests (TEST-03) ─────` and the doc-comment at line 1942 declares `/// TEST-03 (Phase 57): stats CLI …`. Both use the identifier `TEST-03`, making it impossible to unambiguously trace a failing test to its design spec. Downstream automation that maps test IDs to plan items will match the wrong entry.

**Fix:** Rename one label. The Phase 57 stats rejection test should use its own phase-scoped ID, e.g. `STATS-12` or `P57-SC5`:

```rust
/// P57-SC5 / STATS-12: stats CLI rejects --from after --to, stderr contains field name and "must be <="
```

---

### WR-03: `validate_time_str` accepts impossible calendar dates (e.g. `"2024-02-31"`)

**File:** `src/stats/config.rs:86-103`

**Issue:** `check_date_part` validates month ∈ `[1, 12]` and day ∈ `[1, 31]` independently. Impossible dates such as `"2024-02-31"`, `"2024-04-31"`, or `"2024-06-31"` pass validation. If a user supplies `from = "2024-02-31"`, the filter silently includes no records (no real timestamp will ever match February 31) instead of returning an actionable error.

The inline comment says "月/日范围校验" without claiming full calendar correctness, but the public documentation says:

```
// 支持两种格式："YYYY-MM-DD" 或 "YYYY-MM-DD HH:MM:SS"
```

A user who writes `2024-02-31` reasonably expects a validation error, not silent data loss.

**Fix:** Add per-month day-limit checks after the existing range check:

```rust
let max_day: u8 = match month {
    2  => 29,  // accept leap-year Feb 29; full leap check is optional
    4 | 6 | 9 | 11 => 30,
    _  => 31,
};
(1..=12).contains(&month) && (1..=max_day).contains(&day)
```

Add unit tests:

```rust
#[test]
fn test_validate_time_str_rejects_feb_31() {
    assert!(validate_time_str("2024-02-31").is_err());
}
#[test]
fn test_validate_time_str_rejects_apr_31() {
    assert!(validate_time_str("2024-04-31").is_err());
}
```

---

## Info

### IN-01: Hardcoded CSV header string in `test_cli_run_csv_output_header_and_row_count` will silently drift

**File:** `tests/integration.rs:2021`

**Issue:** The test asserts an exact CSV header:

```rust
assert_eq!(
    lines.next().unwrap(),
    "ts,ep,sess_id,thrd_id,username,trx_id,statement,appname,client_ip,tag,sql,exec_time_ms,row_count,exec_id,normalized_sql",
    ...
);
```

`FIELD_NAMES` is a public constant in `src/pipeline/mod.rs`. If a field name is ever renamed or the order changes, this test will fail with a confusing string mismatch rather than a clear diff of changed fields.

**Fix:** Derive the expected value from the constant at test time:

```rust
use dm_database_sqllog2db::pipeline::FIELD_NAMES;
let expected_header = FIELD_NAMES.join(",");
assert_eq!(lines.next().unwrap(), expected_header, "CSV header must match FIELD_NAMES order");
```

---

### IN-02: Test name `test_validate_rejects_legacy_sqllog_path_key_via_cli` is misleading

**File:** `tests/integration.rs:1132`

**Issue:** The test name ends in `_via_cli`, but the function calls the Rust library API directly (`Config::from_file`, `cfg.validate()`) — it never invokes the CLI binary. The CLI path is actually tested by `test_cli_legacy_path_key_rejected` at line 1236. The misleading name obscures the boundary being exercised and makes the test map harder to read.

**Fix:** Rename to reflect what it actually tests:

```rust
fn test_validate_rejects_legacy_sqllog_path_key_via_rust_api() { ... }
```

---

_Reviewed: 2026-06-02_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
