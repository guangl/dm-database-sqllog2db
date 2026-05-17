---
phase: 17-filter-nesting
reviewed: 2026-05-17T00:00:00Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - src/cli/init.rs
  - src/cli/run.rs
  - src/cli/show_config.rs
  - src/cli/stats.rs
  - src/cli/validate.rs
  - src/config.rs
  - src/features/filters.rs
  - src/main.rs
  - tests/integration.rs
findings:
  critical: 1
  warning: 3
  info: 3
  total: 7
status: issues_found
---

# Phase 17: Code Review Report

**Reviewed:** 2026-05-17
**Depth:** standard
**Files Reviewed:** 9
**Status:** issues_found

## Summary

Phase 17 refactored the filter config from a flat `MetaFilters` struct to nested `IncludeFilters`/`ExcludeFilters` sub-tables, implemented via a hand-written `Deserialize` impl and a `RawFiltersFeature` intermediate. The deserialization logic, `From<>` conversion, and caller updates are structurally sound. All 904 tests pass, and `cargo clippy --all-targets -- -D warnings` produces no diagnostics.

The main critical finding is a **silent data-loss** scenario in the `From<RawFiltersFeature>` conversion: when a user's TOML file contains both a `[features.filters.include]` sub-table *and* old flat-format keys (e.g. `usernames = [...]`) in the same `[features.filters]` section, the old flat fields are silently discarded. No warning is emitted and no test guards this scenario. The remaining findings are a misnamed/lossy rate calculation in `stats.rs`, incomplete diagnostic display in `show_config.rs` and `validate.rs` after the nesting refactor, and a per-file regex recompilation in `stats.rs` that also silently ignores compilation errors.

---

## Critical Issues

### CR-01: Silent data-loss when old flat fields coexist with new nested sub-table

**File:** `src/features/filters.rs:195-215`

**Issue:** `From<RawFiltersFeature>::from()` uses `raw.include.unwrap_or(IncludeFilters { users: raw.usernames, … })`. When `raw.include` is `Some` (because the TOML file contains a `[features.filters.include]` sub-table), all old-format flat fields that were also deserialized into `RawFiltersFeature` (`raw.usernames`, `raw.client_ips`, `raw.statements`, `raw.tags`, `raw.trxids`, etc.) are silently dropped without warning or error. A user who is partially migrating—or who mistakenly has both formats active—will lose filter rules with no indication.

The same applies symmetrically to the `exclude` side: if `[features.filters.exclude]` is present AND `exclude_usernames = [...]` is also in the `[features.filters]` scope, the flat field is silently dropped.

TOML permits both patterns to coexist (they are different key paths), so this is a reachable and realistic failure mode.

**Fix:** Add a warning (or return a deserialization error) when both the new sub-table and old flat fields are detected simultaneously:

```rust
impl From<RawFiltersFeature> for FiltersFeature {
    fn from(raw: RawFiltersFeature) -> Self {
        let flat_include_present = raw.usernames.is_some()
            || raw.client_ips.is_some()
            || raw.sess_ids.is_some()
            || raw.thrd_ids.is_some()
            || raw.statements.is_some()
            || raw.appnames.is_some()
            || raw.tags.is_some()
            || raw.start_ts.is_some()
            || raw.end_ts.is_some()
            || raw.trxids.is_some();

        if raw.include.is_some() && flat_include_present {
            log::warn!(
                "[features.filters] contains both a `[features.filters.include]` sub-table \
                 and legacy flat fields (e.g. `usernames`). The sub-table takes priority; \
                 flat fields are ignored. Remove the legacy keys to suppress this warning."
            );
        }
        // … rest of From impl unchanged
    }
}
```

Apply the same guard for `exclude`. Also add a test that verifies the warning fires:

```rust
#[test]
fn test_mixed_format_new_format_wins_and_warns() {
    let toml = r#"
[sqllog]
path = "sqllogs"
[features.filters]
enable = true
usernames = ["old_user"]   # legacy flat field
[features.filters.include]
users = ["new_user"]       # new sub-table
[exporter.csv]
file = "out.csv"
"#;
    let cfg: Config = toml::from_str(toml).unwrap();
    let f = cfg.features.filters.unwrap();
    // new format wins
    assert_eq!(f.include.users, Some(vec!["new_user".to_string()]));
    // old flat field was dropped (not merged)
    // NOTE: after fix, this test also asserts a warning was emitted
}
```

---

## Warnings

### WR-01: `stats.rs` rate calculation uses integer division, overstating throughput for sub-second runs

**File:** `src/cli/stats.rs:348`

**Issue:** `let rate = total_records / elapsed.as_secs().max(1);` uses `u64` integer division. `elapsed.as_secs()` truncates to whole seconds, so any run completing in under 1 second is forced to divide by 1 instead of the actual elapsed time. For 5 000 records in 0.5 seconds the code reports 5 000 rec/s instead of the correct 10 000 rec/s — a 50% understatement. The same truncation overstates the rate for runs that take e.g. 1.9 seconds (divides by 1, not 1.9). The `elapsed_secs` field (correct `f64`) is already computed on line 347 and used elsewhere in the same function.

**Fix:**

```rust
// line 347-348 in stats.rs
let elapsed_secs = elapsed.as_secs_f64();
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
let rate = if elapsed_secs > 0.0 {
    (total_records as f64 / elapsed_secs) as u64
} else {
    0
};
```

### WR-02: `show_config.rs` silently omits most filter fields after nesting refactor

**File:** `src/cli/show_config.rs:100-125`

**Issue:** The `handle_show_config` filter display block only shows `enable`, `include.start_ts`, `include.end_ts`, `include.trxids`, `include.users`, and `include.ips`. All of the following are silently absent:

- `include.sessions`, `include.threads`, `include.statements`, `include.apps`, `include.tags`
- The entire `exclude` sub-table (all 7 fields)
- `indicators` (exec\_ids, min\_runtime\_ms, min\_row\_count)
- `sql` (includes, excludes)
- `record_sql` (includes, excludes)

A user running `show-config` after Phase 17 gets a misleading partial view that omits configured exclude filters entirely. The `validate.rs` diagnostic has the same gap (see WR-03).

**Fix:** Add display blocks for all nested filter fields, following the same `kv(...)` pattern already used in the file:

```rust
// add after the existing include.ips block
if let Some(sess) = &f.include.sessions {
    kv("include.sessions", &sess.join(", "), None, diff);
}
// … threads, statements, apps, tags …

// Add exclude sub-section
if f.exclude.has_filters() {
    if let Some(u) = &f.exclude.users {
        kv("exclude.users", &u.join(", "), None, diff);
    }
    // … ips, sessions, threads, statements, apps, tags …
}

// Add indicators
if f.indicators.has_filters() {
    if let Some(ids) = &f.indicators.exec_ids {
        kv("indicators.exec_ids", &format!("{} entries", ids.len()), None, diff);
    }
    // … min_runtime_ms, min_row_count …
}
```

### WR-03: `stats.rs` silently ignores regex compilation errors in filter path

**File:** `src/cli/stats.rs:465-466`

**Issue:** Filters are recompiled from scratch for every file via `process_file()`. The compilation result is silently discarded on error:

```rust
let compiled_meta: Option<CompiledMetaFilters> = filter_cfg
    .and_then(|f| CompiledMetaFilters::try_from_include_exclude(&f.include, &f.exclude).ok());
```

If a user passes an invalid regex pattern to `stats` (e.g. via `--set features.filters.include.users=[bad`), the `.ok()` call turns the `Err` into `None` and filtering is silently disabled. The stats output will then include all records with no indication that filtering was skipped.

Additionally, this compiles the same regex patterns N times (once per file) rather than once before the loop. This is a redundant allocation inside the hot path.

**Fix:** Compile filters once before the per-file loop, check and propagate (or at minimum surface) errors:

```rust
// In handle_stats, before the for loop:
let compiled_meta: Option<CompiledMetaFilters> = if let Some(filter_cfg) = cfg.features.filters.as_ref().filter(|f| f.has_filters()) {
    match CompiledMetaFilters::try_from_include_exclude(&filter_cfg.include, &filter_cfg.exclude) {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("{} Filter regex error: {e}", color::red("Error:"));
            return; // or propagate
        }
    }
} else {
    None
};
```

Then pass `compiled_meta` into `ProcessFileCtx` and remove the per-file recompilation in `process_file()`.

---

## Info

### IN-01: `validate.rs` omits most filter fields from diagnostic output

**File:** `src/cli/validate.rs:27-57`

**Issue:** `handle_validate` displays `include.start_ts`, `include.end_ts`, `include.trxids`, `include.users`, `include.ips`, indicator fields, and `sql` — but omits `include.sessions`, `.threads`, `.statements`, `.apps`, `.tags`, all `exclude.*` fields, and `record_sql`. This is the same incompleteness as WR-02 but in the `validate` command. Unlike `show_config`, `validate` is read-only diagnostic output, so the impact is lower (no functional regression).

**Fix:** Add log lines for the missing fields following the existing pattern in `validate.rs`.

### IN-02: Missing test for mixed-format silent drop (no negative assertion)

**File:** `src/features/filters.rs` (test module)

**Issue:** `test_backward_compat_flat_format` (line 1348) verifies old flat format. `test_new_nested_format_include` (line 1293) verifies new format. But there is no test that simultaneously provides both `[features.filters.include]` sub-table AND legacy flat fields (e.g. `usernames = [...]`) in the same TOML and asserts which fields survive. This leaves the silent-drop behavior in CR-01 completely untested.

**Fix:** Add a test:
```rust
#[test]
fn test_new_format_wins_over_flat_when_both_present() {
    let toml = r#"
[sqllog]
path = "sqllogs"
[features.filters]
enable = true
usernames = ["old_user"]
[features.filters.include]
users = ["new_user"]
[exporter.csv]
file = "out.csv"
"#;
    let cfg: Config = toml::from_str(toml).unwrap();
    let f = cfg.features.filters.unwrap();
    assert_eq!(f.include.users, Some(vec!["new_user".to_string()]),
        "new nested format should win; old flat field should be dropped");
    // Also assert old_user is NOT in the result
    assert!(!f.include.users.as_ref().unwrap().contains(&"old_user".to_string()),
        "legacy flat field must not be merged into new format result");
}
```

### IN-03: `backward_compat_flat_format` test asserts only `users` and `ips`, skips 5 other mapped fields

**File:** `src/features/filters.rs:1375-1384`

**Issue:** `test_backward_compat_flat_format` verifies that `include.users`, `include.ips`, and `include.trxids` are correctly mapped from old flat fields, and that `exclude.users` and `exclude.ips` are mapped. It does not assert `include.sessions`, `include.threads`, `include.statements`, `include.apps`, `include.tags`, `include.start_ts`, `include.end_ts`, or the corresponding exclude fields. A regression in any of those 10 unmapped fields would not be caught by this test.

**Fix:** Extend the test with assertions for all fields:
```rust
assert_eq!(filters.include.sessions, Some(vec!["s001".to_string()]));
assert_eq!(filters.include.threads, Some(vec!["t001".to_string()]));
assert_eq!(filters.include.statements, Some(vec!["SELECT".to_string()]));
assert_eq!(filters.include.apps, Some(vec!["myapp".to_string()]));
assert_eq!(filters.include.tags, Some(vec!["audit".to_string()]));
assert_eq!(filters.include.start_ts.as_deref(), Some("2024-01-01T00:00:00"));
assert_eq!(filters.include.end_ts.as_deref(), Some("2024-12-31T23:59:59"));
assert_eq!(filters.exclude.sessions, Some(vec!["s999".to_string()]));
assert_eq!(filters.exclude.threads, Some(vec!["t999".to_string()]));
assert_eq!(filters.exclude.statements, Some(vec!["DROP".to_string()]));
assert_eq!(filters.exclude.apps, Some(vec!["badapp".to_string()]));
assert_eq!(filters.exclude.tags, Some(vec!["sys".to_string()]));
```

---

_Reviewed: 2026-05-17_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
