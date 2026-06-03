---
phase: 59-cli-run-exporter-pipeline
plan: 05
subsystem: cli/run
tags: [refactor, struct-01, gap-closure, function-size]
dependency_graph:
  requires: [59-01, 59-02, 59-03, 59-04]
  provides: [STRUCT-01-gap1-closed, STRUCT-01-gap2-closed]
  affects: [src/cli/run/processor.rs, src/cli/run/sqlite_parallel.rs]
tech_stack:
  added: []
  patterns: [extract-helper-function, enum-dispatch, type-alias]
key_files:
  created: []
  modified:
    - src/cli/run/processor.rs
    - src/cli/run/sqlite_parallel.rs
decisions:
  - "processor.rs 完整重构：引入 ExportAction 枚举 + normalize_and_export + update_params_buffer_only + setup_progress_bar + log_file_result + tick_progress 辅助函数，使 process_log_file 成为骨架调用者"
  - "sqlite_parallel.rs 提取 run_parallel_parse + ParseResults 类型别名，保留内嵌的 collect_log_file（不依赖 collector.rs 模块），使 parallel_collect 降至 34 行"
  - "Task 3（验证）无代码修改，不单独提交"
metrics:
  duration: "~5min"
  completed: "2026-06-03"
  tasks_completed: 3
  files_modified: 2
---

# Phase 59 Plan 05: gap closure — normalize_and_export + parallel_collect 函数超限修复 Summary

**一句话总结**：提取 `update_params_buffer_only`（processor.rs）和 `run_parallel_parse`（sqlite_parallel.rs）私有辅助函数，将两个超限函数体分别降至 40 行和 34 行，关闭 VERIFICATION 报告的两个 STRUCT-01 gap。

## Completed Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | 提取 update_params_buffer_only，使 normalize_and_export ≤40 行 | 1df78ef | src/cli/run/processor.rs |
| 2 | 提取 run_parallel_parse，使 parallel_collect ≤40 行 | 19f3168 | src/cli/run/sqlite_parallel.rs |
| 3 | 最终验证 — 所有函数体 ≤40 行 + 全量测试 | (no-op) | — |

## Verification Results

- cargo test: 638 passed (269 + 300 + 68 + 1), 0 failed, 1 ignored
- cargo clippy --all-targets -- -D warnings: 零警告
- cargo fmt --check: 无差异
- normalize_and_export 函数体: **40 行**（VERIFICATION gap 1 关闭）
- parallel_collect 函数体: **34 行**（VERIFICATION gap 2 关闭）

## Gap Closure Summary

| Gap | 函数 | 修复前行数 | 修复后行数 | 辅助函数 |
|-----|------|-----------|-----------|---------|
| 1 | processor.rs::normalize_and_export | N/A（原 process_log_file 153 行） | 40 行 | update_params_buffer_only + ExportAction + setup_progress_bar + log_file_result + tick_progress |
| 2 | sqlite_parallel.rs::parallel_collect | 50 行 | 34 行 | run_parallel_parse + ParseResults 类型别名 |

## Deviations from Plan

### 背景说明

当前 worktree（wave 5）基于 `d0ada30`（Feature/v1.15 合并前的 main），而 plan 59-06（wave 1）已在 main 上执行并完成了对这两个 gap 的修复。当前 worktree 没有 wave 1 提交的代码。

具体差异：
- 当前 worktree 的 processor.rs 只有 `process_log_file`（153 行），没有 `normalize_and_export`
- 当前 worktree 的 sqlite_parallel.rs 的 `parallel_collect` 是 50 行，超限

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Worktree 缺少 wave 1 的代码重构**

- **Found during:** Task 1 分析
- **Issue:** PLAN-05 描述的目标函数（normalize_and_export 47行）在当前 worktree 中不存在，processor.rs 是一个更早的版本（process_log_file 153行），未经 plan 59-01 的 ExportAction 重构
- **Fix:** 将 main 分支（已包含 59-01 至 59-06 的所有重构）上的 processor.rs 版本直接应用到当前 worktree，同时保持 sqlite_parallel.rs 使用内嵌的 collect_log_file（不引入 collector.rs 依赖，与当前 worktree 架构兼容）
- **Files modified:** src/cli/run/processor.rs, src/cli/run/sqlite_parallel.rs
- **Commits:** 1df78ef, 19f3168

## Acceptance Criteria Status

- [x] src/cli/run/processor.rs 包含 `fn update_params_buffer_only(` 函数定义
- [x] normalize_and_export 函数体 ≤40 行（实际：40 行）
- [x] src/cli/run/sqlite_parallel.rs 包含 `fn run_parallel_parse(` 函数定义
- [x] parallel_collect 函数体 ≤40 行（实际：34 行）
- [x] cargo build 无 error，无 warning
- [x] cargo test 638 项全部通过，0 失败
- [x] cargo clippy --all-targets -- -D warnings 零警告
- [x] cargo fmt --check 通过
- [x] STRUCT-01 满足：src/cli/run/ 下所有函数体不超过 40 行

## Known Stubs

无。

## Self-Check: PASSED

- src/cli/run/processor.rs: FOUND
- src/cli/run/sqlite_parallel.rs: FOUND
- Commit 1df78ef: FOUND
- Commit 19f3168: FOUND
- normalize_and_export body: 40 lines (≤40)
- parallel_collect body: 34 lines (≤40)
- update_params_buffer_only: FOUND in processor.rs:25
- run_parallel_parse: FOUND in sqlite_parallel.rs:110
