---
phase: 71-mod-rs-mod-rs-pub-use
plan: "02"
subsystem: pipeline/filters
tags: [refactor, module-structure, rust]
dependency_graph:
  requires: []
  provides: [pipeline/filters 模块拆分完成]
  affects: [src/pipeline/filters/mod.rs, src/cli/run/prescan.rs]
tech_stack:
  added: []
  patterns: [mod-declaration-only, per-type-impl-files]
key_files:
  created:
    - src/pipeline/filters/feature_ops.rs
    - src/pipeline/filters/indicator_ops.rs
    - src/pipeline/filters/sql_ops.rs
    - src/pipeline/filters/tests.rs
  modified:
    - src/pipeline/filters/mod.rs
    - src/cli/run/prescan.rs
decisions:
  - "TrxidSet 的 pub(crate) use 限定为 #[cfg(test)] 导出，因为所有非类型定义中的 TrxidSet 消费者均在 #[test] 函数内"
  - "prescan.rs 的 use 路径从 filters::types:: 改为 filters::，让 mod.rs 的 pub use 有 crate 内部消费者，消除 clippy unused_imports 错误"
metrics:
  duration_seconds: 306
  completed_date: "2026-06-07"
  tasks_completed: 1
  tasks_total: 1
  files_changed: 6
---

# Phase 71 Plan 02: pipeline/filters/mod.rs 拆分 Summary

**一行概要：** 将 246 行的 pipeline/filters/mod.rs 拆分为 4 个子文件（feature_ops/indicator_ops/sql_ops/tests），mod.rs 精简到 16 行纯声明。

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | 拆分 pipeline/filters/mod.rs 到 4 个新文件 | dcd5444 | feature_ops.rs, indicator_ops.rs, sql_ops.rs, tests.rs, mod.rs, prescan.rs |

## What Was Built

将 `src/pipeline/filters/mod.rs`（246 行）重构为 5 个文件：

- **`feature_ops.rs`**：`FiltersFeature` 的 impl 块（`has_transaction_filters`、`merge_found_trxids`、`#[cfg(test)] has_filters`）
- **`indicator_ops.rs`**：`IndicatorFilters` 的 impl 块（`has_filters`、`#[cfg(test)] matches`）
- **`sql_ops.rs`**：`SqlFilters` 的 impl 块（`has_filters`）
- **`tests.rs`**：全部 18 个单元测试（从 `#[cfg(test)] mod tests` 内提取）
- **`mod.rs`**：精简至 16 行，仅含模块声明 + pub use 重导出

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] TrxidSet unused import 导致 clippy -D warnings 失败**
- **Found during:** Task 1 验证阶段（cargo clippy --all-targets）
- **Issue:** 拆分后 `pub(crate) use serde_helpers::TrxidSet` 在 lib 的非测试编译路径下无消费者，bin target 编译时 clippy 报 `unused import` 错误
- **Fix:** 将该导出限定为 `#[cfg(test)]`，因为所有消费者（filter_processor.rs 中的测试）均在 `#[cfg(test)]` 内
- **Files modified:** src/pipeline/filters/mod.rs
- **Commit:** dcd5444

**2. [Rule 1 - Bug] prescan.rs 使用 filters::types:: 子路径导致 pub use 无内部消费者**
- **Found during:** Task 1 验证阶段（cargo clippy --all-targets）
- **Issue:** 修复 TrxidSet 后，`pub use types::{IndicatorFilters, SqlFilters}` 仍报 unused（在 bin target 编译时），因为 prescan.rs 用 `crate::pipeline::filters::types::IndicatorFilters` 直接访问子模块
- **Fix:** 将 prescan.rs 的两处 import 改为通过 `crate::pipeline::filters::IndicatorFilters/SqlFilters`（顶层 pub use 路径）
- **Files modified:** src/cli/run/prescan.rs
- **Commit:** dcd5444（与主要重构在同一 commit）

## Verification Results

- `cargo clippy --all-targets -- -D warnings`: 通过
- `cargo test`: 全部通过（395 lib 测试 + 87 integration 测试 + 其他）
- `cargo fmt`: 通过
- mod.rs grep 检查：无 fn/struct/impl 关键字

## Self-Check: PASSED

- [x] feature_ops.rs 已创建
- [x] indicator_ops.rs 已创建
- [x] sql_ops.rs 已创建
- [x] tests.rs 已创建
- [x] mod.rs 已改写（16 行）
- [x] commit dcd5444 存在
- [x] cargo clippy 通过
- [x] cargo test 全绿
