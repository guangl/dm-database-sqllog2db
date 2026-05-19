# Requirements: sqllog2db

**Defined:** 2026-05-19
**Core Value:** 用户能够精确指定"导出哪些记录的哪些字段"——过滤逻辑清晰可配置，输出结果完全可控。模板分析与图表让 DBA 能直观理解 SQL 执行模式。

## v1 Requirements (v1.6)

### 文档中文化 (I18N)

- [ ] **I18N-01**: README.md 改为中文
- [ ] **I18N-02**: GitHub Pages 落地页改为中文
- [ ] **I18N-03**: docs/quickstart.md 改为中文
- [ ] **I18N-04**: docs/config-reference.md 改为中文

### 去 SVG 化 (DESVG)

- [ ] **DESVG-01**: README 中移除 SVG 截图和图表引用
- [ ] **DESVG-02**: GitHub Pages 中移除 SVG Gallery section

### 延后文档补全 (DOC)

- [ ] **DOC-01**: 创建 CONTRIBUTING.md（中文）— 贡献指南：环境搭建、编码规约、PR 流程
- [ ] **DOC-02**: 创建 SECURITY.md（中文）— 安全策略：漏洞报告联系方式
- [ ] **DOC-03**: 创建 docs/architecture.md（中文）— 详细架构文档

### GitHub Pages 升级 (PAGES)

- [ ] **PAGES-01**: GitHub Pages 从单页落地页升级为 mdBook 多页文档站（中文）

### 模板报告 (TMPL)

- [ ] **TMPL-03**: 模板统计结果输出为独立 CSV 摘要文件（`*_templates.csv`）
- [ ] **TMPL-03b**: 模板统计结果输出为独立 SQLite 报告文件（`*_templates.db`）

## v2 Requirements

延后至未来版本。

- **PAGES-F02**: Playground / WASM 在线演示 — 高复杂度，非核心需求

## Out of Scope

| Feature | Reason |
|---------|--------|
| 图表功能代码移除 | 仅清理文档引用，保留 src/charts/ 及 plotters 依赖 |
| Playground / WASM Demo | 高复杂度，延后至 v1.7+ |
| README 英文版保留 | README 仅保留中文版，不需要双语 |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| I18N-01 | Phase 24 | Pending |
| I18N-02 | Phase 24 | Pending |
| I18N-03 | Phase 24 | Pending |
| I18N-04 | Phase 24 | Pending |
| DESVG-01 | Phase 24 | Pending |
| DESVG-02 | Phase 24 | Pending |
| DOC-01 | Phase 25 | Pending |
| DOC-02 | Phase 25 | Pending |
| DOC-03 | Phase 25 | Pending |
| PAGES-01 | Phase 26 | Pending |
| TMPL-03 | Phase 27 | Pending |
| TMPL-03b | Phase 27 | Pending |

**Coverage:**
- v1 requirements: 12 total
- Mapped to phases: 12
- Unmapped: 0

---
*Requirements defined: 2026-05-19*
*Last updated: 2026-05-19 — v1.6 phases mapped*
