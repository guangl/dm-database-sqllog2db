---
phase: 53-cli
reviewed: 2026-06-01T00:00:00Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - src/cli/init.rs
  - src/cli/opts.rs
  - src/cli/stats/mod.rs
  - src/config/mod.rs
  - src/config/validate.rs
  - src/main.rs
  - src/stats/config.rs
  - src/stats/mod.rs
  - tests/integration.rs
findings:
  critical: 0
  warning: 2
  info: 3
  total: 5
status: issues_found
---

# Phase 53: Code Review Report

**Reviewed:** 2026-06-01T00:00:00Z
**Depth:** standard
**Files Reviewed:** 9
**Status:** issues_found

## Summary

Phase 53 adds the config/parameter layer for `stats --from`/`--to`/`--top`: a new `StatsConfig` struct, TOML `[stats]` section deserialization, CLI argument wiring, priority merging (CLI > config > default), format validation at both the `validate` subcommand and the `run_stats` entry point, and init template updates. Actual time-range filtering of accumulator records is explicitly deferred to Phase 54 per the `53-CONTEXT.md` decision log.

The implementation is structurally sound and the priority-merge logic is correct. Two warnings are raised: the time-string validator accepts semantically impossible dates (e.g. month 99), and the CLI help text describes `--from`/`--to` as "filtering" before the feature is live. Three info items cover missing range checks on private helper indexing, duplicated validation logic, and a test coverage gap that would not catch the absence of actual time-range filtering.

No critical issues found.

## Warnings

### WR-01: `validate_time_str` accepts impossible calendar and clock values

**File:** `src/stats/config.rs:51-75`

**Issue:** `check_date_part` and `check_time_part` verify only separator positions and that every expected position holds an ASCII digit. Numeric range constraints are not checked. The following strings all pass validation today:

```
"2024-99-99"          // month 99, day 99
"2024-00-00"          // month 0, day 0
"2024-01-01 25:61:99" // hour 25, minute 61, second 99
```

When Phase 54 wires these values into record comparisons, a user who typos a date will receive silently wrong filter results. The problem is present now and will become data-correctness silent-failure in the next phase.

**Fix:** Extend both check functions with per-component range validation:

```rust
fn check_date_part(bytes: &[u8]) -> bool {
    if !(bytes[4] == b'-' && bytes[7] == b'-') { return false; }
    if !bytes[..4].iter().all(|b| b.is_ascii_digit()) { return false; }
    if !bytes[5..7].iter().all(|b| b.is_ascii_digit()) { return false; }
    if !bytes[8..10].iter().all(|b| b.is_ascii_digit()) { return false; }
    let month = (bytes[5] - b'0') * 10 + (bytes[6] - b'0');
    let day   = (bytes[8] - b'0') * 10 + (bytes[9] - b'0');
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

fn check_time_part(bytes: &[u8]) -> bool {
    if !(bytes[10] == b' ' && bytes[13] == b':' && bytes[16] == b':') { return false; }
    if !bytes[11..13].iter().chain(bytes[14..16].iter())
        .chain(bytes[17..19].iter()).all(|b| b.is_ascii_digit()) { return false; }
    let hour = (bytes[11] - b'0') * 10 + (bytes[12] - b'0');
    let min  = (bytes[14] - b'0') * 10 + (bytes[15] - b'0');
    let sec  = (bytes[17] - b'0') * 10 + (bytes[18] - b'0');
    hour <= 23 && min <= 59 && sec <= 59
}
```

---

### WR-02: CLI help text describes `--from`/`--to` as record filtering before the feature is active

**File:** `src/cli/opts.rs:131`, `src/cli/opts.rs:151-162`; `src/cli/init.rs:125-128`

**Issue:** The `stats` subcommand's `after_help` example reads `"Filter records by time range:"` and the argument docs say `"Start of time range"` / `"End of time range"`. The config template comment says `"Start of time range"` with no qualifier. At this phase the values are stored and validated but `StatsAccumulator::update` never consults them — every record is always included. A user running `stats --from 2024-01-01 --to 2024-01-31` today receives output spanning all dates with no error or warning.

**Fix (until Phase 54 ships):** Emit a `log::warn!` when from/to are set, making the partial state explicit:

```rust
// In src/cli/stats/mod.rs, handle_stats, after merge:
if merged_cfg.stats.from.is_some() || merged_cfg.stats.to.is_some() {
    log::warn!(
        "stats: --from/--to time-range filtering is not yet active; \
         all records are included regardless of timestamp"
    );
}
```

Alternatively, revise the help text to `"Reserved for time-range filtering (not yet active)"` until Phase 54 is complete.

---

## Info

### IN-01: Private index helper functions lack `debug_assert` documenting their length precondition

**File:** `src/stats/config.rs:51-75`

**Issue:** `check_date_part` indexes `bytes[0..9]` directly; `check_time_part` indexes `bytes[10..18]`. Both functions accept a bare `&[u8]` slice with no bounds guarantee in their signature. The callers are in a `match bytes.len()` guard (10 or 19), so no out-of-bounds access is possible today. However, the implicit precondition is not documented in the function contract or enforced by an assertion, making the code fragile to future refactoring.

**Fix:** Add `debug_assert` preconditions:

```rust
fn check_date_part(bytes: &[u8]) -> bool {
    debug_assert!(bytes.len() >= 10, "check_date_part: need at least 10 bytes");
    // ...
}
fn check_time_part(bytes: &[u8]) -> bool {
    debug_assert!(bytes.len() >= 19, "check_time_part: need at least 19 bytes");
    // ...
}
```

---

### IN-02: Identical validation logic duplicated across two call sites

**File:** `src/config/validate.rs:15-34` and `src/stats/mod.rs:19-42`

**Issue:** `Config::validate_stats_time_fields` and `validate_cfg_stats_time` in `run_stats` are character-for-character identical: both check `cfg.stats.from` and `cfg.stats.to` against `validate_time_str` and produce the same `ConfigError::InvalidValue` variants. The duplication is intentional per D-09 ("defensive check so CLI values are also caught"), but it means that adding range checks from WR-01 must be done in two places. If one site is updated and the other is not, validation becomes inconsistent.

**Fix:** Extract a single shared function in `src/stats/config.rs` and call it from both sites:

```rust
/// Validates stats.from and stats.to format; used by Config::validate and run_stats.
pub fn validate_stats_time_range(stats: &StatsConfig) -> crate::error::Result<()> {
    if let Some(from) = &stats.from {
        validate_time_str(from).map_err(|reason| /* ConfigError::InvalidValue ... */)?;
    }
    if let Some(to) = &stats.to {
        validate_time_str(to).map_err(|reason| /* ConfigError::InvalidValue ... */)?;
    }
    Ok(())
}
```

---

### IN-03: Integration tests do not verify that `from`/`to` actually filters records

**File:** `tests/integration.rs:1720-1833`

**Issue:** The Phase 53 integration tests (SC#1–SC#4) verify format acceptance/rejection, that values appear in the application log, and that CLI values override config values. No test asserts that records outside the `[from, to]` range are absent from the stats output. Because the filter is not yet implemented, such a test would currently fail — but its absence means the gap will be invisible when Phase 54 adds filtering, unless a test is written then.

**Fix:** When Phase 54 ships, add a test that writes log lines across multiple dates and checks that the stats output contains only lines within the specified range:

```rust
#[test]
fn test_stats_from_to_excludes_out_of_range_records() {
    // Write records at 2023-01-01 and 2025-01-01
    // Run stats --from 2025-01-01 --to 2025-12-31
    // Assert 2023 record is absent from slow_sql.csv / frequent_sql.csv
}
```

---

_Reviewed: 2026-06-01T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
