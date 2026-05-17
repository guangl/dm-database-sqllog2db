# Phase 18: 模板 & 图表配置嵌套化 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-17
**Phase:** 18-模板 & 图表配置嵌套化
**Areas discussed:** 表路径层级, 模板开关粒度, output_* 字段语义, 旧格式向后兼容

---

## 表路径层级

| Option | Description | Selected |
|--------|-------------|----------|
| `[features.template]` / `[features.charts]` | 保持与 Phase 17 一致，所有功能开关都在 [features] 下。与 replace_parameters 并列，风格统一。 | |
| `[template]` / `[charts]`（顶层） | 模板分析和图表是独立展示功能，放顶层更清晰。ROADMAP 中就是写的不带 features 前缀。 | ✓ |

**User's choice:** 顶层表

**Notes:** 进而扩展到完全清空 `[features]`：

| Option | Description | Selected |
|--------|-------------|----------|
| 不动 `replace_parameters`，仅迁 template/charts | Phase 18 范围可控 | |
| 一并迁到顶层 | 一次性清理 `[features]` 命名空间 | ✓ |

| Option | Description | Selected |
|--------|-------------|----------|
| 保留 `[features.filter.*]` 和 `[features.fields]` 不动 | filter 是 Phase 17 设计，不在 Phase 18 范围 | |
| 一并迁，强制清空 `[features]` | 彻底清理，`[features]` 退场 | ✓ |

| Option | Description | Selected |
|--------|-------------|----------|
| 不动 filter（保留 features 前缀） | Phase 17 路径是已定设计 | |
| 一并迁，重写 Phase 17 的路径（Phase 17-02-PLAN 尚未执行） | 趁 Phase 17-02 未执行，统一迁到顶层 | ✓ |

---

## 模板开关粒度

| Option | Description | Selected |
|--------|-------------|----------|
| 拆分为 `enable_normalization` + `enable_aggregation` | 用户可仅开归一化而不运行聚合统计 | 初始选 ✓，后反悔 |
| 保持单一 `enable` | 开就全开，配置更简单 | ✓（最终决策） |

**User's choice:** 单一 `enable = false`

**Notes:** 用户一开始选拆分，追问"仅开 enable_normalization 语义是否清晰"时认为拆得太细，没有实际用途（单独归一化而不聚合对用户无意义），最终改为单一开关。字段名用 `enable`（而非旧版的 `enabled`）对齐 `[filter].enable`。

---

## output_* 字段语义

| Option | Description | Selected |
|--------|-------------|----------|
| 无新增 output 字段，保持现有隐式行为 | ROADMAP 中 output_* 只是占位词，不需要映射到实际字段 | |
| 新增显式字段（output_csv_path / output_sqlite_table） | 用户可自定义模板统计文件路径或表名 | ✓ |
| 仅 output_csv = true/false 开关 | 控制是否生成，不改路径 | |

**User's choice:** 新增显式 `output_csv_path` 和 `output_sqlite_table`

**Notes:**

- 默认行为：不填 = 不生成（须显式指定才输出模板统计文件）
- 这是破坏性变化：旧版 `enable=true` 自动生成 `*_templates.csv` 的行为消失
- 旧配置升级后若不填 `output_csv_path`，不再自动生成模板统计文件
- 用户明确接受此破坡变化

---

## 旧格式向后兼容

| Option | Description | Selected |
|--------|-------------|----------|
| 完全兼容，旧路径能解析 | 类似 Phase 17 的 serde alias，旧 `[features.*]` 路径仍可读 | |
| 不兼容，破坡升级 | 不实现兼容层，实现更简单 | ✓ |

**User's choice:** 破坡升级

**Notes:**

- 旧路径检测：TOML 默认忽略未知 key（`[features.*]` 会被静默忽略，不报错但不起作用）
- validate() 阶段主动检测旧 `[features]` 路径（通过 `_features_deprecated: Option<toml::Value>` 或类似方案），输出清晰迁移错误，列出每条旧路径的新对应路径
- `[features.fields]` 迁移到 `[output.fields]`（与导出配置放在一起更直观）

---

## Claude's Discretion

- `[replace_parameters]` 字段名：字段内容不变，只改表路径，实现阶段可灵活处理
- 旧路径检测的具体实现方式（捕获 struct vs 直接检测）：由规划/实现阶段根据 TOML crate 能力决定最简方案

## Deferred Ideas

- 调研 dm-database-parser-sqllog 1.0.0 — 已在 Phase 6 关闭，与 Phase 18 无关（讨论前过滤，不需要讨论）
- 配置自动迁移 CLI — Out of Scope（REQUIREMENTS.md 明确排除）
- `[features]` 移除后的代码结构清理 — Phase 19 范围
