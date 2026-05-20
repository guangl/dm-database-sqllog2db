---
phase: 31-remove-resume
plan: "02"
status: complete
commits:
  - "f3598c7 feat(31-02): remove resume references from init templates and docs"
self-check: PASSED
---

## Summary

Cleaned all remaining resume references from test files, init templates, README, and architecture docs. Full verification: build, test, clippy, fmt all pass. CLI confirmation: `--resume`/`--state-file` removed from help, `[resume]` removed from init template.

## Key Changes

### Files Modified
- `src/cli/init.rs` — removed zh/en `[resume]` / 断点续传 comment blocks from config templates
- `README.md` — removed 断点续传 support bullet point
- `docs/architecture.md` — removed 断点续传 table row and ResumeState section

### Pre-existing (from Plan 31-01)
- `src/cli/run/tests.rs` — updated handle_run calls (8-arg signature)
- `tests/integration.rs` — removed 3 resume tests, updated all handle_run calls
- `benches/` — updated handle_run calls

### Deviations
None.

## Verification

- [x] `cargo build --release` passes
- [x] `cargo test` passes (285 unit + 36 integration = 321 tests)
- [x] `cargo clippy --all-targets -- -D warnings` passes
- [x] `cargo fmt --check` passes
- [x] `sqllog2db run --help` — no `--resume` or `--state-file`
- [x] `sqllog2db init` — no `[resume]` section in generated config
- [x] Project-wide `grep -rn resume` — zero references
