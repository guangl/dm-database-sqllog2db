# Roadmap: sqllog2db

## Milestones

- ✅ **v1.0 增强 SQL 内容过滤与字段投影** — Phases 1–2 (shipped 2026-04-18)
- ✅ **v1.1 性能优化** — Phases 3–6 (shipped 2026-05-10)
- ✅ **v1.2 质量强化 & 性能深化** — Phases 7–11 (shipped 2026-05-15)
- ✅ **v1.3 SQL 模板分析 & 可视化** — Phases 12–16 (shipped 2026-05-17)
- ✅ **v1.4 代码重构 & 质量深化** — Phases 17–20 (shipped 2026-05-18)
- 🚧 **v1.5 文档完善 & 项目展示** — Phases 21–23 (in progress)

## Phases

<details>
<summary>✅ v1.0 增强 SQL 内容过滤与字段投影 (Phases 1–2) — SHIPPED 2026-04-18</summary>

- [x] Phase 1: 正则字段过滤 (2/2 plans) — completed 2026-04-18
- [x] Phase 2: 输出字段控制 (4/4 plans) — completed 2026-04-18

Full details: `.planning/milestones/v1.0-ROADMAP.md`

</details>

<details>
<summary>✅ v1.1 性能优化 (Phases 3–6) — SHIPPED 2026-05-10</summary>

- [x] Phase 3: Profiling & Benchmarking (3/3 plans) — completed 2026-04-27
- [x] Phase 4: CSV 性能优化 (4/4 plans) — completed 2026-05-09
- [x] Phase 5: SQLite 性能优化 (3/3 plans) — completed 2026-05-10
- [x] Phase 6: 解析库集成 + 验收 (2/2 plans) — completed 2026-05-10

Full details: `.planning/milestones/v1.1-ROADMAP.md`

</details>

<details>
<summary>✅ v1.2 质量强化 & 性能深化 (Phases 7–11) — SHIPPED 2026-05-15</summary>

- [x] Phase 7: 技术债修复 (1/1 plans) — completed 2026-05-10
- [x] Phase 8: 排除过滤器 (2/2 plans) — completed 2026-05-10
- [x] Phase 9: CLI 启动提速 (5/5 plans) — completed 2026-05-14
- [x] Phase 10: 热路径优化 (3/3 plans) — completed 2026-05-15
- [x] Phase 11: Nyquist 补签 (2/2 plans) — completed 2026-05-15

Full details: `.planning/milestones/v1.2-ROADMAP.md`

</details>

<details>
<summary>✅ v1.3 SQL 模板分析 & 可视化 (Phases 12–16) — SHIPPED 2026-05-17</summary>

- [x] Phase 12: SQL 模板归一化引擎 (3/3 plans) — completed 2026-05-15
- [x] Phase 13: TemplateAggregator 流式统计累积器 (2/2 plans) — completed 2026-05-15
- [x] Phase 14: Exporter 集成输出 (4/4 plans) — completed 2026-05-16
- [x] Phase 15: SVG 图表基础设施 + 前两类图表 (5/5 plans) — completed 2026-05-17
- [x] Phase 16: 剩余图表 (5/5 plans) — completed 2026-05-17

Full details: `.planning/milestones/v1.3-ROADMAP.md`

</details>

<details>
<summary>✅ v1.4 代码重构 & 质量深化 (Phases 17–20) — SHIPPED 2026-05-18</summary>

- [x] Phase 17: 过滤器配置嵌套化 (2/2 plans) — completed 2026-05-18
- [x] Phase 18: 模板 & 图表配置嵌套化 (3/3 plans) — completed 2026-05-18
- [x] Phase 19: 代码结构重构 (4/4 plans) — completed 2026-05-18
- [x] Phase 20: 测试覆盖深化 (3/3 plans) — completed 2026-05-18

Full details: `.planning/milestones/v1.4-ROADMAP.md`

</details>

### 🚧 v1.5 文档完善 & 项目展示 (In Progress)

**Milestone Goal:** 补全项目文档（README 更新 + CHANGELOG + LICENSE）并建立 GitHub Pages 展示页面，零代码变更

- [ ] **Phase 21: README 全面更新 + 根文档补全** — 重写 README 覆盖 v1.3/v1.4 全部功能，补全 CHANGELOG.md 和 LICENSE (2 plans)
- [ ] **Phase 22: GitHub Pages 落地页 + 部署流水线** — mdBook 构建单页落地页，GitHub Actions 自动部署，含架构图/性能数据/SVG Gallery
- [ ] **Phase 23: 补充文档 + CI 质量门禁** — docs/quickstart.md、docs/config-reference.md、Asciicast 演示、lychee 链接检查

## Phase Details

### Phase 21: README 全面更新 + 根文档补全

**Goal**: 用户能阅读全面更新的 README，准确反映 v1.3/v1.4 的全部功能特性，且仓库根目录包含 CHANGELOG.md 和 LICENSE
**Depends on**: Phase 20 (previous milestone)
**Requirements**: DOC-01, DOC-02, DOC-03, DOC-04, DOC-05, DOC-06, DOC-07, DOC-08, DOC-09
**Success Criteria** (what must be TRUE):

  1. 用户阅读 README 时能看到 v1.3 模板分析完整说明（normalize_template、TemplateAggregator、双路统计输出）和 v1.4 嵌套配置文档（[filter]、[template]、[charts] 独立子表）
  2. 用户可以从 README 复制粘贴 3 条 QuickStart 命令（init/validate/run）并成功执行，配置示例与 `sqllog2db init` 实际输出一致
  3. 用户访问仓库根目录能看到 CHANGELOG.md（Keep a Changelog 格式，v1.0-v1.4）、LICENSE（Apache-2.0）和 4-6 个项目徽章（CI、crates.io、license、release）
  4. README 中不存在指向非存在文件的链接（CONTRIBUTING.md、SECURITY.md、docs/architecture.md 等已移除或替换为占位标记）

**Plans**: 2 plans

Plans:

- [ ] 21-01-PLAN.md -- 重写 README.md 为纯英文最小骨架（200-250 行），覆盖 v1.3/v1.4 全部功能
- [ ] 21-02-PLAN.md -- 补全 CHANGELOG.md 至 v1.4，折叠 0.x 旧版本；确认 LICENSE 存在

### Phase 22: GitHub Pages 落地页 + 部署流水线

**Goal**: 用户能访问 `guangl.github.io/sqllog2db/` 看到精美的项目展示页，通过 mdBook 构建并由 GitHub Actions 自动部署
**Depends on**: Phase 21
**Requirements**: PAGES-01, PAGES-02, PAGES-03, PAGES-04, PAGES-05, SUPP-01, SUPP-06
**Success Criteria** (what must be TRUE):

  1. 用户访问 `guangl.github.io/sqllog2db/` 能看到项目落地页（项目介绍、安装命令、功能概览），使用 mdBook 构建，零 Node.js 依赖
  2. 用户能看到性能基准表格（合成 CSV 5.2M/s + 真实文件 1.55M/s，标注测试环境）和架构/数据流图（Mermaid.js 或 ASCII）
  3. 用户能在落地页看到 SVG 图表 Gallery（4 张实际生成的图表示例：频率柱状图、延迟直方图、趋势折线图、用户饼图）
  4. GitHub Actions 在推送 `site/**` 变更时自动构建 mdBook 并部署到 `gh-pages` 分支，落地页内容与 README 互补非重复
  5. Cargo.toml 的 `documentation` 字段指向已部署的 GitHub Pages URL

**Plans**: 2 plans

Plans:
**Wave 1**

- [ ] 22-01-PLAN.md -- mdBook 基础设施（book.toml/SUMMARY.md/custom.css）+ GHA 部署流水线 + Cargo.toml documentation 字段

**Wave 2** *(blocked on Wave 1 completion)*

- [ ] 22-02-PLAN.md -- 落地页内容（Hero/Install/Feature/Architecture/Performance/SVG Gallery/Links）+ 4 张示例 SVG 图表

### Phase 23: 补充文档 + CI 质量门禁

**Goal**: 用户能访问更详细的快速入门指南、完整的配置参考文档，以及 Asciicast 终端演示，CI 自动防止文档链接腐化
**Depends on**: Phase 22
**Requirements**: SUPP-02, SUPP-03, SUPP-04, SUPP-05
**Success Criteria** (what must be TRUE):

  1. 用户能找到并阅读 `docs/quickstart.md`，内容比 README QuickStart 更详细（含完整输出示例和故障排除）
  2. 用户能查阅 `docs/config-reference.md`，包含所有配置块的注释示例（filter / template / charts / output / replace_parameters）
  3. 用户能在 README 或落地页中观看嵌入的 Asciicast 终端演示（约 30 秒，展示 `sqllog2db run` 实时输出）
  4. CI 工作流包含 lychee 链接检查，文档中不存在断链

**Plans**: 4 plans

Plans:
**Wave 1**

- [ ] 23-01-PLAN.md -- 创建 docs/quickstart.md（4 场景教程 + 环境准备 + 故障排除）
- [ ] 23-02-PLAN.md -- 创建 docs/config-reference.md（8 配置块 x TOML示例+字段表格+注意事项）
- [ ] 23-03-PLAN.md -- Asciicast 录制 init→validate→run，嵌入 README(SVG) + Pages(交互播放器)

**Wave 2** *(blocked on Wave 1 completion)*

- [ ] 23-04-PLAN.md -- lychee 链接检查 CI workflow（paths 过滤 + 外部重试 + 内部严格）

## Progress

**Execution Order:** Phases execute in numeric order: 21 → 22 → 23

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. 正则字段过滤 | v1.0 | 2/2 | Complete | 2026-04-18 |
| 2. 输出字段控制 | v1.0 | 4/4 | Complete | 2026-04-18 |
| 3. Profiling & Benchmarking | v1.1 | 3/3 | Complete | 2026-04-27 |
| 4. CSV 性能优化 | v1.1 | 4/4 | Complete | 2026-05-09 |
| 5. SQLite 性能优化 | v1.1 | 3/3 | Complete | 2026-05-10 |
| 6. 解析库集成 + 验收 | v1.1 | 2/2 | Complete | 2026-05-10 |
| 7. 技术债修复 | v1.2 | 1/1 | Complete | 2026-05-10 |
| 8. 排除过滤器 | v1.2 | 2/2 | Complete | 2026-05-10 |
| 9. CLI 启动提速 | v1.2 | 5/5 | Complete | 2026-05-14 |
| 10. 热路径优化 | v1.2 | 3/3 | Complete | 2026-05-15 |
| 11. Nyquist 补签 | v1.2 | 2/2 | Complete | 2026-05-15 |
| 12. SQL 模板归一化引擎 | v1.3 | 3/3 | Complete | 2026-05-15 |
| 13. TemplateAggregator 流式统计累积器 | v1.3 | 2/2 | Complete | 2026-05-15 |
| 14. Exporter 集成输出 | v1.3 | 4/4 | Complete | 2026-05-16 |
| 15. SVG 图表基础设施 + 前两类图表 | v1.3 | 5/5 | Complete | 2026-05-17 |
| 16. 剩余图表 | v1.3 | 5/5 | Complete | 2026-05-17 |
| 17. 过滤器配置嵌套化 | v1.4 | 2/2 | Complete | 2026-05-18 |
| 18. 模板 & 图表配置嵌套化 | v1.4 | 3/3 | Complete | 2026-05-18 |
| 19. 代码结构重构 | v1.4 | 4/4 | Complete | 2026-05-18 |
| 20. 测试覆盖深化 | v1.4 | 3/3 | Complete | 2026-05-18 |
| 21. README 全面更新 + 根文档补全 | v1.5 | 0/2 | Not started | - |
| 22. GitHub Pages 落地页 + 部署流水线 | v1.5 | 0/2 | Not started | - |
| 23. 补充文档 + CI 质量门禁 | v1.5 | 0/4 | Not started | - |
