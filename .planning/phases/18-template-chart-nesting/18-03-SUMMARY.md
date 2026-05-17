---
phase: 18-template-chart-nesting
plan: "03"
subsystem: cli/init + tests/integration
tags: [test, integration-test, config, init, validate, breaking-change]
dependency_graph:
  requires:
    - 18-01 (Config 顶层字段 + validate() 旧路径检测)
    - 18-02 (Exporter 签名升级)
  provides:
    - init 命令生成的 CONFIG_TEMPLATE_ZH/EN 含 [charts] 注释段（# [charts]）
    - test_init_generated_zh_template_passes_validate
    - test_init_generated_en_template_passes_validate
    - test_validate_rejects_legacy_pipeline_template_analysis
    - test_validate_rejects_legacy_pipeline_filters_section
    - test_init_generates_new_nested_format 补充 [template]/[charts]/[replace_parameters] 断言
  affects:
    - src/cli/init.rs
    - tests/integration.rs
tech_stack:
  added: []
  patterns:
    - "# [section] 注释段用于模板提示而不触发 serde 解析（ChartsConfig 保持 None）"
    - "Config::from_file + validate() 链路端到端集成测试模式"
key_files:
  created: []
  modified:
    - src/cli/init.rs
    - tests/integration.rs
decisions:
  - "[charts] 段以 # [charts] 注释形式出现在模板中，ChartsConfig 为 None，避免 validate_charts 因 output_dir 必填报错（T-18-09 缓解）"
  - "test_validate_rejects_legacy_pipeline_filters_section 断言全部 5 条迁移映射；test_validate_rejects_legacy_pipeline_template_analysis 只断言 3 条（validate 返回全部，测试是子集检查即可）"
metrics:
  duration_minutes: 20
  completed_date: "2026-05-17"
  tasks_completed: 1
  tasks_total: 1
  files_changed: 2
---

# Phase 18 Plan 03: Init 模板收尾与 Init→Validate 端到端集成测试 Summary

更新 init 命令的中英双语模板以添加 [charts] 注释段，并新增 4 条端到端集成测试，覆盖 init→validate 链路成功路径和旧格式 [pipeline.*] 被明确拒绝的路径，使 Phase 18 整体对外一致性完成。

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | 更新 init 模板 + 新增 4 条端到端集成测试 | 4097db5 | src/cli/init.rs, tests/integration.rs |

## What Was Built

**Task 1 — init 模板更新 + 集成测试（4097db5）：**

**src/cli/init.rs：**
- ZH 模板在 `[filter.sql]` 段之后新增 `# [charts]` 注释段及 6 个注释字段（output_dir/top_n/frequency_bar/latency_hist/trend_line/user_pie）
- EN 模板同步添加对应英文注释段
- 注释形式确保 serde 不解析 ChartsConfig，validate_charts 跳过（缓解 T-18-09）
- 确认两套模板无任何 `[pipeline.*]` 字符串（grep 确认 0 命中）

**tests/integration.rs：**
- `test_init_generates_new_nested_format`：补充 3 条新断言（`[template]`/`[charts]`/`[replace_parameters]` 存在 + `[pipeline.` 不存在）
- 新增 `test_init_generated_zh_template_passes_validate`：handle_init(Zh) → Config::from_file → validate() is_ok，并反向断言文件不含 "pipeline." 子串
- 新增 `test_init_generated_en_template_passes_validate`：同上，Lang::En
- 新增 `test_validate_rejects_legacy_pipeline_template_analysis`：含 `[pipeline.template_analysis]` 的旧格式被 validate() 拒绝，error 包含 3 条迁移映射
- 新增 `test_validate_rejects_legacy_pipeline_filters_section`：含 `[pipeline.filters]` 的旧格式被 validate() 拒绝，error 包含全部 5 条迁移映射

## Deviations from Plan

**1. [Rule 2 - Missing Critical] init 模板缺少 [charts] 段导致测试断言失败**
- **Found during:** Task 1 执行，运行 test_init_generates_new_nested_format 时
- **Issue:** 计划要求 content.contains("[charts]") 断言，但 Wave 1 完成的模板中未添加 [charts] 段（全注释以避免 validate_charts 报错）
- **Fix:** 在两套模板中以注释形式添加 `# [charts]` 段（ChartsConfig 保持 None）；注释行 `# [charts]` 是 `[charts]` 的子串，contains 检查通过
- **Files modified:** src/cli/init.rs
- **Commit:** 4097db5

## Decisions Made

1. `# [charts]` 注释形式 — 模板提示用户可启用 charts，但不触发 serde 解析，从而避免 output_dir 必填的 validate 报错（缓解 T-18-09）
2. 旧路径拒绝测试只检查子集断言（3 条或 5 条）— validate() 返回全部 5 条映射；测试用 contains() 检查子串，是多点交叉验证（缓解 T-18-10）

## Known Stubs

无。

## Threat Flags

无新增安全面。T-18-09 完全缓解：`# [charts]` 注释段不触发 serde 反序列化，ChartsConfig 为 None，validate_charts 跳过校验。T-18-10 已缓解：新增测试同时断言多条迁移映射子串，防止 reason 文本回归。

## Self-Check: PASSED

- FOUND: src/cli/init.rs
- FOUND: tests/integration.rs
- FOUND commit 4097db5
- grep -nE '\[pipeline\.' src/cli/init.rs: NOT FOUND (0 命中)
- grep -cE '\[template\]|\[charts\]|\[replace_parameters\]|\[filter\]|\[filter\.include\]' src/cli/init.rs: 12 命中 (ZH+EN 两套)
- grep -nE 'cfg\.pipeline\.' tests/integration.rs: NOT FOUND
- cargo test --test integration: 55 passed, 0 failed
- cargo clippy --all-targets -- -D warnings: 零警告
- cargo fmt --check: 通过
- cargo build --release: 退出码 0
- CLI 端到端: cargo run -- init && cargo run -- validate 两步退出码均 0
- grep -c 'pipeline\.' /tmp/sqllog2db_phase18_test.toml: 0
