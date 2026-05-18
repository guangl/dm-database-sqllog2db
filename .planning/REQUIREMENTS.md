# Requirements — sqllog2db v1.5 文档完善 & 项目展示

**Milestone:** v1.5 文档完善 & 项目展示
**Created:** 2026-05-18
**Scope:** 文档完善 + GitHub Pages 展示页面，零代码变更

## Active Requirements

### 基础文档 (DOC)

- [ ] **DOC-01**: 用户能在 README 中看到 v1.3 模板分析功能的完整说明（normalize_template、TemplateAggregator、双路统计输出）
- [ ] **DOC-02**: 用户能在 README 中看到 v1.4 嵌套配置模型的完整说明（[filter.include]/[filter.exclude]、[template]、[charts] 独立子表）
- [ ] **DOC-03**: README 中的配置示例代码块与实际 `sqllog2db init` 输出一致（使用当前嵌套格式，非旧扁平格式）
- [ ] **DOC-04**: 用户能在 README 中看到四类 SVG 图表的功能说明和输出示例
- [ ] **DOC-05**: 项目仓库根目录存在 CHANGELOG.md，按 Keep a Changelog 格式记录 v1.0 至 v1.4 所有版本变更
- [ ] **DOC-06**: 项目仓库根目录存在 LICENSE 文件（Apache-2.0）
- [ ] **DOC-07**: README 头部包含项目徽章（CI status、crates.io version、license、release），数量 4-6 个
- [ ] **DOC-08**: README 包含 3-5 个可复制粘贴的 QuickStart 命令行示例（覆盖 init / run / digest / stats / validate）
- [ ] **DOC-09**: README 中不存在的文件链接已移除或替换为占位标记（CONTRIBUTING.md、SECURITY.md、docs/architecture.md 推迟至 v1.6+）

### GitHub Pages 展示 (PAGES)

- [ ] **PAGES-01**: 用户访问 `guangl.github.io/sqllog2db/` 能看到精美的项目落地页（项目介绍、安装命令、功能概览）
- [ ] **PAGES-02**: 落地页使用 mdBook 构建，零 Node.js 依赖，通过 GitHub Actions 自动部署到 gh-pages 分支
- [ ] **PAGES-03**: 落地页包含性能基准展示（表格：合成 CSV 5.2M/s + 真实文件 1.55M/s，标注测试环境）
- [ ] **PAGES-04**: 落地页包含架构/数据流图（Mermaid.js 或 ASCII：日志文件 → Parser → Pipeline → ExporterManager → CSV/SQLite）
- [ ] **PAGES-05**: 落地页与 README 内容互补非重复（Pages 侧重可视化展示，README 是文字参考源）

### 补充文档 & 质量保障 (SUPP)

- [ ] **SUPP-01**: 落地页包含 SVG 图表 Gallery（4 张实际生成的图表示例：频率柱状图、延迟直方图、趋势折线图、用户饼图）
- [ ] **SUPP-02**: 项目存在 docs/quickstart.md，内容比 README QuickStart 更详细（含完整输出示例和故障排除）
- [ ] **SUPP-03**: 项目存在 docs/config-reference.md，包含所有配置块的注释示例（filter / template / charts / output / replace_parameters）
- [ ] **SUPP-04**: README 或 Pages 中嵌入 Asciicast 终端演示（约 30 秒，展示 `sqllog2db run` 实时输出）
- [ ] **SUPP-05**: CI 工作流包含 lychee 链接检查，防止文档断链回归
- [ ] **SUPP-06**: Cargo.toml 的 `documentation` 字段指向 GitHub Pages URL（部署后更新）

## Future Requirements (v1.6+)

- [ ] **DOC-F01**: CONTRIBUTING.md — 贡献指南（环境搭建、编码规约、PR 流程）
- [ ] **DOC-F02**: SECURITY.md — 安全策略（漏洞报告联系方式）
- [ ] **DOC-F03**: docs/architecture.md — 详细架构文档
- [ ] **DOC-F04**: README.zh-CN.md — 中文版 README（或双语 Pages）
- [ ] **DOC-F05**: FAQ / Troubleshooting 板块
- [ ] **PAGES-F01**: GitHub Pages 完整多页站点（导航、搜索、独立 Gallery 页）
- [ ] **PAGES-F02**: Playground / WASM Web Demo

## Out of Scope

- 视频教程 — 维护成本高，Asciicast 替代
- 独立域名 — 项目规模不需要
- 完整 mdBook 多页站点 — v1.5 仅单页落地页
- CONTRIBUTING.md / SECURITY.md / docs/architecture.md — 推迟至 v1.6+
- 中文独立文档 — 推迟至 v1.6+

## Traceability

| REQ-ID | Phase | Status |
|--------|-------|--------|
| | | *待 roadmap 分配* |

---
*Requirements for: sqllog2db v1.5*
*Created: 2026-05-18*
