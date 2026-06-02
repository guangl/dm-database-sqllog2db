---
phase: 59-cli-run-exporter-pipeline
plan: "03"
subsystem: cli
tags: [rust, refactor, cli-run, filter-processor, struct-01]

# Dependency graph
requires:
  - phase: 59-01
    provides: handle_run 拆分，建立私有辅助函数模式
  - phase: 59-02
    provides: process_csv_parallel 拆分完成
provides:
  - run_file_loop 私有辅助函数（D-12），封装顺序导出的文件循环
  - build_include_groups / build_exclude_groups 私有辅助函数（D-13），封装 filter 组构建
affects: [59-04]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "提取纯函数辅助：将过长函数的独立逻辑段提取为模块级私有 fn，保持调用方骨架简洁"
    - "fatal 早返回通过 Result 传播：run_file_loop 返回 Err 时调用方跳过 finalize/log_stats，与原行为一致"

key-files:
  created: []
  modified:
    - src/cli/run/mod.rs
    - src/cli/run/filter_processor.rs

key-decisions:
  - "run_file_loop 返回 Result<(Vec<(PathBuf, usize)>, ErrorStats)>，fatal 时直接返回 Err，调用方用 ? 传播，保留原 fatal 跳过 finalize 的语义"
  - "build_include_groups / build_exclude_groups 字段顺序与原 from_feature 完全一致（users→ips→sessions→threads→statements→apps→tags），确保 11 项单元测试不变"

patterns-established:
  - "Pattern 1: 超 40 行函数拆分——保留骨架函数（负责资源生命周期），提取纯循环/构建逻辑为私有辅助"

requirements-completed:
  - STRUCT-01

# Metrics
duration: 4min
completed: 2026-06-02
---

# Phase 59 Plan 03: cli/run + filter_processor 超长函数拆分 Summary

**将 run_sequential（52 行）与 FilterProcessor::from_feature（43 行）分别提取为 run_file_loop、build_include_groups、build_exclude_groups 三个辅助函数，满足 STRUCT-01 ≤40 行约束**

## Performance

- **Duration:** 4 min
- **Started:** 2026-06-02T23:51:24Z
- **Completed:** 2026-06-02T23:55:31Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- `run_sequential` 函数体从 40 行精简为 17 行，文件循环逻辑提取至 `run_file_loop`（35 行）
- `FilterProcessor::from_feature` 函数体从 43 行精简为 23 行，两个构建辅助函数各 10 行
- fatal 早返回语义完整保留：`run_file_loop` 在 `has_fatal()` 时返回 `Err`，调用方通过 `?` 传播，跳过 `finalize`/`log_stats`

## Task Commits

1. **Task 1: 拆分 run_sequential，提取 run_file_loop（D-12）** - `c2499f1` (refactor)
2. **Task 2: 拆分 FilterProcessor::from_feature，提取 build_include_groups + build_exclude_groups（D-13）** - `1eada7a` (refactor)

**Plan metadata:** (待最终提交)

## Files Created/Modified

- `src/cli/run/mod.rs` — 新增 `fn run_file_loop`，`run_sequential` 精简为骨架
- `src/cli/run/filter_processor.rs` — 新增 `fn build_include_groups` + `fn build_exclude_groups`，`from_feature` 精简为调用委托

## Decisions Made

- `run_file_loop` 通过 `Result` 传播 fatal 错误，而非 `bool` 返回标志，保持与原早返回语义一致
- `build_include_groups` / `build_exclude_groups` 保持与原 `from_feature` 完全相同的字段顺序与 `FilterBuilder` 闭包，确保 11 项现有单元测试行为不变

## Deviations from Plan

None - plan executed exactly as written.

(注：`cargo fmt` 自动将单行 `Self { ... }` 展开为多行，属于格式化工具正常行为，非逻辑偏差)

## Issues Encountered

None

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- D-12、D-13 完成；顺序导出路径与 filter processor 已满足 STRUCT-01
- Plan 04 继续处理 `process_log_file`（D-11）

## Self-Check: PASSED

- FOUND: src/cli/run/mod.rs
- FOUND: src/cli/run/filter_processor.rs
- FOUND: c2499f1 (run_file_loop 提取)
- FOUND: 1eada7a (build_include/exclude_groups 提取)
- FOUND: 59-03-SUMMARY.md

---
*Phase: 59-cli-run-exporter-pipeline*
*Completed: 2026-06-02*
