---
phase: 30
plan: 01
subsystem: config
tags: ["template-analysis", "config-cleanup"]
requires: []
provides: ["RM-05"]
affects: ["pipeline", "cli", "tests"]
tech-stack:
  added: []
  patterns: ["#[allow(dead_code)] for cross-plan staging"]
key-files:
  created: []
  modified:
    - src/config/mod.rs
    - src/config/apply_one.rs
    - src/config/validate.rs
    - src/pipeline/mod.rs
    - src/pipeline/aggregator.rs
    - src/pipeline/template_reporter.rs
    - src/cli/init.rs
    - src/cli/show_config.rs
    - src/cli/run/mod.rs
    - src/cli/run/tests.rs
    - tests/integration.rs
decisions:
  - id: D-01
    description: "Keep TemplateConfig/TemplateReportConfig types with #[allow(dead_code)] — Plan 30-02 将完全移除运行时代码"
  - id: D-02
    description: "Keep hdrhistogram dependency — aggregator.rs 仍使用 hdrhistogram::Histogram，Plan 30-02 处理"
  - id: D-03
    description: "Keep aggregator/template_reporter 模块声明 — Plan 30-02 范围"
metrics:
  duration: "~20 min"
  completed: "2026-05-20"
---

# Phase 30 Plan 01: 配置层清理

移除 [template] 配置段从 Config 结构体、init 模板、apply_one、show_config 和相关测试中。模板类型保留并标注 #[allow(dead_code)]，由 Plan 30-02 负责运行时代码和文件删除。

## Deviations from Plan

### Auto-fixed Issues

**1. template 字段移除导致 cli/run/mod.rs 编译失败**
- 移除 `pub template: Option<TemplateConfig>` 后，`template_report_enabled()` 和 `write_template_reports()` 引用失效
- 修复: 移除这两个函数，设置 `do_template = false`

**2. #[allow(dead_code)] 标注**
- TemplateConfig/TemplateReportConfig/TemplateAggregator/TemplateReporter 在移除 config 字段后变 dead code
- 添加 #[allow(dead_code)] 标注，Plan 30-02 将删除这些类型

**3. 集成测试更新**
- 移除 test_e2e_template_normalization (使用已删除的 cfg.template 字段)
- 移除 init 测试中的 [template] 断言行
- 移除 validate 测试中的 [pipeline.template_analysis] → [template] 断言行
- 移除 test_template_stats_enabled_end_to_end_sequential (依赖模板报告)

**4. 清理未使用导入**
- 移除 src/pipeline/mod.rs 中的 `use std::path::PathBuf`
- 移除 src/cli/run/mod.rs 中的 `use std::path::PathBuf`

## Known Stubs

None.

## Threat Flags

No new security-relevant surface introduced.

## Self-Check: PASSED

- All 330 unit tests pass
- All 39 integration tests pass
- `cargo clippy --all-targets -- -D warnings` passes with no warnings
- `cargo fmt --check` passes
- Commit ccdf6be verified in git log
