---
phase: 51-stats-cli
plan: "01"
subsystem: cli
tags: [cli, stats, clap, scaffold]
dependency_graph:
  requires: [50-01]
  provides: [stats-cli-entry]
  affects: [src/cli/opts.rs, src/cli/mod.rs, src/cli/stats/mod.rs, src/main.rs, tests/integration.rs]
tech_stack:
  added: []
  patterns: [clap-derive-subcommand, handle_xxx-orchestration, config-from-file-no-fallback]
key_files:
  created:
    - src/cli/stats/mod.rs
  modified:
    - src/cli/opts.rs
    - src/cli/mod.rs
    - src/main.rs
    - tests/integration.rs
decisions:
  - "D-05 enforced: Config::from_file used directly in Stats branch (not load_config) to prevent fallback on NotFound"
  - "D-06 enforced: Stats excluded from needs_simple_logging via nested or-pattern to use full logging stack"
  - "D-07 enforced: top==0 validated in handle_stats, returns ConfigError::InvalidValue"
  - "handle_stats accepts &Config (not config_path) — verbosity applied in main.rs Stats branch"
metrics:
  duration: "275s"
  completed: "2026-06-01"
  tasks_completed: 2
  files_changed: 5
---

# Phase 51 Plan 01: stats 子命令 CLI 脚手架 Summary

**One-liner:** `sqllog2db stats` CLI 脚手架，使用 clap derive 实现 `-c/--config` + `--top N` 参数解析，`--top 0` 校验和 config-not-found 强制报错。

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | 新增 Commands::Stats 变体 + 模块声明 + handle_stats 桩 | 28ac161 | src/cli/opts.rs, src/cli/mod.rs, src/cli/stats/mod.rs, src/main.rs |
| 2 | main.rs 分发分支 + 端到端 CLI 集成测试 | a921039 | tests/integration.rs |

## What Was Built

### CLI 层 (src/cli/opts.rs)

在 `Commands` enum 中新增 `Stats` 变体，含：
- `-c/--config`：默认 `config.toml`，`env = "SQLLOG2DB_CONFIG"`
- `--top`：`u32` 类型，默认 `20`，自然排除负数

### 模块注册 (src/cli/mod.rs)

追加 `pub mod stats;`，与现有 init/opts/run/validate 保持字母序。

### handle_stats 桩 (src/cli/stats/mod.rs)

- 签名：`pub fn handle_stats(cfg: &Config, top: u32, quiet: bool) -> Result<()>`
- `top == 0` 时返回 `ConfigError::InvalidValue { field: "--top", value: "0", reason: "must be >= 1" }`
- 写入 `log::info!("stats: top={top}")` 供集成测试验证默认值
- Phase 52 统计逻辑位置以 `// TODO(Phase 52)` 标注

### main.rs 分发分支 (src/main.rs)

- `needs_simple_logging` 使用嵌套 or-pattern 同时排除 `Run` 和 `Stats`（D-06）
- `Stats` 分支：`Config::from_file` → `apply_verbosity_to_config` → `logging::init_logging` → `handle_stats`
- 直接调用 `Config::from_file`（不调用 `load_config`）以实现 D-05（NotFound 不回落默认值）

### 集成测试 (tests/integration.rs)

新增 6 个 `test_cli_stats_*` 测试：
- `test_cli_stats_help_shows_subcommand`：验证 `--help` 包含 `--config`、`--top`、`Number of top records`
- `test_cli_stats_with_valid_config_succeeds`：有效 config 下 exit 0
- `test_cli_stats_top_default_is_20`：通过日志文件验证默认 top=20
- `test_cli_stats_top_explicit_value`：通过日志文件验证 `--top 5` 传递正确
- `test_cli_stats_top_zero_errors`：`--top 0` 退出非零，stderr 含 `--top` 和 `must be >= 1`
- `test_cli_stats_config_not_found_errors`：不存在的 config 路径退出非零

## Verification

```
cargo test stats          → 10 tests pass (4 unit + 6 integration)
cargo test                → 551 tests pass, 0 failures
cargo clippy -- -D warnings → 0 warnings
cargo fmt --check         → passes (enforced by pre-commit hook)
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Clippy: unnested-or-patterns in needs_simple_logging**
- **Found during:** Task 1 commit (pre-commit hook)
- **Issue:** `Some(Run{..}) | Some(Stats{..})` triggers `clippy::unnested-or-patterns`
- **Fix:** Changed to nested form `Some(Run{..} | Stats{..})`
- **Files modified:** src/main.rs
- **Commit:** Included in 28ac161

**2. [Rule 1 - Bug] Clippy: doc comment backtick missing for handle_stats**
- **Found during:** Task 2 commit (pre-commit hook)
- **Issue:** `/// S4: stats with --top 5 passes value 5 to handle_stats` triggers `clippy::doc-markdown`
- **Fix:** Added backticks: `` `handle_stats` ``
- **Files modified:** tests/integration.rs
- **Commit:** Included in a921039

**3. [Rule 1 - Bug] predicates `or()` method not in scope**
- **Found during:** Task 2 initial compilation
- **Issue:** `contains("x").or(contains("y"))` requires `PredicateBooleanExt` in scope
- **Fix:** Replaced with explicit `output.status.success()` + string-contains assertion
- **Files modified:** tests/integration.rs
- **Commit:** Included in a921039

## Known Stubs

| File | Content | Reason |
|------|---------|--------|
| src/cli/stats/mod.rs:17 | `// TODO(Phase 52): statistics logic` | Intentional — Phase 51 builds scaffold only; Phase 52 implements statistics output |

This stub does not prevent Phase 51's goal (CLI parameter parsing, dispatch, error paths all functional).

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes introduced.
The `--config` path and `--top` integer are the only new trust boundaries, covered by T-51-01 (top==0 validated) and T-51-02 (NotFound error surfaces path text only) from the plan's threat model.

## TDD Gate Compliance

Plan has `tdd="true"` on both tasks.

Task 1 (RED→GREEN):
- RED gate: Tests written first, module not yet registered → 0 tests found (expected RED)
- GREEN gate: commit 28ac161 contains both test code and implementation — 4 unit tests pass

Task 2 (RED→GREEN):
- Tests written before verifying GREEN (integration tests added to integration.rs)
- GREEN gate: commit a921039 — 6 integration tests pass

Both RED and GREEN gates satisfied. No REFACTOR needed (code is clean as written).

## Self-Check: PASSED

- [x] src/cli/opts.rs exists with `Stats {` variant
- [x] src/cli/mod.rs contains `pub mod stats`
- [x] src/cli/stats/mod.rs exists with `pub fn handle_stats`
- [x] src/main.rs contains `Commands::Stats` dispatch branch
- [x] tests/integration.rs contains 6 `test_cli_stats_*` functions
- [x] Commit 28ac161 exists
- [x] Commit a921039 exists
- [x] cargo test: 551 tests pass, 0 failures
- [x] cargo clippy -- -D warnings: 0 warnings
