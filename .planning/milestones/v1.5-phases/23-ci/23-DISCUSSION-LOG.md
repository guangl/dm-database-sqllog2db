# Phase 23: 补充文档 + CI 质量门禁 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-18
**Phase:** 23-补充文档 + CI 质量门禁
**Areas discussed:** 补充文档与 CI 细节

---

## docs/quickstart.md 深度

| Option | Description | Selected |
|--------|-------------|----------|
| 完整输出示例 + 故障排除 | 每个命令展示终端输出、常见错误、环境准备 | |
| 分场景教程 | 场景1-导出CSV、场景2-导出SQLite、场景3-统计分析、场景4-模板聚合 | ✓ |

**User's choice:** 分场景教程

### docs/config-reference.md 格式

| Option | Description | Selected |
|--------|-------------|----------|
| 每个配置块独立章节 | TOML 示例 + 字段表格 + 注意事项 | ✓ |
| 单一大 TOML 示例 | 完整 config.toml 逐行注释 | |

**User's choice:** 每个配置块独立章节

### Asciicast 录制内容

| Option | Description | Selected |
|--------|-------------|----------|
| 完整 3 步流程 | init → validate → run，30-45 秒 | ✓ |
| run 命令特写 | 只展示实时输出，20-30 秒 | |

**User's choice:** 完整 3 步流程

### Asciicast 嵌入位置

| Option | Description | Selected |
|--------|-------------|----------|
| 嵌入在 Pages | Pages 放交互式播放器，README 链接 | |
| 嵌入在 README | README 放 SVG 链接到 asciinema.org | |
| 两处都放 | README 静态 SVG 预览+链接，Pages 交互式播放器 | ✓ |

**User's choice:** 两处都放

### docs/ 目录结构

| Option | Description | Selected |
|--------|-------------|----------|
| 平铺在 docs/ 根 | quickstart.md + config-reference.md 直接在 docs/ 下 | ✓ |
| 分类子目录 | docs/guides/ + docs/reference/ + docs/design/ | |

**User's choice:** 平铺在 docs/ 根

### lychee 外部链接处理

| Option | Description | Selected |
|--------|-------------|----------|
| 重试 + 超时 | --max-retries 3 --timeout 30，失败阻塞 CI | ✓ |
| 仅警告不阻塞 | 外部链接失败只警告 | |

**User's choice:** 重试 + 超时

### lychee 触发时机

| Option | Description | Selected |
|--------|-------------|----------|
| 每次 PR 和 main 推送 | 所有情况都运行 lychee | |
| 仅文档变更时 | Markdown 文件变更时触发（paths 过滤） | ✓ |

**User's choice:** 仅文档变更时触发

### lychee 检查范围

| Option | Description | Selected |
|--------|-------------|----------|
| 检查所有 Markdown 文件 | README + CHANGELOG + docs/ + site/，排除 .planning/ | ✓ |
| 仅检查文档目录 | README + docs/ + site/ | |
| 仅检查内部链接 | 只检查相对路径链接，忽略外部 URL | |

**User's choice:** 检查所有 Markdown 文件

---

## Claude's Discretion

- docs/quickstart.md 各场景的具体命令和示例输出
- docs/config-reference.md 的具体字段表格和 TOML 示例
- Asciicast 录制的具体终端内容
- lychee GitHub Actions workflow 的完整实现
- lychee 忽略的 URL 模式

## Deferred Ideas

- docs/architecture.md — v1.6+（DOC-F03）
- FAQ / Troubleshooting 板块 — v1.6+（DOC-F05）
- 自动化文档生成 — v1.5 范围外
