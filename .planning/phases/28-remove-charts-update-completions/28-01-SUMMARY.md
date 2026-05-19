---
phase: 28-remove-charts-update-completions
plan: 01
type: execute
subsystem: charts
tags: [removal, charts, plotters, config]
provides: [clean-codebase, reduced-dependencies]
requires: []
affects: [src/charts/, src/config/, src/cli/, src/pipeline/, Cargo.toml]
key-files:
  created: []
  modified:
    - src/config/mod.rs
    - src/config/validate.rs
    - src/config/apply_one.rs
    - src/cli/run/mod.rs
    - src/cli/show_config.rs
    - src/cli/init.rs
    - src/pipeline/mod.rs
    - src/pipeline/aggregator.rs
    - src/lib.rs
    - src/main.rs
    - Cargo.toml
    - Cargo.lock
    - tests/integration.rs
  deleted:
    - src/charts/frequency_bar.rs
    - src/charts/latency_hist.rs
    - src/charts/mod.rs
    - src/charts/trend_line.rs
    - src/charts/user_pie.rs
decisions:
  - "Retained PIPELINE_MIGRATION_HINT ([pipeline.charts] -> [charts]) for backward compatibility"
  - "Retained aggregator structs/methods as dead-code (Phase 30 will remove aggregator)"
  - "Pre-commit hook enforces compilation, so all 3 tasks committed in a single commit"
metrics:
  commit: "905850e"
  duration: "~35 min"
  completed_date: "2026-05-19"
---

# Phase 28 Plan 01: Remove SVG Charts Module

## One-liner

Removed entire `src/charts/` directory, `ChartsConfig` type, `plotters` dependency, all charts-related config fields/validation/display/init templates, and all associated tests.

## Tasks Executed

| # | Name | Status | Details |
|---|------|--------|---------|
| 1 | Delete charts module files and type declarations | Done | Deleted 5 files, removed `mod charts` from lib.rs/main.rs, removed ChartsConfig/ChartEntry from pipeline/mod.rs and config/mod.rs |
| 2 | Delete charts validation, runtime references, and plotters dependency | Done | Removed validate_charts(), 6 charts.* apply_one arms, generate_charts calls in run/mod.rs, [charts] display in show_config.rs, [charts] templates in init.rs (ZH+EN), plotters from Cargo.toml |
| 3 | Delete/modify charts tests and verify full chain | Done | Removed 5 pipeline/mod.rs charts tests, 8 validate.rs charts tests, 9 apply_one.rs charts tests, modified config/mod.rs test (5->4 fields), removed [charts] assertion from integration.rs |

## Acceptance Criteria Verification

- `ls src/charts/` -- No such file or directory (PASS)
- `grep -c 'mod charts;' src/lib.rs` -- 0 (PASS)
- `grep -c 'mod charts;' src/main.rs` -- 0 (PASS)
- `grep -c 'ChartsConfig' src/pipeline/mod.rs` -- 0 (PASS)
- `grep -c 'ChartEntry' src/pipeline/mod.rs` -- 0 (PASS)
- `grep -c 'ChartsConfig' src/config/mod.rs` -- 0 (PASS)
- `grep -c 'validate_charts' src/config/validate.rs` -- 0 (PASS)
- `grep -c 'generate_charts' src/cli/run/mod.rs` -- 0 (PASS)
- `grep -c '\[charts\]' src/cli/init.rs` -- 0 (PASS)
- `grep -c 'plotters' Cargo.toml` -- 0 (PASS)
- All charts test function names removed from src/ (PASS)
- `assert.*\[charts\]` removed from tests/integration.rs (PASS)
- `cargo build` -- succeeds (PASS)
- `cargo clippy --all-targets -- -D warnings` -- clean (PASS)
- `cargo test` -- 376+395+62 = 833 tests pass (PASS)
- `cargo fmt --check` -- clean (PASS)
- `grep -r 'src/charts/' src/` -- no references (PASS)

## Deviations from Plan

### Rule 2 - Missing dead code exemptions

When `ChartsConfig` was removed, the aggregator module still exports `ChartEntry`, `iter_chart_entries()`, `iter_hour_counts()`, `iter_user_counts()` which were used by the charts module. These are dead code in production builds but are still referenced by unit tests. Added `#[allow(dead_code)]` annotations on the 4 items. These will be removed in Phase 30 (aggregator removal).

### Pre-commit hook enforcement

The project pre-commit hook runs `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`. This prevented per-task commits (since intermediate states don't compile). All 3 tasks are committed in a single commit.

## Threat Model Compliance

| Threat | Disposition | Outcome |
|--------|------------|---------|
| T-28-01 (Tampering - charts module removal) | Mitigate | cargo build verifies no residual references |
| T-28-SC (Tampering - dependency removal) | Mitigate | plotters fully removed; no missing symbols |

## Known Stubs

None. All charts references have been fully removed or annotated as dead code for planned Phase 30 removal.

## Threat Flags

None. No new attack surface introduced.

## Self-Check: PASSED
