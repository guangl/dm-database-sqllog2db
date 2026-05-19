# 24-03: docs/quickstart.md + docs/config-reference.md 中文化

**Status:** Complete
**Tasks:** 2/2
**Self-Check:** PASSED

## What Was Built

将 docs/quickstart.md（约 309 行）和 docs/config-reference.md（约 260 行）两篇技术文档完整翻译为中文。

## Key Files

| File | Action | Lines |
|------|--------|-------|
| docs/quickstart.md | 全文翻译 | ~290 |
| docs/config-reference.md | 全文翻译 | ~250 |

## Decisions

- 所有章节标题翻译为中文
- 配置节名称（[sqllog]、[exporter.csv] 等）保持原样
- 所有代码块、TOML 配置示例、bash 命令保持原样
- 表格字段名和类型保持英文
- CLI flag 名称保持英文
- 环境变量名保持原样
- 跨文档链接（../README.md、quickstart.md 等）保持原样
- 英文技术缩写保留（CSV、SQLite、TOML、SVG、NVMe 等）

## Verification

所有验证通过：quickstart 标题和场景已翻译、config-reference 章节已翻译、代码块和命令保持原样、TOML 配置块保持原样。
