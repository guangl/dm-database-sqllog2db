---
phase: 30
plan: 02
subsystem: runtime
tags: ["template-analysis", "runtime-cleanup"]
requires: ["30-01"]
provides: ["RM-05"]
affects: ["cli/run", "exporter", "pipeline"]
tech-stack:
  added: []
  patterns: ["#[allow(dead_code)] for normalize_template helpers"]
key-files:
  created: []
  modified:
    - src/cli/run/mod.rs
    - src/cli/run/processor.rs
    - src/cli/run/parallel.rs
    - src/exporter/mod.rs
    - src/exporter/csv/mod.rs
    - src/exporter/sqlite/mod.rs
  deleted:
    - src/pipeline/aggregator.rs
    - src/pipeline/template_reporter.rs
    - src/exporter/csv/companion.rs
decisions:
  - id: D-01
    description: "删除 aggregator.rs 后 TemplateStats 类型消失，测试文件必须同步清理"
  - id: D-02
    description: "normalize_template 函数保留并标注 #[allow(dead_code)] — 热循环中最后调用点被移除"
  - id: D-03
    description: "提前清理大部分 Plan 30-03 测试 — 编译依赖已消失的类型无法延迟"
metrics:
  duration: "~30 min"
  completed: "2026-05-20"
---

# Phase 30 Plan 02: 运行时代码清理

删除 aggregator.rs、template_reporter.rs、companion.rs 三个文件。从 cli/run 热循环中移除所有模板聚合/报告代码。从 Exporter 全部层移除 write_template_stats。

## Deviations from Plan

### Auto-fixed Issues

**1. 清理测试文件中的 TemplateStats 引用**
- 删除 aggregator.rs 后 TemplateStats 类型消失，exporter/csv/tests.rs、sqlite/tests.rs、exporter/tests.rs 中的测试无法编译
- 修复: 提前清理 Plan 30-03 范围内的测试代码（约 400 行）

**2. normalize_template 死代码**
- 模板聚合块（processor.rs）是 normalize_template 的最后调用点
- 移除后 normalize_template 及其 14 个私有辅助函数变 dead_code
- 添加 #[allow(dead_code)] 到 normalize_template

## Known Stubs

None.

## Threat Flags

No new security-relevant surface introduced.

## Self-Check: PASSED

- All unit tests pass (302 tests)
- All 39 integration tests pass
- `cargo clippy --all-targets -- -D warnings` passes
- `cargo fmt --check` passes
