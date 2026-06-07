---
phase: 71-mod-rs-mod-rs-pub-use
plan: "04"
subsystem: stats
tags: [refactor, module-structure, pub-use]
dependency_graph:
  requires: []
  provides: [stats/runner.rs, stats/tests.rs]
  affects: [src/stats/mod.rs, src/stats/runner.rs, src/stats/tests.rs]
tech_stack:
  added: []
  patterns: [mod-declarations-only, pub-use-reexport, single-responsibility-modules]
key_files:
  created:
    - src/stats/runner.rs
    - src/stats/tests.rs
  modified:
    - src/stats/mod.rs
decisions:
  - "runner.rs 使用 `use super::config as stats_config` 避免与 crate::config 命名冲突"
  - "tests.rs 直接使用 `use super::runner::run_stats` 而非 `use super::*`，明确依赖路径"
  - "mod.rs 保留 #[allow(unused_imports)] 属性宏以防 lib 二进制目标未用时 clippy 警告"
metrics:
  duration: ~5min
  completed: 2026-06-07T11:59:59Z
  tasks_completed: 1
  tasks_total: 1
  files_changed: 3
---

# Phase 71 Plan 04: stats/mod.rs 拆分为 runner.rs + tests.rs Summary

将 `src/stats/mod.rs`（200 行）中的三个函数实现拆分到 `runner.rs`，5 个测试拆分到 `tests.rs`，`mod.rs` 缩减为 19 行仅含模块声明与 pub use 重导出。

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | 拆分 stats/mod.rs 到 runner.rs + tests.rs | e87c131 | src/stats/mod.rs, src/stats/runner.rs, src/stats/tests.rs |

## What Was Built

**src/stats/runner.rs** — 接管 `run_stats`（pub）、`scan_files_into_accumulator`（私有）、`write_stats_output`（私有）三个函数。顶部 use 使用 `super::config as stats_config` 避免与 `crate::config` 名称冲突。

**src/stats/tests.rs** — 接管原 `#[cfg(test)] mod tests` 内全部内容（5 个 `#[test]` + `make_csv_config`/`write_test_log` 两个 helper）。使用 `use super::runner::run_stats` 明确导入路径。

**src/stats/mod.rs** — 改写为 19 行：doc 注释 + 4 个 `pub mod` + `mod runner` + `#[cfg(test)] mod tests` + `pub use runner::run_stats` + 两个 `pub use config::*` 重导出。

## Verification Results

- `cargo build`: 通过
- `cargo clippy --all-targets -- -D warnings`: 通过（0 warnings）
- `cargo test`: 全部通过（912 个测试，2 个 ignored）
- grep 检查 mod.rs 无 `fn`/`struct`/`impl`: OK
- mod.rs 行数: 19 行（≤ 20 行）
- `crate::stats::run_stats` / `StatsConfig` / `validate_time_str` 原路径可达: 已验证

## Deviations from Plan

None — 计划完全按预期执行。

`cargo fmt` 自动将 `runner.rs` 中的 use 语句按字母顺序重排（将 `use super::*` 移至 `use crate::*` 之前），属于正常格式化行为，不影响语义。

## Known Stubs

None.

## Threat Flags

None — 此次为纯内部重构，未引入新的网络端点、auth 路径或文件访问模式。

## Self-Check: PASSED

- [x] src/stats/runner.rs 存在
- [x] src/stats/tests.rs 存在
- [x] src/stats/mod.rs 已简化（19 行）
- [x] commit e87c131 存在
