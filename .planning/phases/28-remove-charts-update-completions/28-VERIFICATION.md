---
phase: 28-remove-charts-update-completions
verified: 2026-05-20T19:15:00Z
status: passed
score: 14/14 must-haves verified
overrides_applied: 0
gaps: []
---

# Phase 28: 移除图表、自更新、补全 — Verification Report

**Phase Goal:** Remove SVG charts, self-update, and Shell completions/man-page generation features to simplify the project
**Verified:** 2026-05-20T19:15:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

#### From ROADMAP Success Criteria

| #  | Truth | Status | Evidence |
| -- | ----- | ------ | -------- |
| 1  | `src/charts/` 目录已移除，`plotters` 依赖从 Cargo.toml 中删除 | VERIFIED | `ls src/charts/` returns "No such file or directory"; `grep -c 'plotters' Cargo.toml` returns 0 |
| 2  | `src/cli/update.rs` 已移除，`self_update`/`reqwest`/`rustls` 依赖从 Cargo.toml 中删除 | VERIFIED | `ls src/cli/update.rs` returns "No such file or directory"; `grep -c 'self_update\|reqwest\|rustls' Cargo.toml` returns 0 |
| 3  | `sqllog2db --help` 不再显示 `self-update`、`completions`、`man` 子命令 | VERIFIED | `cargo run -- --help` output confirmed — only run/init/validate/show-config/stats/digest shown |
| 4  | `clap_complete`/`clap_mangen` 依赖从 Cargo.toml 中删除 | VERIFIED | `grep -c 'clap_complete\|clap_mangen' Cargo.toml` returns 0 |
| 5  | `[charts]` 配置段被移除，包含该配置段的旧文件在 `validate` 时不再被接受（或被忽略） | VERIFIED | Config struct has no `charts` field; old `[pipeline]` format (wrapping `[pipeline.charts]`) returns migration error via `PIPELINE_MIGRATION_HINT` (intentionally retained) |

#### From PLAN 01 (Charts, RM-01)

| #  | Truth | Status | Evidence |
| -- | ----- | ------ | -------- |
| 6  | `src/charts/` 目录已不存在 | VERIFIED | Directory absent |
| 7  | Cargo.toml 不再包含 plotters 依赖 | VERIFIED | grep returns 0; stale Cargo.lock entry present but harmless |
| 8  | Config 结构体不再包含 charts 字段 | VERIFIED | `grep -n 'ChartsConfig\|pub charts:' src/config/mod.rs` returns 0 |
| 9  | validate() 和 validate_and_compile() 不再调用 validate_charts() | VERIFIED | `grep -c 'validate_charts' src/config/validate.rs` returns 0 |
| 10 | apply_one() 不再处理 charts.* 键 | VERIFIED | `grep -n 'charts\.' src/config/apply_one.rs` returns 0 |
| 11 | init 生成的模板不再包含 [charts] 注释段 | VERIFIED | `grep -c '\[charts\]' src/cli/init.rs` returns 0 |
| 12 | 所有 charts 相关的测试已被删除或修改 | VERIFIED | `grep -rn 'test_charts_config\|test_validate_charts\|test_apply_one_charts\|test_config_has_5_top_level' src/` returns 0; `test_config_has_4_top_level_optional_fields` correctly updated |

#### From PLAN 02 (Self-update, RM-02)

| #  | Truth | Status | Evidence |
| -- | ----- | ------ | -------- |
| 13 | `src/cli/update.rs` 文件已不存在 | VERIFIED | File absent |
| 14 | Cargo.toml 不再包含 self_update 依赖 | VERIFIED | `grep -c 'self_update' Cargo.toml` returns 0 |
| 15 | error::Error 枚举不再包含 Update 变体 | VERIFIED | `grep -c 'UpdateError\|Error::Update' src/error.rs` returns 0 |
| 16 | UpdateError 类型已被删除 | VERIFIED | Same as above |
| 17 | 所有测试通过，clippy 无警告 | VERIFIED | See full suite below |

#### From PLAN 03 (Completions/Man, RM-07)

| #  | Truth | Status | Evidence |
| -- | ----- | ------ | -------- |
| 18 | Cargo.toml 不再包含 clap_complete 和 clap_mangen 依赖 | VERIFIED | `grep -c 'clap_complete\|clap_mangen' Cargo.toml` returns 0 |
| 19 | src/cli/opts.rs 不再引用 clap_complete | VERIFIED | `grep -c 'clap_complete\|Completions\|^    Man\|generate_completions' src/cli/opts.rs` returns 0 |
| 20 | src/lang.rs 的 apply_zh 不再包含 completions/man 的本地化 | VERIFIED | `grep -c 'mut_subcommand("completions"\|mut_subcommand("man"' src/lang.rs` returns 0 |

**Score:** 14/14 must-haves verified (consolidated deduplicated count)

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `src/charts/` | MUST NOT exist | VERIFIED | Directory fully deleted (5 files removed) |
| `src/cli/update.rs` | MUST NOT exist | VERIFIED | File deleted (97 lines) |
| `src/lib.rs` | MUST NOT contain "mod charts;" | VERIFIED | grep returns 0 |
| `src/main.rs` | MUST NOT contain "mod charts;" / "SelfUpdate" / "check_for_updates" / "UpdateError" / "Completions" / "Man" | VERIFIED | grep returns 0 |
| `src/config/mod.rs` | MUST NOT contain "ChartsConfig" or "pub charts:" | VERIFIED | grep returns 0 |
| `src/pipeline/mod.rs` | MUST NOT contain "ChartsConfig" or "ChartEntry" | VERIFIED | grep returns 0 |
| `src/cli/mod.rs` | MUST NOT contain "pub mod update;" | VERIFIED | grep returns 0 |
| `src/cli/opts.rs` | MUST NOT contain "clap_complete", "Completions", "Man", "SelfUpdate" | VERIFIED | grep returns 0 |
| `src/error.rs` | MUST NOT contain "UpdateError" or "Error::Update" | VERIFIED | grep returns 0 |
| `src/lang.rs` | MUST NOT contain completions/man mut_subcommand | VERIFIED | grep returns 0 |
| `Cargo.toml` | MUST NOT contain "plotters", "self_update", "clap_complete", "clap_mangen" | VERIFIED | All grep return 0 |
| `tests/integration.rs` | MUST NOT contain `assert.*[charts]` | VERIFIED | grep returns 0 |

### Key Link Verification

| From | To | Via | Pattern | Status |
| ---- | -- | --- | ------- | ------ |
| `src/main.rs` / `src/lib.rs` | `src/charts/` | `mod charts;` | no longer present | VERIFIED |
| `src/config/mod.rs` | `src/pipeline/mod.rs` | `use ChartsConfig` | no longer present | VERIFIED |
| `src/cli/mod.rs` | `src/cli/update.rs` | `pub mod update;` | no longer present | VERIFIED |
| `src/main.rs` | `src/cli::update` | `check_for_updates_at_startup` / `handle_update` | no longer present | VERIFIED |
| `src/error.rs` | `UpdateError` | `Error::Update` | no longer present | VERIFIED |
| `src/cli/opts.rs` | `clap_complete` | `use clap_complete::{Shell, generate};` | no longer present | VERIFIED |
| `src/main.rs` | `clap_mangen` | Man match arm | no longer present | VERIFIED |
| `src/lang.rs` | completions/man | `mut_subcommand("completions"/"man")` | no longer present | VERIFIED |

All 8 key links verified as removed.

### Data-Flow Trace (Level 4)

Not applicable — Phase 28 is a removal phase with no new data-flow paths added. The only remaining data-flows are from retained modules (run, stats, digest), which are outside this phase's scope.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| `cargo build` | `cargo build 2>&1 | tail -5` | `Finished dev profile` | PASS |
| `cargo clippy` (no warnings) | `cargo clippy --all-targets -- -D warnings 2>&1 | tail -5` | Clean output | PASS |
| `cargo test` (all pass) | `cargo test 2>&1 | grep 'test result'` | 376 + 394 + 62 = 832 tests passed | PASS |
| `cargo fmt` (formatting clean) | `cargo fmt --check 2>&1` | No output (clean) | PASS |
| --help no longer shows removed subcommands | `cargo run -- --help 2>&1 | grep -E 'self-update\|completions\|man'` | No matches | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ---------- | ----------- | ------ | -------- |
| RM-01 | 28-01-PLAN.md | Remove SVG charts module (`src/charts/`), `plotters` dep, `[charts]` config section | SATISFIED | charts dir deleted, plotters removed from Cargo.toml, ChartsConfig removed from Config struct and type system |
| RM-02 | 28-02-PLAN.md | Remove self-update (`cli/update.rs`), `self_update`/`reqwest`/`rustls` deps, `self-update` subcommand | SATISFIED | update.rs deleted, self_update dep removed, SelfUpdate variant and UpdateError type removed |
| RM-07 | 28-03-PLAN.md | Remove Shell completions + Man page (`completions`/`man` subcommands), `clap_complete`/`clap_mangen` deps | SATISFIED | Completions/Man variants removed from opts.rs, match arms removed from main.rs, localizations removed from lang.rs, deps removed from Cargo.toml |

All 3 requirement IDs (RM-01, RM-02, RM-07) from the PLAN frontmatter are accounted for. No orphaned requirements.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| `src/pipeline/aggregator.rs` | 44, 178, 193, 199 | `ChartEntry` struct + `iter_*` methods with `#[allow(dead_code)]` | Warning | Dead code deferred to Phase 30 (documented in PLAN 01 SUMMARY) |
| `src/config/mod.rs` | 19 | `PIPELINE_MIGRATION_HINT` retains `[pipeline.charts] -> [charts]` | Info | Intentionally retained for backward-compatible migration error messages |

No blocker anti-patterns found. The two items above are documented intentional decisions.

### Human Verification Required

None. All automated checks pass. The removal is complete and verifiable through codebase inspection.

## Gaps Summary

No gaps found. All 14 must-haves are VERIFIED. The phase goal — removing SVG charts, self-update, and shell completions/man-page generation — is fully achieved:

- **SVG charts (RM-01):** Full removal of `src/charts/` directory (5 files), `plotters` dependency, `ChartsConfig` type, `[charts]` config field, all validation/apply_one/show_config/init template references, and all associated tests.
- **Self-update (RM-02):** Full removal of `src/cli/update.rs` file (97 lines), `self_update` dependency with features (`reqwest`, `rustls`, `compression-flate2`), `SelfUpdate` CLI subcommand, `UpdateError` type, and all associated tests and localizations.
- **Completions/Man (RM-07):** Full removal of `Completions` and `Man` CLI subcommands, `generate_completions` method, `clap_complete`/`clap_mangen` dependencies, and Chinese localizations for both subcommands.

Deferred items (documented as intentional):
- `ChartEntry` and `iter_*` methods in `aggregator.rs` — to be removed in Phase 30
- `PIPELINE_MIGRATION_HINT` with `[pipeline.charts]` reference — intentionally retained for migration error messages
- `plotters` stale entry in `Cargo.lock` — harmless, not in Cargo.toml, build verified

Full verification suite: `cargo build` (PASS), `cargo clippy --all-targets -- -D warnings` (PASS), `cargo test` — 832 all passing (PASS), `cargo fmt --check` (PASS).

---

_Verified: 2026-05-20T19:15:00Z_
_Verifier: Claude (gsd-verifier)_
