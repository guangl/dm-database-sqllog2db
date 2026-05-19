# Phase 22: GitHub Pages 落地页 + 部署流水线 - Context

**Gathered:** 2026-05-18
**Status:** Ready for planning

## Phase Boundary

使用 mdBook 构建单页项目落地页，部署到 `guangl.github.io/sqllog2db/`。GitHub Actions 在 site/ 目录变更时自动构建部署。落地页作为完整主页（与 README 最小骨架互补），包含架构图、性能数据、SVG Gallery 和 Asciicast 演示。

## Implementation Decisions

### Pages 内容与定位
- **D-01:** Pages 作为完整项目主页（含简介、安装、功能、性能），README 是最简索引。两者独立但互补。
- **D-02:** 页面结构：Hero 标题 → 安装命令 → 功能概览（图标+描述）→ 架构图（Mermaid）→ 性能表格 → SVG Gallery（4张+交互说明）→ 链接索引。
- **D-03:** 架构图使用 Mermaid.js（与 README 一致，mdBook 通过 mermaid.js CDN 支持）。

### mdBook 配置
- **D-04:** 源文件放在 `site/` 目录（book.toml + SUMMARY.md + 页面 .md），构建输出到 `site/book/`。
- **D-05:** 使用 mdBook 默认主题 + 少量自定义 CSS（品牌色、字体调整），增加页面辨识度。
- **D-06:** SVG Gallery 内嵌 SVG 图片 + 可折叠交互说明（图表含义、生成方式、使用场景）。

### 部署流水线
- **D-07:** GitHub Actions 在推送 `site/**` 变更时触发（paths 过滤），构建 mdBook 并部署到 `gh-pages` 分支。
- **D-08:** 无需手动触发，精准触发减少无意义构建。

### 内容嵌入
- **D-09:** Asciicast 演示在 Pages 中嵌入交互式播放器（asciinema-player），README 中放静态 SVG 预览+链接。
- **D-10:** Cargo.toml 的 `documentation` 字段在 Pages 部署后更新为 `https://guangl.github.io/sqllog2db/`。

### Claude's Discretion
- 自定义 CSS 的具体样式（颜色、字体、间距）
- 4 张 SVG 图表的具体选择（生成代表性示例）
- mdBook 的 book.toml 详细配置
- GitHub Actions workflow 的具体实现（actions-gh-pages 等）
- Mermaid 图的节点布局和样式

## Canonical References

### 项目文档
- `.planning/REQUIREMENTS.md` — v1.5 全部需求（PAGES-01 至 PAGES-05、SUPP-01、SUPP-06 分配给 Phase 22）
- `.planning/ROADMAP.md` — Phase 22 定义、依赖关系（依赖 Phase 21）、成功标准
- `.planning/PROJECT.md` — 项目上下文（架构、性能数据、v1.5 决策）

### 上游产物
- `.planning/phases/21-readme/21-CONTEXT.md` — Phase 21 决策（README 最小骨架影响 Pages 内容分工）
- `README.md` — Phase 21 产物，Pages 需与之互补非重复

### 外部参考
- https://rust-lang.github.io/mdBook/ — mdBook 官方文档（配置、主题、部署）
- https://github.com/peaceiris/actions-gh-pages — GitHub Pages 部署 Action
- https://mermaid.js.org/ — Mermaid 图表语法
- https://asciinema.org/ — Asciicast 播放器嵌入

## Existing Code Insights

### Reusable Assets
- SVG 图表生成代码在 `src/charts/` — 可直接运行生成 Gallery 用的 4 张示例图
- 性能数据来自 `benches/` — 运行 `cargo bench` 获取最新数据
- CLI 命令（`sqllog2db run` 等）— Asciicast 录制演示内容

### Established Patterns
- mdBook 是 Rust 生态标准文档工具
- GitHub Actions 已有 `ci.yaml` — 新增 workflow 遵循现有风格

### Integration Points
- Phase 21 README 链接到 Pages URL
- Pages URL `guangl.github.io/sqllog2db/` 写入 Cargo.toml `documentation` 字段
- 仓库 Settings > Pages 需配置 `gh-pages` 分支作为部署源

## Specific Ideas

- 性能表格格式：合成 CSV / 合成 SQLite / 真实文件（M 系列 NVMe SSD），标注硬件环境
- SVG Gallery 每张图包含：图表内嵌 + 生成命令 + 图表含义说明
- 安装命令同时展示 `cargo install` 和 `cargo build --release` 两种方式

## Deferred Ideas

- 完整多页 mdBook 站点（导航、搜索、独立 Gallery 页）— v1.6+（PAGES-F01）
- Playground / WASM Web Demo — v1.6+（PAGES-F02）
- 独立域名 — 不需要（项目规模）

---
*Phase: 22-GitHub Pages 落地页 + 部署流水线*
*Context gathered: 2026-05-18*
