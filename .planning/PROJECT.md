# sqllog2db — 达梦 SQL 日志解析工具

## Current State

sqllog2db 完成七个里程碑迭代（v1.0–v1.6），具备完整的 SQL 模板分析、SVG 可视化、嵌套配置模型、模块化代码架构，全中文项目文档和 GitHub Pages mdBook 多页文档站。v1.7 进入精简阶段。Phase 28–31 完成，移除了 SVG 图表、self-update、Shell 补全、Man page、stats、digest 和断点续传。

## Current Milestone: v1.7 项目精简

**Goal:** 移除低频和非核心功能，减少依赖体积和代码复杂度，保留核心解析导出能力。

**Target features:**
- ✅ 移除 SVG 图表模块（charts/*, plotters）— Phase 28
- ✅ 移除 self-update 自更新（update.rs, self_update, reqwest, rustls）— Phase 28
- ✅ 移除 Shell 补全 + Man page（clap_complete, clap_mangen）— Phase 28
- ✅ 移除 stats 统计命令（cli/stats.rs）— Phase 29
- ✅ 移除 digest 摘要命令（cli/digest.rs, pipeline/fingerprint.rs, serde_json）— Phase 29
- ✅ 移除断点续传（resume.rs, [resume] 配置）— Phase 31
- 移除模板分析+报告（aggregator, template_reporter, hdrhistogram）

## Previous Milestones

- ✅ **v1.6** (2026-05-19) — 文档中文化 & 延后需求补全（Phases 24–27）
- ✅ **v1.5** (2026-05-19) — 文档完善 & 项目展示（Phases 21–23）
- ✅ **v1.4** (2026-05-18) — 代码重构 & 质量深化（Phases 17–20）
- ✅ **v1.3** (2026-05-17) — SQL 模板分析 & 可视化（Phases 12–16）
- ✅ **v1.2** (2026-05-15) — 质量强化 & 性能深化（Phases 7–11）
- ✅ **v1.1** (2026-05-10) — 性能优化（Phases 3–6）
- ✅ **v1.0** (2026-04-18) — 增强 SQL 内容过滤与字段投影（Phases 1–2）

## What This Is

sqllog2db 是一个用于解析达梦数据库 SQL 日志文件并将其导出为 CSV 或 SQLite 的命令行工具。以流式方式处理日志记录，通过可选的 Pipeline 过滤器处理后写入配置的导出器。支持正则表达式多字段过滤（AND 语义 include + OR-veto exclude）、输出字段精确控制、SQL 模板归一化与统计聚合。

## Core Value

用户能够精确指定"导出哪些记录的哪些字段"——过滤逻辑清晰可配置，输出结果完全可控。SQL 模板归一化帮助 DBA 理解 SQL 执行模式。

## Requirements

### Validated

- ✓ 流式解析达梦 SQL 日志文件 — existing
- ✓ 导出到 CSV 和 SQLite — existing
- ✓ Pipeline 过滤器（记录级 + 事务级） — existing
- ✓ 字段投影（ordered_indices Vec） — existing
- ✓ 参数归一化 / SQL 指纹 — existing
- ✓ 增量断点续传（resume state） — existing
- ✓ 并行 CSV 处理（rayon） — existing
- ✓ FILTER-01 正则表达式多字段过滤 — v1.0
- ✓ FILTER-02 多关键词 AND 语义 — v1.0
- ✓ FIELD-01 输出字段控制 — v1.0
- ✓ PERF-01  profiling 基础设施 — v1.1
- ✓ PERF-02/03/08 CSV 性能优化 — v1.1
- ✓ PERF-04/06 SQLite 批量事务 + prepared statement — v1.1
- ✓ PERF-07/09 解析库 1.0.0 升级 — v1.1
- ✓ DEBT-01/02 SQLite 技术债修复 — v1.2
- ✓ FILTER-03 排除过滤器 — v1.2
- ✓ PERF-10 热路径门控分析 — v1.2
- ✓ PERF-11 validate_and_compile() 统一 — v1.2
- ✓ DEBT-03 Nyquist 审计补签 — v1.2
- ✓ TMPL-01 SQL 模板归一化引擎 — v1.3
- ✓ TMPL-02 TemplateAggregator 流式统计 — v1.3
- ✓ TMPL-04 双路统计输出 — v1.3
- ✓ CHART-01/02/03/04/05 四类 SVG 图表 — v1.3
- ✓ CONFIG-01/02/05 过滤器配置嵌套化 — v1.4
- ✓ CONFIG-03/04 模板/图表配置嵌套化 — v1.4
- ✓ REFACTOR-01/02/03/04 代码结构重构 — v1.4
- ✓ TEST-01/02/03/04 测试覆盖深化 — v1.4
- ✓ I18N-01: README.md 改为中文 — v1.6
- ✓ I18N-02: GitHub Pages 落地页改为中文 — v1.6
- ✓ I18N-03: docs/quickstart.md 改为中文 — v1.6
- ✓ I18N-04: docs/config-reference.md 改为中文 — v1.6
- ✓ DESVG-01: README 中移除 SVG 截图和图表引用 — v1.6
- ✓ DESVG-02: GitHub Pages 中移除 SVG Gallery section — v1.6
- ✓ DOC-01: CONTRIBUTING.md（中文）— v1.6
- ✓ DOC-02: SECURITY.md（中文）— v1.6
- ✓ DOC-03: docs/architecture.md（中文）— v1.6
- ✓ PAGES-01: GitHub Pages 单页→多页 mdBook 文档站 — v1.6
- ✓ TMPL-03: 模板统计结果独立 CSV 摘要文件 — v1.6
- ✓ TMPL-03b: 模板统计结果独立 SQLite 报告文件 — v1.6
- ✓ RM-01: 移除 SVG 图表模块 — v1.7 Phase 28
- ✓ RM-02: 移除 self-update 自更新功能 — v1.7 Phase 28
- ✓ RM-07: 移除 Shell 补全和 Man page 生成 — v1.7 Phase 28
- ✓ RM-03: 移除 stats 统计命令 — v1.7 Phase 29
- ✓ RM-04: 迁移 normalize_template 并移除 digest 命令 — v1.7 Phase 29

### Active

（下一里程碑需求待定义 — 运行 `/gsd:new-milestone`）

### Out of Scope

- OR 条件组合（FILTER-04）— 简单列表 AND 已满足需求
- 跨字段联合条件（FILTER-05）— 暂不支持复合谓词
- 运行时动态修改过滤规则 — 配置在启动时加载
- `exclude_trxids` 正则支持 — 保持 HashSet 精确匹配
- SQLite WAL 模式 — 用户决策移除
- JSON / Parquet 导出 — 超出范围
- stats.rs 拆分 — D-01 范围外，延后
- Playground / WASM Demo — 高复杂度，延后至未来版本

## Context

- 架构：过滤层（`src/pipeline/filters/`）+ 模板分析层（`src/pipeline/fingerprint.rs` + `template_aggregator.rs` + `template_reporter.rs`）
- v1.4 重构：配置模型 5 顶层字段（template/charts/filter/output/replace_parameters）、代码 5 模块拆分
- v1.6 新增：`[template.report]` 配置段 + `TemplateReporter` 独立 CSV/SQLite 报告输出
- `CompiledMetaFilters` + `CompiledSqlFilters` 预编译，`validate_and_compile()` 单次编译贯穿全链路
- `TemplateAggregator` 通过 `Option<&mut TemplateAggregator>` 侧路径接入热循环
- `ordered_indices: Vec<usize>` 注入 Exporter，支持任意字段顺序投影
- `pipeline.is_empty()` 保证无过滤时零开销快路径
- 文档：全中文 README + mdBook 多页文档站（四章导航：首页、快速入门、配置参考、架构设计）
- Rust LOC: ~16,000+ (src/) | 测试: ~832 tests | 基准: ~5.2M records/sec (CSV synthetic)

## Constraints

- **性能**: 过滤逻辑不能破坏热循环的零开销快路径（pipeline.is_empty() 检查）
- **配置格式**: TOML，与现有 `config.toml` 风格保持一致
- **兼容性**: 旧版扁平配置通过 serde alias 向后兼容
- **函数长度**: ≤ 40 行（CLAUDE.md 约束）

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| 列表默认 AND 语义 | 简单直观，覆盖最常见场景 | ✓ v1.0 |
| ordered_indices Vec 替代 FieldMask | 支持任意字段顺序 | ✓ v1.0 |
| FILTER-03 集成进 CompiledMetaFilters | 避免双调用开销 | ✓ v1.2 |
| validate_and_compile() 合并接口 | 消除双重 Regex::new() | ✓ v1.2 |
| PERF-10 D-G1 门控 >5% | 避免盲目优化 | ✓ v1.2（未命中）|
| hdrhistogram 存储耗时样本 | ~24KB/模板 vs Vec<u64> ~40MB | ✓ v1.3 |
| 并行 CSV map-reduce merge() | 消除锁竞争 | ✓ v1.3 |
| RawFiltersFeature 中间 struct 向后兼容 | serde#2341 flatten+alias 不可靠 | ✓ v1.4 |
| 破坏性升级无 serde alias | validate() 明确拒绝旧路径 | ✓ v1.4 |
| DryRunExporter → ExporterKind::DryRun | struct variant 减少代码量 | ✓ v1.4 |
| ExporterManager 收紧至 pub(crate) | Exporter trait 保留 pub（bench） | ✓ v1.4 |
| proptest 属性测试 | 幂等性/不变性验证优于 fuzz | ✓ v1.4 |
| README 纯英文最小骨架 | 国际可见性 + 降低中英混排维护负担 | ✓ v1.5 |
| rsvg-convert 替代 ImageMagick | macOS IMv7 字体渲染失败，librsvg 干净解决 | ✓ v1.5 |
| ASCII art 替代 Mermaid.js（Pages） | mdBook 不支持 Mermaid.js，无需 JS 依赖 | ✓ v1.5 |
| lychee CI 内部严格 + 外部重试 | 防断链回归，crates.io 速率限制排除 | ✓ v1.5 |
| README 改为中文（I18N-01） | 目标用户为中文 DBA，无需双语维护 | ✓ v1.6 |
| I18N + DESVG 合并为 Phase 24 | 修改相同文件集（README、docs/*、site/） | ✓ v1.6 |
| TMPL-03 为独立 CSV 报告（非 JSON） | 与既有 `*_templates.csv` 输出模式一致 | ✓ v1.6 |
| TMPL-03b 为独立 SQLite 报告 | 补充另一种结构化输出格式 | ✓ v1.6 |
| `[templates]` → `[template.report]` | 语义更清晰，明确为独立报告配置 | ✓ v1.6 |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-05-20 — v1.7 Phase 29 complete*
