# Roadmap: sqllog2db

## Milestones

- ✅ **v1.0 增强 SQL 内容过滤与字段投影** — Phases 1–2 (shipped 2026-04-18)
- ✅ **v1.1 性能优化** — Phases 3–6 (shipped 2026-05-10)
- ✅ **v1.2 质量强化 & 性能深化** — Phases 7–11 (shipped 2026-05-15)
- ✅ **v1.3 SQL 模板分析 & 可视化** — Phases 12–16 (shipped 2026-05-17)
- ✅ **v1.4 代码重构 & 质量深化** — Phases 17–20 (shipped 2026-05-18)
- ✅ **v1.5 文档完善 & 项目展示** — Phases 21–23 (shipped 2026-05-19)
- [ ] **v1.6 文档中文化 & 延后需求补全** — Phases 24–27 (in progress)

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

<details>
<summary>✅ v1.5 文档完善 & 项目展示 (Phases 21–23) — SHIPPED 2026-05-19</summary>

- [x] Phase 21: README 全面更新 + 根文档补全 (2/2 plans) — completed 2026-05-19
- [x] Phase 22: GitHub Pages 落地页 + 部署流水线 (2/2 plans) — completed 2026-05-19
- [x] Phase 23: 补充文档 + CI 质量门禁 (4/4 plans) — completed 2026-05-19

Full details: `.planning/milestones/v1.5-ROADMAP.md`

</details>

### v1.6 文档中文化 & 延后需求补全 (Phases 24–27) — IN PROGRESS

- [ ] **Phase 24: 文档中文化 & 去 SVG 化** — 6 requirements
- [ ] **Phase 25: 延后文档补全** — 3 requirements
- [ ] **Phase 26: GitHub Pages 多页文档站** — 1 requirement
- [ ] **Phase 27: 模板报告独立输出** — 2 requirements

## Phase Details

### Phase 24: 文档中文化 & 去 SVG 化

**Goal**: 用户能阅读全中文的 README、快速入门指南和配置参考文档，且文档中不再包含 SVG 截图和图表引用
**Depends on**: Phase 23 (v1.5)
**Requirements**: I18N-01, I18N-02, I18N-03, I18N-04, DESVG-01, DESVG-02
**Success Criteria** (what must be TRUE):
1. README.md 全部为中文，无英文段落残留，无 SVG 截图或图表引用
2. GitHub Pages 落地页内容全部为中文，SVG Gallery 段落已移除
3. docs/quickstart.md 全部为中文
4. docs/config-reference.md 全部为中文
**Plans**: 3 plans

Plans:
- [ ] 24-01-PLAN.md — README.md 中文化 + 去 SVG + book.toml language="zh"
- [ ] 24-02-PLAN.md — GitHub Pages 落地页中文化 + 移除 SVG Gallery
- [ ] 24-03-PLAN.md — docs/quickstart.md + docs/config-reference.md 中文化

### Phase 25: 延后文档补全

**Goal**: 用户能访问 CONTRIBUTING.md、SECURITY.md 和 docs/architecture.md 三份中文文档，涵盖贡献指引、安全策略和架构说明
**Depends on**: Phase 24 (中文化文档风格定稿后新文档遵循相同模式)
**Requirements**: DOC-01, DOC-02, DOC-03
**Success Criteria** (what must be TRUE):
1. CONTRIBUTING.md（中文）提供完整的环境搭建、编码规约和 PR 提交流程
2. SECURITY.md（中文）提供漏洞报告联系方式和安全策略说明
3. docs/architecture.md（中文）提供数据流、模块划分和关键设计的架构文档
**Plans**: 1 plan

Plans:
- [ ] 25-01-PLAN.md — 创建 CONTRIBUTING.md、SECURITY.md、docs/architecture.md 三份中文文档

### Phase 26: GitHub Pages 多页文档站

**Goal**: GitHub Pages 从单页落地页升级为 mdBook 多页文档站，用户可通过导航栏访问各文档页面
**Depends on**: Phase 24, Phase 25 (所有文档内容定稿后进行站点重构)
**Requirements**: PAGES-01
**Success Criteria** (what must be TRUE):
1. GitHub Pages 显示多页文档站结构（导航栏、搜索功能），非单页落地页
2. 所有已中文化文档（README、quickstart、config-reference、architecture）在文档站中可访问
3. 文档站由 GitHub Actions 自动触发部署
**Plans**: 1 plan

Plans:
- [ ] 26-01-PLAN.md — 重构 SUMMARY.md 为四章导航 + 创建章节页面 + 重写首页概览
**UI hint**: yes

### Phase 27: 模板报告独立输出

**Goal**: 用户在启用模板分析时，能通过独立文件获取模板统计摘要报告，内容与主流程结果一致
**Depends on**: Phase 23 (独立于文档阶段，仅涉及 src/ 代码变更)
**Requirements**: TMPL-03, TMPL-03b
**Success Criteria** (what must be TRUE):
1. 启用模板分析时，输出目录生成 `*_templates.csv` 独立摘要文件
2. 启用模板分析时，输出目录生成 `*_templates.db` SQLite 独立报告文件
3. 报告文件内容与 `--include-templates` 双路统计输出保持一致
4. 禁用模板分析时，不生成任何额外的报告文件
**Plans**: 1 plan

Plans:
- [ ] 25-01-PLAN.md — 创建 CONTRIBUTING.md、SECURITY.md、docs/architecture.md 三份中文文档

## Progress

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
| 21. README 全面更新 + 根文档补全 | v1.5 | 2/2 | Complete | 2026-05-19 |
| 22. GitHub Pages 落地页 + 部署流水线 | v1.5 | 2/2 | Complete | 2026-05-19 |
| 23. 补充文档 + CI 质量门禁 | v1.5 | 4/4 | Complete | 2026-05-19 |
| 24. 文档中文化 & 去 SVG 化 | v1.6 | 0/3 | Not started | - |
| 25. 延后文档补全 | v1.6 | 0/0 | Not started | - |
| 26. GitHub Pages 多页文档站 | v1.6 | 0/1 | Not started | - |
| 27. 模板报告独立输出 | v1.6 | 0/0 | Not started | - |
