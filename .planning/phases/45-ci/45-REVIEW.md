---
phase: "45"
reviewed: 2026-05-25T00:00:00Z
depth: standard
files_reviewed: 7
files_reviewed_list:
  - src/cli/run/sqlite_parallel.rs
  - src/cli/run/mod.rs
  - src/exporter/sqlite/mod.rs
  - src/exporter/mod.rs
  - src/cli/run/tests.rs
  - .github/workflows/bench.yml
  - scripts/collect_bench_results.sh
findings:
  critical: 0
  warning: 2
  info: 1
  total: 3
status: has_findings
---

# Phase 45: Code Review Report

**Reviewed:** 2026-05-25
**Depth:** standard
**Files Reviewed:** 7
**Status:** has_findings

## Summary

Reviewed the SQLite parallel parse path implementation (`sqlite_parallel.rs`), the main
orchestrator (`cli/run/mod.rs`), the SQLite exporter (`exporter/sqlite/mod.rs`), the
exporter manager (`exporter/mod.rs`), integration tests, and CI configuration files.

The core logic is sound: the WAL-mode setup sequence is correct (COMMIT -> locking_mode=NORMAL ->
journal_mode=WAL -> BEGIN), PARAMS records receive the same treatment in parallel and sequential
paths, `field_mask`/`ordered_indices` are correctly propagated via `cfg` even though they are not
forwarded through the parameter list, and the `jq -s ... add // {}` idiom in the bench script
handles the empty-results case properly.

Two warning-level defects were found: silent parse-error discard in the SQLite parallel path, and
a test that does not exercise what its comment claims.

---

## Warnings

### WR-01: Parse errors silently dropped in SQLite parallel collect path

**File:** `src/cli/run/sqlite_parallel.rs:42`

**Issue:** `collect_log_file` discards every parse error via `let Ok(record) = result else { continue }` with no logging and no stats increment. In the sequential path (`processor.rs:146-150`), the same error triggers `file_stats.add_parse_error()` and a `log::warn!` call, so the returned `ErrorStats` and the application log both reflect malformed lines. In the CSV parallel path, per-file work is delegated to `process_log_file` which retains that behaviour. Only the SQLite parallel path is silent. Users who depend on the log or on `ErrorStats::parse_errors > 0` to detect corrupt log files will miss all parse failures when running with `--jobs > 1` against an SQLite destination.

**Fix:**
```rust
// collect_log_file: replace the bare `continue` with a warn + counter
let mut parse_errors: usize = 0;

for result in parser.iter() {
    if interrupted.load(Ordering::Relaxed) {
        break;
    }
    let record = match result {
        Ok(r) => r,
        Err(e) => {
            parse_errors += 1;
            log::warn!("{} | parse error: {e:?}", file.display());
            continue;
        }
    };
    process_record(...);
}
// return (rows, parse_errors) and surface the count in the caller
```

Alternatively, reuse `process_log_file` (which already handles errors) the same way the CSV
parallel path does — that would eliminate the divergence entirely.

---

### WR-02: `test_parallel_merge_consistent` does not test sequential mode

**File:** `src/cli/run/tests.rs:141`

**Issue:** The comment on line 141 reads _"Force sequential by using a single-file config, so
jobs never matters"_. However `make_cfg` is called with `dir.path()`, which contains both
`a.log` and `b.log`. `handle_run` counts files in that directory and, on any machine where
`available_parallelism() > 1` (all modern CI hosts), sets `use_csv_parallel = true` for
**both** `cfg_seq` and `cfg_par`. The assertion on line 157 therefore compares two parallel
runs, not a sequential run against a parallel one. A genuine regression in the sequential
path would go undetected by this test.

**Fix:** To actually force the sequential path, either (a) write a single log file into a
dedicated `seq_dir` directory as `test_sqlite_parallel_matches_sequential` already does, or
(b) mock `jobs = 1` somehow. Option (a):
```rust
// sequential: single file forces log_files.len() == 1 -> parallel never triggered
let seq_dir = dir.path().join("seq");
std::fs::create_dir(&seq_dir).unwrap();
std::fs::write(seq_dir.join("only.log"), log_line).unwrap();
let cfg_seq = make_cfg_dir(&seq_dir, &csv_seq);
```

---

## Info

### IN-01: Three parameters accepted but unconditionally discarded in `process_sqlite_parallel`

**File:** `src/cli/run/sqlite_parallel.rs:196`

**Issue:** `show_progress`, `field_mask`, and `ordered_indices` are formal parameters of
`process_sqlite_parallel` but are immediately suppressed with `let _ = (show_progress,
field_mask, ordered_indices)`. The comment acknowledges this ("保留参数以维持与
`process_csv_parallel` 调用方对称"). The `field_mask` and `ordered_indices` values are
redundantly re-derived from `cfg` inside `ExporterManager::from_config`. Progress is
simply not shown for the SQLite parallel path, which differs silently from the CSV
parallel behaviour.

**Fix:** At minimum, add a `#[allow(unused_variables)]`-style suppression or a named
constant comment that links to the tracking issue for future work. Longer-term, connect
`show_progress` to a spinner update in `merge_and_write` for consistency.

---

_Reviewed: 2026-05-25_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
