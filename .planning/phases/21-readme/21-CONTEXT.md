# Phase 21: README 全面更新 + 根文档补全 - Context

**Gathered:** 2026-05-18
**Status:** Ready for planning

## Phase Boundary

重写 README.md 为最小骨架（纯英文），覆盖 v1.3 模板分析/SVG 图表和 v1.4 嵌套配置的全部功能。补全 CHANGELOG.md 至 v1.4 版本。LICENSE 已存在（Apache-2.0）无需变更。

## Implementation Decisions

### README 结构与内容
- **D-01:** README 精简为最小骨架：项目简介+功能概览 → 安装 → 3步 QuickStart → 性能数据+架构图 → 链接索引。其余所有内容（stats/digest 示例、高级用法、FAQ、故障排查、开发测试、man page、shell补全、致谢）移出 README。
- **D-02:** 纯英文 README（中文版推迟至 v1.6+）。
- **D-03:** 功能概览按领域分组（解析与导出 / 过滤与字段控制 / 模板分析与图表 / 配置与性能），不是按版本号。
- **D-04:** 配置示例只展示 5-10 行核心 TOML（sqllog.path + exporter），详细内容链接到 docs/config-reference.md。
- **D-05:** 架构图使用 Mermaid.js 格式（GitHub 原生渲染）。
- **D-06:** QuickStart 保留 3 个核心命令（init → validate → run），digest 和 stats 用一句话提及并链接到 docs/quickstart.md。
- **D-07:** 嵌入 1-2 张代表性 SVG 图表截图（PNG 渲染），其余链接到 Pages Gallery。
- **D-08:** 性能数据更新为最新 benchmark 结果。
- **D-09:** 不存在的文档链接（CONTRIBUTING.md、SECURITY.md、docs/architecture.md）替换为 "(Coming v1.6)" 占位标记。
- **D-10:** 链接索引保留所有现有链接，不存在的标注状态。

### CHANGELOG 补全
- **D-11:** 按实际 crate 版本号补全：v1.0、v1.2、v1.2.1、v1.3、v1.4（v1.1 功能合入 v1.2）。
- **D-12:** 每个版本保持现有 Added/Changed/Fixed/Performance 详细分类。
- **D-13:** 0.x 旧版本（0.1.0–0.10.7）折叠为一个摘要段落。
- **D-14:** v1.0 条目需要迁移说明（从 0.x 到 1.0 的变更概述），即使破坏性变更很小。

### 跨 Phase 协调
- **D-15:** README 中链接到 docs/quickstart.md 和 docs/config-reference.md 时标注 "(Coming in Phase 23)"。Phase 23 创建文件后链接自动生效。

### Claude's Discretion
- 具体 Mermaid 图的节点和布局
- 性能数据的具体数值（需运行 benchmark）
- CHANGELOG 各版本的具体变更条目（从 git log 和里程碑文档提取）
- README 中嵌入哪 2 张图表（建议：频率柱状图 + 延迟直方图，最具代表性）

## Canonical References

### 项目文档
- `.planning/REQUIREMENTS.md` — v1.5 全部需求（DOC-01 至 DOC-09 分配给 Phase 21）
- `.planning/ROADMAP.md` — Phase 21 定义、依赖关系、成功标准
- `.planning/PROJECT.md` — 项目上下文（架构、性能数据、约束）

### 现有文件（需修改）
- `README.md` — 当前 395 行，需重写
- `CHANGELOG.md` — 当前停在 0.10.7，需补全至 v1.4
- `LICENSE` — Apache-2.0，已存在，无需变更

### 外部参考
- https://keepachangelog.com/en/1.0.0/ — CHANGELOG 格式标准
- https://mermaid.js.org/ — Mermaid 图表语法（GitHub 原生支持）

## Existing Code Insights

### Established Patterns
- 项目文档注释风格：中文用于领域概念，英文用于公开 API（见 CONVENTIONS.md）
- 英文标题 + 中文正文的混合风格（当前 README）→ Phase 21 改为纯英文
- Keep a Changelog 格式已在 CHANGELOG.md 中使用，保持一致

### Integration Points
- README 链接到 `docs/quickstart.md` 和 `docs/config-reference.md`（Phase 23 创建）
- README 链接到 GitHub Pages（Phase 22 部署）
- CHANGELOG 版本号与 crates.io `dm-database-sqllog2db` 保持一致

## Specific Ideas

- README 长度目标：200-250 行（从当前 395 行精简约 40-50%）
- CHANGELOG 目标：5 个新版本条目（v1.0 → v1.4）+ 0.x 折叠摘要
- 性能表格保留行：CSV synthetic、SQLite synthetic、真实文件

## Deferred Ideas

- README.zh-CN.md 中文版 — v1.6+
- CONTRIBUTING.md — v1.6+
- SECURITY.md — v1.6+
- docs/architecture.md — v1.6+
- CHANGELOG 自动化生成（git-cliff）— 不在 v1.5 范围

---
*Phase: 21-README 全面更新 + 根文档补全*
*Context gathered: 2026-05-18*
