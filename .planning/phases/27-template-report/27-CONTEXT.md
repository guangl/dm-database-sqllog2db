# Phase 27: 模板报告独立输出 - Context

**Gathered:** 2026-05-19
**Status:** Ready for planning

## Phase Boundary

模板统计结果输出为独立文件：CSV 摘要文件（`*_templates.csv`）和 SQLite 报告文件（`*_templates.db`），在 `run` 完成后自动生成。不涉及新的 CLI 子命令。

**In scope:** TMPL-03, TMPL-03b
**Out of scope:** JSON 输出格式；新的 CLI 子命令；模板分析算法变更

## Implementation Decisions

### 触发方式
- **D-01:** 自动随 `run` 生成 — 只要配置了模板分析（`pipeline.aggregator` 启用），`run` 完成后自动生成 `*_templates.csv` 和 `*_templates.db`

### 输出格式
- **D-02:** CSV 报告字段复用 `TemplateStats` 结构 — template_key, count, avg_us, min_us, max_us, p50_us, p95_us, p99_us, first_seen, last_seen
- **D-03:** SQLite 采用多表范式化设计 — template_keys（模板标识）+ template_stats（统计值）+ latency_percentiles（延迟百分位），三表关联

### 文件命名
- **D-04:** 自动派生 — 从主输出文件名派生：输出为 `out.csv` → `out_templates.csv` / `out_templates.db`；SQLite 模式下类似处理

### 配置模型
- **D-05:** 新增 `[templates]` 配置段 — 可选启用/禁用独立报告输出（与现有 `pipeline.aggregator` 模板分析功能解耦，但默认跟随启用）

### Claude's Discretion
- SQLite 三表的具体 schema（字段类型、主键、外键、索引）
- `[templates]` 配置段的具体字段设计
- 文件命名派生逻辑在单文件/多文件模式下的行为
- CSV 分隔符和编码细节

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求文档
- `.planning/REQUIREMENTS.md` — TMPL-03（CSV）、TMPL-03b（SQLite）的权威描述
- `.planning/ROADMAP.md` — Phase 27 的范围（独立于文档阶段）

### 核心代码
- `src/pipeline/aggregator.rs` — `TemplateAggregator` 和 `TemplateStats` 结构，当前模板统计的完整实现，含 `finalize()` 方法
- `src/exporter/mod.rs` — `Exporter` trait 定义（`initialize` → `export_one_preparsed` → `finalize` 生命周期）
- `src/exporter/csv/companion.rs` — 现有 companion CSV 写入模式（参考实现）
- `src/exporter/csv/mod.rs` — CSV 导出器实现
- `src/exporter/sqlite/mod.rs` — SQLite 导出器实现
- `src/config/exporter.rs` — 导出器相关配置

### 项目规范
- `.planning/PROJECT.md` — Key Decisions 含 "TMPL-03 为独立 CSV 报告（非 JSON）"、"TMPL-03b 为独立 SQLite 报告"
- `.planning/codebase/ARCHITECTURE.md` — Exporter trait、ExporterKind enum 的设计模式
- `.planning/codebase/CONVENTIONS.md` — 错误处理、命名、配置模型风格

## Existing Code Insights

### Reusable Assets
- `TemplateAggregator::finalize()` — 已有完整的模板统计输出流程，直接调用即可
- `TemplateStats` — 已实现 `serde::Serialize`，可直接用于 CSV 序列化
- `CompiledProcessor` — 当前 `process_with_meta()` 中 `Option<&mut TemplateAggregator>` 模式，热循环中传入聚合器

### Established Patterns
- `Exporter` trait 三阶段生命周期：`initialize()` → `export_*()` → `finalize()`
- `BufWriter` + `itoa` 零分配 CSV 格式化（16MB 缓冲）
- 配置 struct 命名：`[section]` TOML → `SectionConfig` Rust struct → `from_config()` 构造
- CSV 导出优先于 SQLite 的优先级选择模式

### Integration Points
- `src/cli/run/mod.rs` — `handle_run()` 主循环，在 `exporter_manager.finalize()` 之后调用模板报告生成
- `src/pipeline/aggregator.rs` — `TemplateAggregator` 目前通过 `Option<&mut TemplateAggregator>` 侧路径接入热循环，`finalize()` 后 stats 可用
- 现有双路输出（CSV + SQLite 同时）的模式可供参考

## Specific Ideas

- 模板报告生成可作成 `TemplateReporter` 独立 struct，不耦合到 `ExporterKind` 枚举（避免枚举变体膨胀）
- CSV 模板报告复用现有的 `itoa` + `BufWriter` 零分配模式
- SQLite 模板报告复用 `rusqlite` 批量插入模式（与现有 SQLite exporter 一致）

## Deferred Ideas

None — discussion stayed within phase scope

---

*Phase: 27-模板报告独立输出*
*Context gathered: 2026-05-19*
