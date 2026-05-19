---
phase: 30
plan: 03
subsystem: tests
tags: ["template-analysis", "test-cleanup", "verification"]
requires: ["30-01", "30-02"]
provides: ["RM-05"]
affects: ["tests"]
tech-stack:
  added: []
  patterns: []
key-files:
  created: []
  modified:
    - src/cli/run/tests.rs
decisions:
  - id: D-01
    description: "保留 test_no_template_stats_when_disabled — 验证伴生文件不生成，行为仍然正确"
  - id: D-02
    description: "大部分 Plan 30-03 测试已在 Plan 30-01/02 中提前清理 — 删除依赖已消失类型的测试是编译必要条件"
metrics:
  duration: "~5 min"
  completed: "2026-05-20"
---

# Phase 30 Plan 03: 测试清理 + 编译验证

清理 run/tests.rs 中的剩余模板引用，验证全链路编译、测试、clippy、fmt 通过。

## Deviations from Plan

### Pre-completed Items

**1. 测试清理已由 Plan 30-01 和 30-02 完成**
- test_template_stats_enabled_end_to_end_sequential — 30-01 移除
- test_e2e_template_normalization — 30-01 移除
- 4 个 exporter/csv 测试 — 30-02 移除
- 6 个 exporter/sqlite 测试 — 30-02 移除
- 4 个 exporter 测试 — 30-02 移除

### Plan 30-03 执行项
- 从 test_parallel_merge_consistent 中移除 [template] 配置
- 更新 test_aggregator_disabled_none_path 注释

## Known Stubs

None.

## Threat Flags

No new security-relevant surface introduced.

## Self-Check: PASSED

- All tests pass (302 unit + 39 integration)
- `cargo clippy --all-targets -- -D warnings` passes
- `cargo fmt --check` passes
