# 26-01: GitHub Pages 多页文档站

**Status:** Complete
**Tasks:** 3/3
**Self-Check:** PASSED

## What Was Built

将 GitHub Pages 从单页落地页升级为 mdBook 多页中文文档站。

| 文件 | 变更 |
|------|------|
| site/src/SUMMARY.md | 单页 "[sqllog2db](index.md)" → 四章导航 + 子页面 |
| site/src/index.md | 大型落地页 → 简洁概览页（安装 + 章节导航 + 链接） |
| site/src/quickstart.md | 新建，{{#include}} docs/quickstart.md |
| site/src/config-reference.md | 新建，{{#include}} docs/config-reference.md |
| site/src/architecture.md | 新建，{{#include}} docs/architecture.md |
| site/src/contributing.md | 新建，{{#include}} CONTRIBUTING.md |
| site/src/security.md | 新建，{{#include}} SECURITY.md |
| site/book.toml | description 中文化 |

## Key Decisions

- 四章结构（D-01）：快速入门 → 配置参考 → 架构说明 → 贡献指南（含安全策略子页面）
- 章节页面通过 {{#include}} 嵌入 Phase 24+25 源文件，无重复维护
- language="zh" 保持不变（D-02，Phase 24 已改）
- 首页不含 SVG Gallery（Phase 24 已移除）
- 首页不含 Demo/Terminal Recording（归属于 README）
- pages.yml 无需修改（构建命令 mdbook build site 和发布路径 site/book 兼容多页输出）

## Note

本地未安装 mdbook CLI（CI 通过 actions-mdbook action 安装）。文件结构和内容已通过验证，mdbook build 将在 CI pages.yml 触发时自动执行。
