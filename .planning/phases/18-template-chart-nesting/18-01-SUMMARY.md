---
phase: 18-template-chart-nesting
plan: "01"
subsystem: config/pipeline
tags: [refactor, breaking-change, config, pipeline, toml]
dependency_graph:
  requires: []
  provides:
    - TemplateConfig struct (enable/output_csv_path/output_sqlite_table)
    - OutputConfig struct with field_mask/ordered_field_indices
    - Config top-level fields (replace_parameters/template/filter/charts/output)
    - legacy [pipeline] detection via pipeline_deprecated field
  affects:
    - src/cli/run.rs
    - src/cli/stats.rs
    - src/cli/validate.rs
    - src/cli/show_config.rs
    - src/cli/init.rs
    - src/exporter/mod.rs
    - src/main.rs
    - tests/integration.rs
tech_stack:
  added: []
  patterns:
    - Option<toml::Value> catch-all field for deprecated section detection
    - "#[doc(hidden)] pub field for cross-crate accessibility"
key_files:
  created: []
  modified:
    - src/pipeline/mod.rs
    - src/pipeline/filters.rs
    - src/config/mod.rs
    - src/exporter/mod.rs
    - src/cli/validate.rs
    - src/cli/stats.rs
    - src/cli/show_config.rs
    - src/cli/run.rs
    - src/cli/init.rs
    - src/main.rs
    - tests/integration.rs
decisions:
  - "Use pub + #[doc(hidden)] for pipeline_deprecated field to allow access from integration tests (external crate)"
  - "Breaking upgrade with no serde alias compatibility — validate() rejects [pipeline] immediately"
  - "TemplateConfig uses 'enable' not 'enabled' to align with NormalizeConfig and FiltersFeature naming"
metrics:
  duration_minutes: 90
  completed_date: "2026-05-17"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 11
---

# Phase 18 Plan 01: Config Pipeline Namespace Elimination Summary

将所有 `[pipeline.*]` TOML 子表提升为 Config 顶层字段，彻底清空 `[pipeline]` 命名空间；新增 TemplateConfig 与 OutputConfig struct，旧路径通过 `pipeline_deprecated` 字段在 validate() 阶段被明确拒绝并输出 5 条迁移映射。

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | 重构 pipeline/mod.rs — TemplateConfig + OutputConfig | c39e8cc | src/pipeline/mod.rs, src/pipeline/filters.rs |
| 2 | 重构 config/mod.rs — 顶层字段 + 旧路径检测 | 7d895c4 | src/config/mod.rs + 9 downstream files |

## What Was Built

**Task 1 — pipeline/mod.rs:**
- 删除 `TemplateAnalysisConfig`（含 `enabled: bool`）和 `PipelineConfig`（含 field_mask/ordered_field_indices）
- 新增 `TemplateConfig { enable, output_csv_path, output_sqlite_table }`，实现 D-03/D-04
- 新增 `OutputConfig { fields: Option<Vec<String>> }` + `field_mask()` + `ordered_field_indices()` 方法（从 PipelineConfig 迁移）
- 更新 filters.rs 中全部 `pipeline.filters.*` 错误字段路径为 `filter.*`
- 补充 6 个新单元测试：`test_template_config_*` 和 `test_output_config_*`

**Task 2 — config/mod.rs 与下游:**
- Config struct 拆分 `pub pipeline: PipelineConfig` 为 5 个顶层 `Option<T>` 字段
- 新增 `#[doc(hidden)] pub pipeline_deprecated: Option<toml::Value>` 捕获旧 `[pipeline]` 段
- validate() 优先检测旧路径，返回含 5 条迁移映射的错误消息
- 私有方法重命名：validate_pipeline_filters → validate_filter，validate_pipeline_fields → validate_output_fields，validate_pipeline_charts → validate_charts
- apply_one() 支持 12 个新路径键，拒绝旧 pipeline.* 路径
- init.rs 模板同步升级为新格式（不再生成 [pipeline.*]）
- 全部 431 个 lib 测试 + 51 个集成测试通过

## Deviations from Plan

**1. [Rule 3 - Blocking] Task 1 删除 PipelineConfig 导致全局编译失败**
- **Found during:** Task 1 执行后
- **Issue:** 删除 PipelineConfig 后，src/config/mod.rs 及所有下游文件立即编译失败，无法独立运行 `cargo test --lib pipeline::`
- **Fix:** 将 Task 2 及所有下游文件（exporter、cli/*、main.rs、tests/integration.rs）的迁移合并处理，作为单次原子变更完成
- **Files modified:** 所有 Task 2 列出的文件
- **Commit:** 7d895c4

**2. [Rule 2 - Missing Critical] pipeline_deprecated 字段需 pub 可见性**
- **Found during:** Task 2 集成测试编译
- **Issue:** `_pipeline_deprecated: Option<toml::Value>` 设为 `pub(crate)` 后，integration tests（外部 crate）无法使用 `..Default::default()` struct update 语法
- **Fix:** 改为 `#[doc(hidden)] pub pipeline_deprecated: Option<toml::Value>`（pub 但文档隐藏）
- **Files modified:** src/config/mod.rs
- **Commit:** 7d895c4

**3. [Rule 1 - Bug] init.rs 模板仍生成旧格式导致集成测试失败**
- **Found during:** `test_init_generates_new_nested_format` 集成测试
- **Issue:** init.rs 的中英文 TOML 模板仍包含 `[pipeline.*]` 段，validate() 调用会直接返回错误
- **Fix:** 更新 init.rs 两套模板为新格式（[filter]、[replace_parameters]、[template]、[output.fields]）
- **Files modified:** src/cli/init.rs
- **Commit:** 7d895c4

## Decisions Made

1. `pub + #[doc(hidden)]` 模式用于 `pipeline_deprecated` 字段 — 技术上需要 pub 以支持跨 crate struct update 语法，`#[doc(hidden)]` 隐藏文档使其不出现在公开 API 中
2. 破坏性升级无 serde alias 兼容层 — 符合 CONTEXT.md D-05，validate() 明确拒绝旧路径并提供完整迁移指引
3. `TemplateConfig.enable` 字段名（非 `enabled`）— 与 NormalizeConfig、FiltersFeature 命名对齐，符合 D-03

## Known Stubs

无 — 所有字段均有真实 serde 绑定，无占位数据。

## Threat Flags

无新增安全面。T-18-01 已完全缓解：`pipeline_deprecated` 字段捕获旧 `[pipeline]` 段，validate() 返回明确迁移错误，不会静默接受旧配置。

## Self-Check: PASSED

- FOUND: src/pipeline/mod.rs
- FOUND: src/config/mod.rs
- FOUND: 18-01-SUMMARY.md
- FOUND commit c39e8cc (Task 1)
- FOUND commit 7d895c4 (Task 2)
- `pub struct TemplateConfig` at line 131 of pipeline/mod.rs
- `pub struct OutputConfig` at line 181 of pipeline/mod.rs
- `TemplateAnalysisConfig` occurrence count = 0 (correctly deleted)
- 5 top-level fields confirmed in Config struct (replace_parameters/template/filter/charts/output)
