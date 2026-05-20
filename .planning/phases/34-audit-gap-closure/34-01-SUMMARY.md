---
phase: 34-audit-gap-closure
plan: 01
subsystem: config
tags: [config, validation, serde, toml, deprecation]

# Dependency graph
requires:
  - phase: 30-codebase-streamline
    provides: [template analysis removal]
provides:
  - [template] TOML section explicit rejection with clear error message
affects: [any future plan touching config validation]

# Tech tracking
tech-stack:
  added: []
  patterns: [serde rename for deprecated config section capture]

key-files:
  created: []
  modified:
    - src/config/mod.rs
    - src/config/validate.rs

key-decisions:
  - "[template] rejection uses its own error message (not PIPELINE_MIGRATION_HINT) because template functionality was fully removed, not migrated"

patterns-established:
  - "Deprecated TOML sections are captured via serde rename + Option<toml::Value> field, then rejected in validate_and_compile()"

requirements-completed: [RM-05, RM-08]

# Metrics
duration: 8min
completed: 2026-05-20
---

# Phase 34-01: [template] Deprecation Rejection Summary

**Close INT-02 audit gap by adding explicit [template] section rejection in config validation, matching the existing [pipeline] deprecation pattern**

## Performance

- **Duration:** 8 min
- **Started:** 2026-05-20T16:25:00Z
- **Completed:** 2026-05-20T16:33:00Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments
- Config struct adds `template_deprecated: Option<toml::Value>` field with serde rename "template"
- `validate_and_compile()` returns clear error message when [template] section is present
- Existing test updated to remove [template] reference; new test covers rejection behavior
- All four gates pass: build --release, test (606), clippy, fmt

## Task Commits

Each task was committed atomically:

1. **Task 1: Add template_deprecated field and rejection logic** - `883bc95` (combined with Task 2 due to pre-commit test breakage)
2. **Task 2: Update tests** - `883bc95` (test commit)
3. **Task 3: Full gate verification** - `46e1cab` (chore)

**Plan metadata:** (committed below as part of this summary)

_Note: Task 1 commit was merged into Task 2 because the pre-commit hook runs `cargo test`, and the existing test `test_validate_new_top_level_format_passes` (which included [template]) would fail before the test update in Task 2. This is an expected dependency between code change and test update — see Deviations._

## Files Created/Modified
- `src/config/mod.rs` - Config struct: added `template_deprecated: Option<toml::Value>` with serde rename "template"
- `src/config/validate.rs` - `validate_and_compile()`: added [template] rejection check; updated existing test; added new `test_validate_rejects_template_section` test

## Decisions Made
- [template] rejection uses the Chinese error message `配置段 [template] 已废弃，请移除此配置段` (not PIPELINE_MIGRATION_HINT), because template functionality was fully removed in Phase 30 with no migration path. Users only need to remove the section.

## Deviations from Plan

### Pre-commit coupling

**1. [Rule 3 - Blocking] Task 1 and Task 2 commits merged due to pre-commit test failure**
- **Found during:** Task 1 commit (pre-commit hook)
- **Issue:** The pre-commit hook runs `cargo test`, which includes `test_validate_new_top_level_format_passes` — this test's TOML config contains `[template]`. After Task 1 added the rejection logic, the test fails before Task 2's test update can be applied.
- **Fix:** The code changes from Task 1 and test changes from Task 2 are committed together in `883bc95`. This is the correct behavior: the implementation and test update are inherently coupled because the new validation rejects the TOML used by the old test.
- **Files modified:** src/config/mod.rs, src/config/validate.rs (both in the same commit)
- **Verification:** `cargo test` passes (all 606 tests)
- **Committed in:** `883bc95` (Task 2 commit)

---

**Total deviations:** 1 (pre-commit coupling)
**Impact on plan:** Zero. All changes delivered correctly. The merged commit is a workflow artifact, not a functional gap.

## Issues Encountered
- Pre-commit hook prevented standalone Task 1 commit because existing test TOML contained `[template]`. This is expected — the test update is a dependent change.

## Next Phase Readiness
- Config validation now explicitly rejects [template] section, closing INT-02 audit gap
- Ready for next plan (34-02) which will address remaining audit gaps

---
*Phase: 34-audit-gap-closure*
*Completed: 2026-05-20*
