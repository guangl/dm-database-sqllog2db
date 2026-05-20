---
phase: 32-cleanup-project-structure
plan: 02
subsystem: cleanup
tags: code-cleanup, dead-code, stale-tests
requires:
  - phase: 28
    provides: SVG charts, self-update, shell completions removal
  - phase: 29
    provides: stats/digest commands removal
  - phase: 30
    provides: template analysis removal
  - phase: 31
    provides: resume/checkpoint removal
  - phase: 32-01
    provides: Config and pipeline type cleanup
provides:
  - Removed stale reference to companion/template feature in cli/run tests
  - Removed stale comment referencing removed TemplateAggregator in processor.rs
affects: []

tech-stack:
  added: []
  patterns: []

key-files:
  created: []
  modified:
    - src/cli/run/tests.rs
    - src/cli/run/processor.rs

key-decisions: []

patterns-established: []

requirements-completed: [RM-08]

duration: 1min
completed: 2026-05-20
---

# Phase 32 Plan 02: Exporter/CLI dead code verification and stale test removal

**Verify all Exporter layer and CLI layer dead code cleanup from Phases 28-31 is complete; remove one residual stale test and comment**

## Performance

- **Duration:** 1 min
- **Started:** 2026-05-20T04:42:49Z
- **Completed:** 2026-05-20T04:44:28Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments

- Verified that companion.rs, write_template_stats, ExporterKind/ExporterManager template methods, and CSV companion tests are all already removed
- Verified that CLI opts no longer contain Stats/Digest/Completions/SelfUpdate/Man command variants, and Run variant has no resume/state_file fields
- Verified that main.rs has no stale match arms or error handling for removed commands
- Verified that cli/run/mod.rs, parallel.rs, processor.rs have no TemplateAggregator, template_reporter, resume, or charts code
- Removed stale `test_no_template_stats_when_disabled` test referencing removed companion feature
- Removed stale CR-01 comment in processor.rs referencing removed TemplateAggregator

## Task Commits

All cleanup tasks were pre-verified as already complete (codebase was already in desired state from prior phases). Only one commit was needed for residual cleanup:

1. **Task 1: Delete companion.rs + Exporter layer write_template_stats** — already clean (no changes needed)
2. **Task 2: Delete CLI opts command variants + main.rs match arms** — already clean (no changes needed)
3. **Task 3: Delete cli/run modules template/resume/charts dead code** — `3a8aae2` (feat)

## Files Modified

- `src/cli/run/tests.rs` — Removed stale `test_no_template_stats_when_disabled` test that referenced removed companion feature
- `src/cli/run/processor.rs` — Removed stale comment referencing `aggregator` (CR-01, removed TemplateAggregator)

## Decisions Made

None — followed plan as specified. All planned deletions were verified as already complete from earlier phases.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Cleanup] Removed stale test and comment**
- **Found during:** Task 3 (cli/run module cleanup)
- **Issue:** `test_no_template_stats_when_disabled` in `tests.rs` references `companion_path` and `out_templates.csv` (removed companion feature). `processor.rs` comment references removed `TemplateAggregator`.
- **Fix:** Removed stale test function and updated stale comment.
- **Files modified:** src/cli/run/tests.rs, src/cli/run/processor.rs
- **Verification:** `cargo build` + `cargo test --no-run` both pass.
- **Committed in:** `3a8aae2` (Task 3 commit)

---

**Total deviations:** 1 auto-fixed (1 missing cleanup)
**Impact on plan:** Minimal — both test and comment are structural residuals from earlier removal phases. No scope creep.

## Issues Encountered

None — all planned deletions confirmed as already complete from Phases 28-31.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Exporter layer and CLI layer dead code fully cleaned up
- Ready for Plan 32-03 (final cleanup plan)

---
*Phase: 32-cleanup-project-structure*
*Completed: 2026-05-20*
