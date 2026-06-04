---
phase: 66-compat
plan: 01
subsystem: testing
tags: [integration-test, csv, parallel, rayon, compat]

# Dependency graph
requires:
  - phase: 64-csv
    provides: process_csv_parallel implementation (parallel.rs)
  - phase: 65-parity
    provides: sqlite parallel parity verification
provides:
  - 3 new integration tests: test_parallel_csv_content_matches_sequential,
    test_parallel_csv_filter_matches_sequential, test_init_no_parallel_fields
  - COMPAT-01 verification (72 integration tests pass, no regressions)
  - COMPAT-02 parallel CSV content consistency assertion
  - COMPAT-03 config format stability assertion
affects: [future parallel feature changes, init template changes]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - sorted row-set comparison for parallel vs sequential output correctness

key-files:
  created: []
  modified:
    - tests/integration.rs

key-decisions:
  - "排序后行集合对比（sorted set comparison）而非字节级对比，因为并行路径文件间行顺序不确定"
  - "每个文件单独运行 handle_run 收集顺序基线，避免 append 模式的复杂性"
  - "test_init_no_parallel_fields 以轻量 grep 断言替代全文件 diff，维护成本低"

patterns-established:
  - "Pattern: 并行路径测试使用显式文件路径列表（Vec<String>）而非 glob 目录，确保触发并行路径"

requirements-completed: [COMPAT-01, COMPAT-02, COMPAT-03]

# Metrics
duration: 15min
completed: 2026-06-04
---

# Phase 66 Plan 01: Compat Summary

**3 个 COMPAT 集成测试验证并行 CSV 路径输出与顺序路径一致，config.toml 格式保持 v1.16 基线**

## Performance

- **Duration:** 15 min
- **Started:** 2026-06-04T00:00:00Z
- **Completed:** 2026-06-04T00:15:00Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- 新增 `test_parallel_csv_content_matches_sequential`：2 文件 × 20 条记录，无过滤器，排序后行集合断言并行与顺序输出相等
- 新增 `test_parallel_csv_filter_matches_sequential`：2 文件，启用 `include.users = ["TESTUSER"]` 过滤器，验证过滤一致性
- 新增 `test_init_no_parallel_fields`：断言 init 模板不含 `parallel`/`jobs` 字段，确认 config 格式稳定
- 全量 72 个集成测试 + 1 个 jemalloc 测试全部通过（无回归）

## Task Commits

1. **Task 1: 新增 COMPAT 集成测试** - `66bea3c` (test)

## Files Created/Modified

- `tests/integration.rs` - 追加 3 个 Phase 66 兼容性验证集成测试（COMPAT-01/02/03）

## Decisions Made

- 使用排序后行集合对比策略（sorted set comparison），而非字节级对比，因为并行路径的文件处理顺序由 rayon work-stealing 决定，不确定
- 顺序基线通过每个文件单独构建 `Config`（单文件 `inputs`）运行 `handle_run` 来生成，避免 append 模式带来的多余 header 处理复杂性
- `test_init_no_parallel_fields` 使用轻量 grep 断言（`!content.contains("parallel")`），维护成本远低于全文件 diff 基线对比

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- clippy `doc_markdown` 警告：注释中 `handle_run` 需加反引号，`doc_link_with_quotes` 警告：`["TESTUSER"]` 需改为反引号包裹，已即时修复
- `cargo fmt` 格式化了一处链式调用断行，已应用

## Next Phase Readiness

- Phase 66 所有兼容性验证完成，v1.17 并行 CSV 路径已通过完整测试矩阵
- COMPAT-01/02/03 全部满足，milestone v1.17 可标记完成

---
*Phase: 66-compat*
*Completed: 2026-06-04*
