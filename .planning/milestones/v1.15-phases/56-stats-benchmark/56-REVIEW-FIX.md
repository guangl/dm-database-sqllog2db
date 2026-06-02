---
phase: 56-stats-benchmark
fixed_at: 2026-06-02T00:00:00Z
review_path: .planning/phases/56-stats-benchmark/56-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 56: Code Review Fix Report

**Fixed at:** 2026-06-02T00:00:00Z
**Source review:** .planning/phases/56-stats-benchmark/56-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5
- Fixed: 5
- Skipped: 0

## Fixed Issues

### WR-01 + IN-01: `stats` parse errors logged at wrong level

**Files modified:** `src/stats/mod.rs`
**Commit:** 02f22e6
**Applied fix:** Changed `log::info!` to `log::warn!` for the parse error count summary in
`scan_files_into_accumulator`. This matches the behavior of `processor.rs:145` which uses
`log::warn!` for the same condition. Per instructions, the minimal fix (log level only) was
applied without changing function signatures or exit codes — that is a larger refactor.

### WR-02: `scan_files` early-termination yields misleading output

**Files modified:** `src/scanner.rs`
**Commit:** fb11de1
**Applied fix:** Replaced the bare `build_parser(file_path)?` with a `match` that logs a
`log::warn!` before returning `Err`. The warning names the failing file and how many files
remain unscanned, giving users context about partial progress. The abort-on-failure behavior
is preserved (no behavioral change, only better observability).

### WR-03: `build_parser` doc comment incorrectly references `PathNotFound`

**Files modified:** `src/scanner.rs`
**Commit:** 926a4e0
**Applied fix:** Updated the doc comment to explicitly state that both non-UTF8 paths and
file-open failures map to `InvalidPath` (not `PathNotFound`), and added a note that callers
can inspect the `reason` field to distinguish the two sub-cases.

### IN-02: `benches/BENCHMARKS.md` Phase 56 section missing

**Files modified:** `benches/BENCHMARKS.md`
**Commit:** 55a8a91
**Applied fix:** Added a `## Phase 56 — CI Benchmark Artifact Collection（v1.15）` section
after the footnote, documenting that Phase 56 (D-04) introduced only the CI artifact workflow
with no new criterion benchmark groups, and that existing baselines remain current.

---

_Fixed: 2026-06-02T00:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
