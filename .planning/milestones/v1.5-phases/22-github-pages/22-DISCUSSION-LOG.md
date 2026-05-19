# Phase 22: GitHub Pages 落地页 + 部署流水线 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-18
**Phase:** 22-GitHub Pages 落地页 + 部署流水线
**Areas discussed:** GitHub Pages 内容与展示

---

## Pages 与 README 分工

| Option | Description | Selected |
|--------|-------------|----------|
| Pages 侧重可视化 | README=文字参考源，Pages=可视化展示 | |
| Pages 作为完整主页 | Pages 是完整项目主页，README 是最简索引 | ✓ |

**User's choice:** Pages 作为完整主页

### Pages 主题

| Option | Description | Selected |
|--------|-------------|----------|
| mdBook 默认主题 | 内置 light/rust 主题，零定制成本 | |
| mdBook + 自定义 CSS | 基于默认主题，少量自定义品牌色/字体 | ✓ |

**User's choice:** mdBook + 自定义 CSS

### SVG Gallery 展示

| Option | Description | Selected |
|--------|-------------|----------|
| 内嵌 SVG + 说明 | 嵌入 4 张 SVG，每张配简短说明 | |
| 截图 + 下载 SVG | PNG 缩略图 + 原始 SVG 下载链接 | |
| 内嵌 SVG + 交互说明 | 嵌入 SVG + 可折叠说明区块 | ✓ |

**User's choice:** 内嵌 SVG + 交互说明

### 部署触发策略

| Option | Description | Selected |
|--------|-------------|----------|
| 推送 site/ 变更时触发 | 精准触发，减少无意义构建 | ✓ |
| 推送到 main 时触发 | 简单但可能频繁 | |
| 手动 + 自动结合 | main 推送 + workflow_dispatch | |

**User's choice:** 推送 site/ 变更时触发

### Pages 内容结构

| Option | Description | Selected |
|--------|-------------|----------|
| 推荐结构 | Hero → 安装 → 功能概览 → 架构图 → 性能表格 → SVG Gallery → 链接 | ✓ |
| 你来设计 | Claude 设计最佳页面结构 | |

**User's choice:** 推荐结构

### mdBook 源文件目录

| Option | Description | Selected |
|--------|-------------|----------|
| site/ | mdBook 源文件在 site/，构建输出 site/book/ | ✓ |
| docs/book/ | 源文件在 docs/book/，与用户文档共存 | |

**User's choice:** site/

---

## Claude's Discretion

- 自定义 CSS 的具体样式（颜色、字体、间距）
- 4 张 SVG 图表的具体选择
- mdBook book.toml 详细配置
- GitHub Actions workflow 具体实现
- Mermaid 图节点布局和样式

## Deferred Ideas

- 完整多页 mdBook 站点 — v1.6+（PAGES-F01）
- Playground / WASM Web Demo — v1.6+（PAGES-F02）
- 独立域名 — 不需要
