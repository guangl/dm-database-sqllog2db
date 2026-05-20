---
phase: 31-remove-resume
plan: "01"
status: complete
commits:
  - "fe3f65a feat(31-01): remove resume source files and all references"
self-check: PASSED
---

## Summary

Deleted `src/resume.rs` and `src/config/resume.rs` source files (ResumeState, ResumeConfig), and removed all references across 13 files. `cargo build`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (321 tests), and `cargo fmt --check` all pass.

## Key Changes

### Files Deleted
- `src/resume.rs` — ResumeState struct (214 lines, 9 unit tests)
- `src/config/resume.rs` — ResumeConfig struct (21 lines)

### Files Modified
- `src/config/mod.rs` — removed `mod resume`, `pub use ResumeConfig`, `pub resume: ResumeConfig`
- `src/lib.rs` — removed `pub(crate) mod resume;`
- `src/main.rs` — removed `mod resume;`, `resume`/`state_file` from Run command match and handle_run call
- `src/cli/run/mod.rs` — handle_run 10→8 params, removed resume_state init/skip/save logic
- `src/cli/run/parallel.rs` — process_csv_parallel 13→12 params, removed resume skip logic
- `src/cli/opts.rs` — removed `--resume` and `--state-file` from Run command
- `src/lang.rs` — removed `zh_run` resume/state_file help text
- `src/error.rs` — added `#[allow(dead_code)]` on `ReadFailed` variant
- `src/cli/run/tests.rs` — updated 5 handle_run calls to 8-arg signature
- `tests/integration.rs` — deleted 3 resume tests, updated all handle_run calls
- `benches/bench_csv.rs`, `benches/bench_filters.rs`, `benches/bench_sqlite.rs` — updated handle_run calls

### Deviations
None. All tasks executed as planned.

## Verification

- [x] `cargo build` passes
- [x] `cargo clippy --all-targets -- -D warnings` passes (1 allow: ReadFailed dead_code)
- [x] `cargo test` passes (285 unit + 36 integration = 321 tests)
- [x] `cargo fmt --check` passes
- [x] `src/resume.rs` and `src/config/resume.rs` deleted
- [x] No `crate::resume`, `ResumeState`, `ResumeConfig` references remain
- [x] handle_run: 8-arg signature (removed resume, state_file_override)
