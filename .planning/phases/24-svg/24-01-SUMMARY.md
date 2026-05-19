# 24-01: README.md 中文化 + 去 SVG + book.toml

**Status:** Complete
**Tasks:** 2/2
**Self-Check:** PASSED

## What Was Built

将 README.md 完整翻译为中文（约 200 行），移除了所有 SVG 截图引用（PNG 图片和 Gallery 链接），并用纯文字描述替代。同时将 site/book.toml 的 language 从 "en" 改为 "zh"。

## Key Files

| File | Action | Lines |
|------|--------|-------|
| README.md | 全文翻译 + 移除 SVG Charts + 添加中文描述 | ~200 |
| site/book.toml | language = "en" → "zh" | 1 行改动 |

## Decisions

- 术语翻译遵循统一的术语表（pipeline→处理管道, exporter→导出器, filter→过滤器 等）
- SVG Charts 章节完全移除，替换为纯文字描述（含四类图表说明）
- Mermaid 图表和 ASCII art 流程图保留
- 所有代码块、命令、URL、Badge 保持原样
- book.toml 仅修改 language 字段，其他配置保持不变

## Verification

所有验证通过：PNG 引用已移除、SVG Charts 章节已移除、中文描述已添加、book.toml language 已更新为 "zh"。
