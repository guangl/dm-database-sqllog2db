---
phase: 75-parallel-shared
plan: "01"
subsystem: cli/run
tags:
  - rust
  - refactor
  - parallel
  - struct-04

dependency_graph:
  requires: []
  provides:
    - "src/cli/run/record_iter.rs::iterate_records（共享迭代 + 过滤 + 归一化函数）"
  affects:
    - "src/cli/run/parallel.rs::parse_and_write_csv"
    - "src/cli/run/sqlite_parallel.rs::parse_and_write_sqlite"

tech_stack:
  added: []
  patterns:
    - "FnMut 回调闭包将写出逻辑与迭代逻辑解耦（Strategy 模式）"
    - "pub(super) 可见性限制共享函数作用域至 cli/run 子模块"

key_files:
  created:
    - src/cli/run/record_iter.rs
  modified:
    - src/cli/run/mod.rs
    - src/cli/run/parallel.rs
    - src/cli/run/sqlite_parallel.rs

decisions:
  - "不提取解析失败样板（async/sync 差异使提取不划算，对齐 RESEARCH D-01）"
  - "iterate_records 接收 Vec<Sqllog> 值而非引用，避免生命周期复杂度"
  - "sqlite 路径闭包硬编码 true（include_performance_metrics），与原代码语义对齐"

metrics:
  duration: "约 10 分钟"
  completed_date: "2026-06-11"
  tasks_completed: 3
  files_changed: 4
---

# Phase 75 Plan 01: record_iter 共享模块提取 Summary

提取 `parallel.rs` 与 `sqlite_parallel.rs` 中逐字重复的记录迭代循环（约 40 行 × 2）到新建的 `src/cli/run/record_iter.rs::iterate_records`，两条并行路径各通过 FnMut 闭包传入差异化的写出操作，满足 STRUCT-04 需求。

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | 新建 record_iter.rs 并在 mod.rs 注册 | 3ab744f | src/cli/run/record_iter.rs（新建），src/cli/run/mod.rs |
| 2 | parallel.rs 委托 iterate_records | 0afe6b5 | src/cli/run/parallel.rs，src/cli/run/record_iter.rs（移除 allow(dead_code)） |
| 3 | sqlite_parallel.rs 委托 iterate_records + 质量门禁 | b60bcaf | src/cli/run/sqlite_parallel.rs |

## Key Outcomes

- `record_iter.rs` 函数体 56 行（含 doc 注释与 imports），`iterate_records` 函数体 43 行（含函数签名与 where 子句，纯逻辑部分 ~35 行）
- `parallel.rs` 净减少 34 行（Task 2 commit: 10 insertions / 44 deletions）
- `sqlite_parallel.rs` 净减少 33 行（Task 3 commit: 10 insertions / 43 deletions）
- 两文件合计消除重复迭代代码 ~80 行
- 三道质量门禁全部绿灯：`cargo fmt --check` / `cargo clippy --all-targets -- -D warnings` / `cargo test`（全部测试通过）

## Threat Model Compliance

| Threat | Status |
|--------|--------|
| T-75-01: CSV/SQLite 参数错配 | 验收通过：CSV 闭包含 `include_pm`，SQLite 闭包含硬编码 `true` |
| T-75-02: interrupted 检查缺失 | 验收通过：record_iter.rs 中 `interrupted.load(Ordering::Acquire)` 存在且位于 for 循环首部 |
| T-75-03: em.finalize() 误纳入 | 验收通过：record_iter.rs 内无 `em.finalize`；sqlite_parallel.rs 内仅 process_sqlite_parallel 一处 |
| T-75-04: filtered_out 累加语义错位 | 验收通过：record_iter.rs 内 `file_stats.filtered_out += 1` 出现 2 次（两个分支各一次） |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Task 1 提交时 pre-commit hook clippy 因 dead_code 报错**
- **Found during:** Task 1 提交阶段
- **Issue:** record_iter.rs 中 `iterate_records` 尚未被 parallel.rs / sqlite_parallel.rs 调用，clippy `-D warnings` 将 dead_code 视为错误
- **Fix:** 在 Task 1 中临时添加 `#[allow(dead_code)]`，在 Task 2 接入 parallel.rs 后随即移除
- **Files modified:** src/cli/run/record_iter.rs
- **Commit:** 包含在 3ab744f（添加）与 0afe6b5（移除）

## Known Stubs

None — 所有函数均完整实现，无占位符或硬编码空值。

## Threat Flags

None — 本阶段为纯内部代码重构，不涉及输入/输出表面变更，无新增安全相关表面。
