---
phase: 63-test-coverage
plan: 01
subsystem: testing
tags: [rust, cargo-llvm-cov, serde, toml, pipeline, filters, coverage]

requires:
  - phase: 62-docs
    provides: stable codebase with pipeline/filters/types.rs and serde_helpers.rs

provides:
  - baseline coverage report (target/llvm-cov/html/ + baseline-summary.txt)
  - 19 new tests in src/pipeline/filters/types.rs covering serde_helpers + FiltersFeature::from + has_filters branches
affects:
  - 63-02 (csv/sqlite exporter coverage)
  - 63-03 (error.rs + prescan.rs coverage)
  - 63-04 (wave 2 verification)

tech-stack:
  added: []
  patterns:
    - "FilterWrapper pattern: wrap FiltersFeature in a struct to trigger toml::from_str deserialization chain indirectly"
    - "Inline TOML string: use r#\"...\"# or plain string literals to drive serde_helpers::vec_to_hashset/vec_to_i64_hashset without calling pub(super) functions directly"

key-files:
  created: []
  modified:
    - src/pipeline/filters/types.rs

key-decisions:
  - "FilterWrapper struct used to bypass FiltersFeature custom Deserialize wrapping requirement (per RESEARCH.md Pitfall 2)"
  - "Inline string literals (not r#\"...\"# with hashes) used where TOML has no double quotes, per clippy needless_raw_string_hashes lint"
  - "Task 1 (baseline report) has no git commit — target/ is gitignored; HTML report and baseline-summary.txt exist on disk only"

patterns-established:
  - "Pattern: indirect serde_helpers testing via FilterWrapper + toml::from_str in types.rs mod tests"
  - "Pattern: legacy flat field coverage via TOML with no [filter.include] sub-table"

requirements-completed: [TEST-01, TEST-02]

duration: 25min
completed: 2026-06-03
---

# Phase 63 Plan 01: Test Coverage Baseline Summary

**Baseline coverage report generated (90.68% line / 85.81% function) and 19 new tests added to pipeline/filters/types.rs covering serde_helpers vec_to_hashset/vec_to_i64_hashset, FiltersFeature::from legacy/mixed-format paths, and IncludeFilters/ExcludeFilters has_filters branches**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-06-03
- **Completed:** 2026-06-03
- **Tasks:** 2
- **Files modified:** 1 (src/pipeline/filters/types.rs)

## Accomplishments

- Baseline coverage report generated: `target/llvm-cov/html/index.html` + `target/llvm-cov/baseline-summary.txt`
- 19 new test functions added to `src/pipeline/filters/types.rs` (mod tests block at file end)
- All 3 quality gates pass: `cargo test` (288 tests pass) + `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check`
- serde_helpers.rs now covered via indirect path (FilterWrapper → toml::from_str triggers vec_to_hashset / vec_to_i64_hashset)
- FiltersFeature::from legacy flat field mapping fully tested (9 include fields + 7 exclude fields)
- Mixed-format priority branches covered for both include and exclude
- IncludeFilters/ExcludeFilters has_filters() branches covered for ips/sessions/threads/apps/tags (5 fields each)

## Baseline Coverage Numbers (for Wave 2 comparison)

**TOTAL: 90.68% line / 85.81% function**

### Files with line coverage < 80% (baseline):

| File | Line % | Function % | Priority |
|------|--------|-----------|---------|
| `pipeline/filters/serde_helpers.rs` | 0.00% | 0.00% | P1 — zero coverage |
| `exporter/csv/writer.rs` | 66.29% | 75.00% | P1 — critical hot path |
| `cli/run/processor.rs` | 68.75% | 66.67% | P2 |
| `cli/init.rs` | 68.33% | 50.00% | P2 |
| `cli/run/prescan.rs` | 70.79% | 64.29% | P2 |
| `pipeline/filters/types.rs` | 82.21% | 31.58% | P1 — function coverage very low |
| `exporter/sqlite/mod.rs` | 78.26% | 53.33% | P1 — function coverage low |

## New Test Functions (19 total)

### serde_helpers::vec_to_hashset coverage (Tests 1-2)
1. `test_trxids_deserialized_to_hashset` — Some branch: trxids=["TX001","TX002"] → HashSet len 2
2. `test_trxids_absent_returns_none` — None branch: no trxids field → `include.trxids` is None

### serde_helpers::vec_to_i64_hashset coverage (Tests 3-4)
3. `test_exec_ids_deserialized_to_hashset` — Some branch: exec_ids=[1,2,42] → HashSet len 3
4. `test_exec_ids_absent_returns_none` — None branch: no exec_ids field → `indicators.exec_ids` is None

### FiltersFeature::from legacy field mapping (Tests 5-7)
5. `test_legacy_flat_usernames_mapped_to_include_users` — usernames → include.users
6. `test_legacy_flat_exclude_usernames_mapped` — exclude_usernames → exclude.users
7. `test_legacy_flat_all_include_fields_mapped` — client_ips/sess_ids/thrd_ids/statements/appnames/tags/start_ts/end_ts

### FiltersFeature::from mixed-format priority (Tests 8-9)
8. `test_mixed_format_new_table_takes_priority` — [filter.include] sub-table wins over flat usernames
9. `test_mixed_format_exclude_new_table_priority` — [filter.exclude] sub-table wins over flat exclude_usernames

### IncludeFilters::has_filters branches (Tests 10-14)
10. `test_include_filters_has_filters_with_ips`
11. `test_include_filters_has_filters_with_sessions`
12. `test_include_filters_has_filters_with_threads`
13. `test_include_filters_has_filters_with_apps`
14. `test_include_filters_has_filters_with_tags`

### ExcludeFilters::has_filters branches (Tests 15-19)
15. `test_exclude_filters_has_filters_with_ips`
16. `test_exclude_filters_has_filters_with_sessions`
17. `test_exclude_filters_has_filters_with_threads`
18. `test_exclude_filters_has_filters_with_apps`
19. `test_exclude_filters_has_filters_with_tags`

## Task Commits

Each task was committed atomically:

1. **Task 1: baseline coverage report** — no commit (target/ is gitignored; HTML report + baseline-summary.txt on disk only)
2. **Task 2: new mod tests block** — `9c84539` (test)

## Files Created/Modified

- `src/pipeline/filters/types.rs` — Added `#[cfg(test)] mod tests` block (19 tests) at file end; production code unchanged

## Decisions Made

- FilterWrapper pattern chosen over direct `toml::from_str::<FiltersFeature>` to satisfy RESEARCH.md Pitfall 2 (FiltersFeature has custom Deserialize requiring correct TOML structure)
- Used plain string literals instead of `r#"..."#` for TOML without double-quote content, per clippy `needless_raw_string_hashes` lint
- Test 8 and Test 9 verify only result correctness (not log::warn! emission) per CONTEXT.md D-04 (log output is not business logic)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed clippy needless_raw_string_hashes in 3 test string literals**
- **Found during:** Task 2 (post-write clippy check)
- **Issue:** 3 raw string literals `r#"..."#` flagged by clippy because the TOML content has no `"` characters
- **Fix:** Changed `test_trxids_absent_returns_none`, `test_exec_ids_deserialized_to_hashset`, `test_exec_ids_absent_returns_none` to use plain string literals
- **Files modified:** src/pipeline/filters/types.rs
- **Verification:** `cargo clippy --all-targets -- -D warnings` exits 0
- **Committed in:** 9c84539 (Task 2 commit)

**2. [Rule 1 - Bug] Fixed clippy item-in-docs-missing-backticks for FilterWrapper doc comment**
- **Found during:** Task 2 (post-write clippy check)
- **Issue:** `FiltersFeature` in doc comment not wrapped in backticks
- **Fix:** Added backticks around both occurrences in the `FilterWrapper` doc comment
- **Files modified:** src/pipeline/filters/types.rs
- **Verification:** `cargo clippy --all-targets -- -D warnings` exits 0
- **Committed in:** 9c84539 (Task 2 commit)

**3. [Rule 1 - Bug] Applied cargo fmt to fix line-length formatting in assert! calls**
- **Found during:** Task 2 (post-write fmt check)
- **Issue:** `cargo fmt --check` failed — 8 assert! calls exceeded rustfmt line length
- **Fix:** `cargo fmt` auto-formatted assertions to multi-line style
- **Files modified:** src/pipeline/filters/types.rs
- **Verification:** `cargo fmt --check` exits 0
- **Committed in:** 9c84539 (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (all Rule 1 — code style/lint corrections)
**Impact on plan:** All fixes cosmetic/lint-compliance. No scope creep. Production code unchanged.

## Issues Encountered

None — all deviations were lint/format issues caught by the quality gate and auto-fixed inline.

## Known Stubs

None — no stubs or placeholders introduced in this plan.

## Threat Flags

None — only test code added; no new network endpoints, auth paths, file access patterns, or schema changes.

## Next Phase Readiness

- Baseline report on disk (`target/llvm-cov/baseline-summary.txt`) ready for Wave 2 comparison
- `pipeline/filters/serde_helpers.rs` now covered indirectly via types.rs tests
- `pipeline/filters/types.rs` function coverage expected to rise significantly from 31.58% baseline
- Plan 02 (csv/sqlite exporter coverage) can proceed immediately

## Self-Check: PASSED

- [x] `src/pipeline/filters/types.rs` exists and contains mod tests block
- [x] `target/llvm-cov/baseline-summary.txt` contains TOTAL line
- [x] `target/llvm-cov/html/index.html` exists
- [x] Commit `9c84539` exists in git log
- [x] 19 test functions (≥ 10 required)
- [x] FilterWrapper count ≥ 2: 10
- [x] toml::from_str count ≥ 5: 9

---
*Phase: 63-test-coverage*
*Completed: 2026-06-03*
