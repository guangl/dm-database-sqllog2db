# Phase 26: GitHub Pages 多页文档站 - Context

**Gathered:** 2026-05-19
**Status:** Ready for planning

## Phase Boundary

将现有单页 GitHub Pages 落地页重构为 mdBook 多页中文文档站。依赖 Phase 24（文档中文化）和 Phase 25（延后文档补全）的内容产出。

**In scope:** PAGES-01
**Out of scope:** WASM/Playground 在线演示（PAGES-F02，延后至 v1.7+）；站点英文版保留（不保留双语版本）

## Implementation Decisions

### 站点结构
- **D-01:** 四大章节结构 — 快速入门（quickstart）→ 配置参考（config reference）→ 架构说明（architecture）→ 贡献指南（contributing + security）

### 语言配置
- **D-02:** site/book.toml 的 `language` 已在 Phase 24 改为 `"zh"`，Phase 26 不再修改此字段
- **D-03:** 所有章节内容均来自 Phase 24+25 的中文化产出

### 依赖关系
- **D-04:** Phase 26 依赖 Phase 24 和 Phase 25 完成（文档内容中文化定稿后才能重构站点导航和组织）

### Claude's Discretion
- SUMMARY.md 的具体章节排列和子页面拆分方案
- 是否保留现有 custom.css 主题样式
- mdBook 的 fold/nav 等配置微调
- 各章节之间的交叉引用和导航

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求文档
- `.planning/REQUIREMENTS.md` — PAGES-01（GitHub Pages 多页文档站）的权威描述
- `.planning/ROADMAP.md` — Phase 26 的范围和依赖关系（依赖 Phase 24+25）

### 现有站点配置
- `site/book.toml` — mdBook 构建配置（language 已在 Phase 24 改为 zh，其余字段需评估）
- `site/src/SUMMARY.md` — 当前单页目录（需改为多章节结构）
- `site/src/index.md` — 当前落地页内容（已被 Phase 24 翻译为中文）
- `site/theme/custom.css` — 当前自定义样式

### CI/CD
- `.github/workflows/pages.yml` — GitHub Pages 部署流水线（需评估是否适配 mdBook 多页构建）

### 内容来源
- `README.md` — 项目概述（Phase 24 中文版）
- `docs/quickstart.md` — 快速入门（Phase 24 中文版）
- `docs/config-reference.md` — 配置参考（Phase 24 中文版）
- `docs/architecture.md` — 架构文档（Phase 25 新建）
- `CONTRIBUTING.md` — 贡献指南（Phase 25 新建）
- `SECURITY.md` — 安全策略（Phase 25 新建）

## Existing Code Insights

### Reusable Assets
- mdBook 已配置完成（`site/book.toml`），只需修改 SUMMARY.md 和添加内容页面
- pages.yml 部署流水线已验证可用（v1.5 产出）

### Established Patterns
- v1.5 决策 "ASCII art 替代 Mermaid.js" — 站点中不使用 Mermaid.js 图表
- v1.5 决策 "rsvg-convert 替代 ImageMagick" — 站点构建不依赖 ImageMagick

### Integration Points
- site/src/ 目录下的各章节页面需与 SUMMARY.md 的章节定义一一对应
- pages.yml 中的 mdbook 构建步骤需确认多页输出正确

## Specific Ideas

- 建议保留现有 landing page 的精华内容作为 index.md（首页概述），其余拆分为独立章节
- 站点导航建议使用 mdBook 的 fold 功能（当前已启用 fold level=1）

## Deferred Ideas

None — discussion stayed within phase scope

---

*Phase: 26-GitHub Pages 多页文档站*
*Context gathered: 2026-05-19*
