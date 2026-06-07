---
phase: 71-mod-rs-mod-rs-pub-use
plan: 01
subsystem: cli/stats
tags: [refactor, module-structure, pub-use]
dependency_graph:
  requires: []
  provides: [stats-handler-split]
  affects: [src/cli/stats]
tech_stack:
  added: []
  patterns: [mod-rs-pub-use, handler-module-pattern]
key_files:
  created:
    - src/cli/stats/handler.rs
    - src/cli/stats/tests.rs
  modified:
    - src/cli/stats/mod.rs
decisions:
  - "merge_stats_options 改为 pub(super) fn，使 tests.rs 通过 super::handler:: 路径访问"
  - "mod.rs 注释中 crate::stats::run_stats 用反引号包裹，满足 clippy::doc-markdown"
metrics:
  duration: "~5 minutes"
  completed_date: "2026-06-07"
  tasks_completed: 1
  tasks_total: 1
  files_changed: 3
---

# Phase 71 Plan 01: stats/mod.rs 拆分 Summary

**One-liner:** 将 stats/mod.rs 的函数实现与测试拆分至 handler.rs + tests.rs，mod.rs 仅保留 6 行模块声明与 pub use 重导出。

## What Was Built

将原有 147 行的 `src/cli/stats/mod.rs` 重构为三个文件：

- **`src/cli/stats/handler.rs`** — `handle_stats` + `merge_stats_options` 函数实现（42 行）
- **`src/cli/stats/tests.rs`** — 全部 8 个单元测试 + `make_test_config_with_log` helper（89 行）
- **`src/cli/stats/mod.rs`** — 仅 6 行：注释 + `mod handler` + `#[cfg(test)] mod tests` + `pub use handler::handle_stats`

外部调用路径 `crate::cli::stats::handle_stats` 通过 `pub use` 重导出保持完全不变，`src/main.rs` 无需修改。

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | 拆分 stats/mod.rs 到 handler.rs + tests.rs | bf83dca | src/cli/stats/mod.rs, src/cli/stats/handler.rs, src/cli/stats/tests.rs |

## Verification Results

- `cargo build` — 通过
- `cargo clippy --all-targets -- -D warnings` — 通过，0 warnings
- `cargo test` — 通过（395 lib 单元测试，包含 cli::stats::tests 中全部 8 个测试）
- `cargo fmt` — 格式化已应用
- mod.rs grep 检验：无 `fn`/`struct`/`impl` 关键字

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 修复 clippy::doc_markdown 警告**
- **Found during:** Step 4 (cargo clippy)
- **Issue:** mod.rs 注释中 `crate::stats::run_stats` 未用反引号包裹，触发 `clippy::doc-markdown` lint
- **Fix:** 改为 `` `crate::stats::run_stats` ``
- **Files modified:** src/cli/stats/mod.rs
- **Commit:** 包含在 bf83dca 内（在提交前修复）

## Known Stubs

None.

## Threat Flags

None — 纯重构，无新增网络端点、认证路径或 schema 变更。

## Self-Check: PASSED

- [x] src/cli/stats/handler.rs — FOUND
- [x] src/cli/stats/tests.rs — FOUND
- [x] src/cli/stats/mod.rs — FOUND (6 lines, mod-only)
- [x] Commit bf83dca — FOUND
