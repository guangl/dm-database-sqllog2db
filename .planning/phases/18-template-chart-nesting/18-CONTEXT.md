# Phase 18: 模板 & 图表配置嵌套化 - Context

**Gathered:** 2026-05-17
**Status:** Ready for planning

<domain>
## Phase Boundary

将所有功能配置从 `[features.*]` 命名空间完全迁移到语义化的顶层子表，**彻底清空 `[features]`**：

- `[features.template_analysis]` → `[template]`
- `[features.charts]` → `[charts]`
- `[features.replace_parameters]` → `[replace_parameters]`
- `[features.filter.*]` → `[filter.*]`（Phase 17 已实现路径同步迁移）
- `[features.fields]` → `[output.fields]`

这是破坏性升级（no serde alias 兼容层），validate() 阶段检测旧 `[features.*]` 路径并给出清晰迁移提示。`init` 命令输出新格式。

</domain>

<decisions>
## Implementation Decisions

### 表路径层级

- **D-01:** 所有功能配置迁移到顶层，`[features]` 命名空间完全清空。新路径一览：
  - `[template]` — 模板分析（归一化 + 聚合）
  - `[charts]` — 图表生成
  - `[replace_parameters]` — SQL 参数替换（normalized_sql 列）
  - `[filter]` / `[filter.include]` / `[filter.exclude]` / `[filter.indicators]` / `[filter.sql]` / `[filter.record_sql]` — 过滤器（Phase 17 路径 `[features.filter.*]` 同步迁移）
  - `[output.fields]` — 字段投影列表

- **D-02:** Phase 17 的 `[features.filter.*]` 路径（已部分实现）需在本 Phase 修改为 `[filter.*]`。Phase 17-02-PLAN.md 尚未执行，可以直接按新路径设计。执行时需同步更新 Phase 17-01-PLAN.md 已产出的 struct 路径。

### 模板配置

- **D-03:** `[template]` 子表用单一 `enable = false`（对齐 `[filter]` 的 enable 字段名）。开启时同时启用：
  1. 热循环中的 `normalize_template` 调用（生成模板 key）
  2. `TemplateAggregator`（统计聚合）
  - 旧字段名为 `enabled`（template_analysis），新名统一为 `enable`。

- **D-04:** `[template]` 新增两个显式输出字段：
  - `output_csv_path = ""` — 模板统计 CSV 输出路径；不填则不生成
  - `output_sqlite_table = ""` — 模板统计 SQLite 表名；不填则不生成
  - **破坏性变化**：旧版 `enable=true` 自动生成 `*_templates.csv` 的行为消失。旧配置升级后若不填 `output_csv_path`，将不再生成模板统计文件。

### 向后兼容策略

- **D-05:** 破坡性升级，不实现 serde alias 兼容层。原因：
  1. `[features.*]` → 顶层的路径变化是 TOML 表名变化，不是字段名变化，无法用 alias 直接兼容
  2. 用户量小，文档说明足够（同 REQUIREMENTS.md 中的 "配置自动迁移 CLI" Out of Scope 决策）

- **D-06:** 旧路径处理：TOML 解析时会自动忽略 `[features]` 下的未知 key（不报错，但不起作用）。validate() 阶段主动检测：若 `[features]` 存在任何子表（通过 `_features_deprecated: Option<toml::Value>` 捕获），输出明确的迁移错误，列出每个需要修改的旧路径及其新路径对应关系。

### Claude's Discretion

- **`[replace_parameters]` 字段名**：当前 `enable = true`，迁移后字段名和默认值保持不变，只是表路径变化。若实现阶段发现命名有更好选择，可调整。
- **旧路径检测的具体实现**：`_features_deprecated` 捕获方案 vs 直接读取 `features.*` 路径检测。由规划/实现阶段根据 TOML crate 能力决定最简方案。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 配置核心结构

- `src/features/mod.rs:129-182` — `TemplateAnalysisConfig`、`ChartsConfig`、`FeaturesConfig` struct（Phase 18 主要重构对象）
- `src/config.rs` — `Config` struct 和 `validate()` 函数；validate 阶段需新增旧路径检测逻辑
- `src/features/filters.rs:43-112` — `FiltersFeature` struct（Phase 17 实现，需随 Phase 18 迁移路径）

### 热路径（不能破坏）

- `src/cli/run.rs:759-763` — `do_template` 判断逻辑（读取 `features.template_analysis.enabled`，Phase 18 需更新到新路径）
- `src/cli/run.rs:813-821` — charts 生成逻辑（读取 `features.charts`，Phase 18 需更新）

### init 命令（需输出新格式）

- `src/cli/init.rs:75-162` — `CONFIG_TEMPLATE_ZH` / `CONFIG_TEMPLATE_EN` 常量（Phase 18 完成后需更新为新格式）

### 需求规范

- `.planning/REQUIREMENTS.md` — CONFIG-03（[template] 子表）、CONFIG-04（[charts] 子表）
- `.planning/phases/17-filter-nesting/17-CONTEXT.md` — Phase 17 的 filter 路径设计（Phase 18 需将其从 `[features.filter.*]` 迁至 `[filter.*]`）

### 现有配置示例（迁移基准）

- `config.toml` — 项目根目录当前配置，含旧格式 `[features.*]`；Phase 18 完成后可更新为新格式（非必须）

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- `TemplateAnalysisConfig` (`src/features/mod.rs:129`) — 目前只有 `enabled: bool`，Phase 18 重命名字段、增加 output_* 字段
- `ChartsConfig` (`src/features/mod.rs:137`) — 含 `output_dir`, `top_n`, `frequency_bar`, `latency_hist`, `trend_line`, `user_pie`；路径迁移，字段不变
- `FeaturesConfig` (`src/features/mod.rs:173`) — 顶层功能配置聚合 struct；Phase 18 后此 struct 可能退化为空或废弃
- `validate()` / `validate_and_compile()` in `src/config.rs` — Phase 18 需在此处添加旧路径检测 + 新路径校验

### Established Patterns

- `#[serde(default)]` — 所有子表字段均用 `default`，缺省时不报错
- `Option<T>` 表示"未配置该功能"（`None = 禁用`）——`[template]`/`[charts]` 保持此模式
- validate() 阶段跨字段依赖检查（如 charts 依赖 template_analysis.enabled）：Phase 18 依赖变为 `[template].enable = true`

### Integration Points

- `src/cli/run.rs:759` — `do_template` 计算，需从 `config.template.enable` 读取
- `src/cli/run.rs:813, 920` — `config.charts` 引用，需更新到 `config.charts`（顶层）
- `src/cli/run.rs:934` — `exporter_manager.write_template_stats(stats, None)` — Phase 18 需将 `None` 改为从 `config.template.output_csv_path` / `output_sqlite_table` 传入路径
- `src/cli/init.rs` — init 命令生成的配置模板需全部更新为新格式

</code_context>

<specifics>
## Specific Ideas

**新格式目标（init 命令输出）：**

```toml
[replace_parameters]
enable = true

[template]
enable = false
# output_csv_path = "outputs/templates.csv"     # 不填则不生成
# output_sqlite_table = "sql_templates"          # 不填则不生成

[filter]
enable = false

[filter.include]
# users = ["SYSDBA"]
# ips = ["127.0.0.1"]

[filter.exclude]
# users = ["guest"]

[filter.indicators]
# min_runtime_ms = 1000

[filter.sql]
# includes = ["FROM USER_TABLES"]

[charts]
output_dir = "charts/"
top_n = 10
```

**旧格式检测错误示例（validate() 输出）：**

```
配置格式已升级，请迁移以下字段：
  [features.template_analysis] → [template]
  [features.charts]            → [charts]
  [features.replace_parameters] → [replace_parameters]
  [features.filter.*]          → [filter.*]
  [features.fields]            → [output.fields]
```

</specifics>

<deferred>
## Deferred Ideas

- 调研 dm-database-parser-sqllog 1.0.0 新特性 — 已在 Phase 6 关闭（PERF-07），无关
- 配置自动迁移 CLI — Out of Scope（REQUIREMENTS.md 明确排除）
- `[features]` 完全移除后的代码结构整理 — Phase 19 范围（REFACTOR-01）

</deferred>

---

*Phase: 18-模板 & 图表配置嵌套化*
*Context gathered: 2026-05-17*
