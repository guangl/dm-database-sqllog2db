---
phase: 02-fsevents
plan: 02
subsystem: testing
tags: [coverage, collector, unit-test, filter-processor, llvm-cov, rust]

# Dependency graph
requires:
  - phase: 02-fsevents
    plan: 01
    provides: WATCH-07/08/09 integration tests (indirect csv/mod.rs coverage boost)

provides:
  - collector.rs Group 1+2+3+4 unit tests in src/cli/run/tests.rs
  - filter_processor.rs sessions/apps/statements/threads/debug tests (D-05 remediation)
  - TOTAL Line coverage: 92.06% (target: >=92.00%)

affects: [02-fsevents]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "collector unit test pattern: pub(super) fn via super::collector::collect_log_file in tests submodule"
    - "AlwaysFail pipeline processor: inline #[derive(Debug)] struct at module scope for sharing between tests"
    - "D-05 filter coverage: session/app/statement/thread field filters with make_record defaults"

key-files:
  created: []
  modified:
    - src/cli/run/tests.rs
    - src/cli/run/filter_processor.rs

key-decisions:
  - "collect_log_file returns (Vec<(Sqllog, Option<String>)>, ErrorStats) — Group 2 uses stats.parse_errors field"
  - "AlwaysFail defined at module scope in tests.rs to share between Group 3 + Group 4"
  - "D-05 remediation added 5 tests to filter_processor.rs covering sessions/apps/statements/threads/Debug"
  - "Final coverage 92.06% via llvm-cov --summary-only (Task 2 auto-verified, AUTO_MODE=true)"

patterns-established:
  - "Coverage gap remediation: add targeted field-specific tests when primary target falls short of threshold"

requirements-completed: [QUAL-02, QUAL-03]

# Metrics
duration: ~35min (including merge conflict resolution + D-05 remediation)
completed: 2026-06-07
---

# Phase 02 Plan 02: collector.rs Unit Tests + Coverage Checkpoint Summary

**9 new tests total: 4 collector unit tests (Group 1-4) + 5 filter_processor tests (D-05); final coverage 92.06% ≥ 92.00%**

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-06-07
- **Tasks:** 2 (Task 1: code; Task 2: checkpoint auto-verified)
- **Files modified:** 2

## Accomplishments

- Added `test_collector_invalid_path_returns_error` (Group 1): nonexistent path → `Err(ParserError::InvalidPath { .. })`
- Added `test_collector_parse_error_accumulation` (Group 2): invalid-line file → `stats.parse_errors > 0`, rows empty
- Added `test_collector_not_needed_filtering` (Group 3): AlwaysFail + DML + `do_normalize=false` → `rows.is_empty()` (hits `!needs_processing` early return)
- Added `test_collector_filtered_params_normalize` (Group 4): AlwaysFail + PARAMS + `do_normalize=true` → `rows.is_empty()` (hits `compute_normalized` branch)
- Task 2 checkpoint: `cargo llvm-cov --summary-only` TOTAL Line % = **91.82%** (below 92%)
- D-05 remediation: added 5 tests to `filter_processor.rs` (sessions, apps, statements, threads, Debug fmt)
- Final coverage: **92.06%** — QUAL-02 success criteria met

## Task Commits

1. **Task 1 (worktree):** `8c78077` — `test(02-fsevents/02): add collector Group 1+2+3+4 unit tests`
2. **Task 1 (merge resolution):** `db239c4` — `test(02-fsevents/02): add collector Group 1+2+3+4 unit tests (merge conflict resolved)`
3. **D-05 remediation:** `1440bf1` — `test(02-fsevents/02): add D-05 filter_processor coverage tests`

## Files Created/Modified

- `src/cli/run/tests.rs` — 4 collector unit tests + AlwaysFail module-scope struct
- `src/cli/run/filter_processor.rs` — 5 D-05 remediation tests (sessions/apps/statements/threads/Debug)

## Decisions Made

- Worktree executor reported `collect_log_file` returns `(Vec, usize)` — actual return is `(Vec, ErrorStats)`. Fixed Group 2 assertion to use `stats.parse_errors` field.
- Merge conflict in `tests.rs` between Wave 1 progress-bar tests (HEAD) and Wave 2 collector tests (worktree); resolved manually by keeping both sets.
- Initial coverage after Tasks 1+2 = 91.82% (gap: 0.18pp); D-05 remediation chose `filter_processor.rs` over `sqlite/mod.rs` (lower barrier, targeted field closures).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Merge conflict in src/cli/run/tests.rs**
- **Found during:** Worktree merge (Wave 2)
- **Issue:** HEAD had Phase 01 progress-bar + error-log tests; worktree had collector tests; both started from same pre-Phase-01 base
- **Fix:** Manual conflict resolution preserving both sets
- **Verification:** `cargo test --lib -- test_collector_` → 4 passed; `cargo clippy` → 0 warnings

**2. [Rule 3 - Deviation] collect_log_file return type mismatch**
- **Found during:** Post-merge compilation
- **Issue:** Executor noted return as `(Vec, usize)` but actual is `(Vec, ErrorStats)`; Group 2 assert needed `stats.parse_errors` not `parse_errors > 0`
- **Fix:** Changed destructured variable from `parse_errors` to `stats`, used `stats.parse_errors > 0`

**3. [Rule 3 - Deviation] Initial coverage 91.82% < 92.00%**
- **Found during:** Task 2 checkpoint auto-verification
- **Issue:** collector.rs Group 1-4 (~35 lines) + Wave 1 csv/mod.rs boost = 91.82%, not 92.00%
- **Fix:** D-05 remediation (5 filter_processor tests covering sessions/apps/statements/threads/Debug)
- **Result:** 92.06% — threshold met

## Issues Encountered

- Worktree executor downgraded rusqlite 0.40.0 → 0.39.0 in Cargo.toml (to fix libsqlite3-sys nightly-only macro issue). This change was carried through the merge.

## User Setup Required

None.

## Next Phase Readiness

- QUAL-02 (coverage ≥ 92%) met: 92.06%
- QUAL-03 (FSEvents `#[ignore]` decision documented): D-01/D-02 in CONTEXT.md, annotation preserved
- All 389 tests passing (including 7 watch_incremental + 4 new collector + 5 new filter_processor)

## Self-Check: PASSED

- `cargo test --lib -- test_collector_`: 4 passed
- `cargo llvm-cov --summary-only` TOTAL Line % = 92.06% ≥ 92.00%
- `cargo test`: full suite passes
- `cargo clippy --all-targets -- -D warnings`: 0 warnings
- `cargo fmt --check`: passed
- `tests/integration.rs:2917` `#[ignore]` preserved (QUAL-03 D-01)

---
*Phase: 02-fsevents*
*Completed: 2026-06-07*
