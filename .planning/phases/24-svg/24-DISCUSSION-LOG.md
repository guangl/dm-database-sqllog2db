# Phase 24: 文档中文化 & 去 SVG 化 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-19
**Phase:** 24-文档中文化 & 去 SVG 化
**Areas discussed:** 翻译策略, SVG 替代方案, CLAUDE.md 中文化, book.toml 语言设置

---

## 翻译策略

| Option | Description | Selected |
|--------|-------------|----------|
| 全手动精翻 | 逐句阅读并手写中文翻译，确保术语准确、表达自然 | |
| 机翻+人工校对 | 先用翻译工具生成初稿，再人工校对修正，速度快但需多轮审核 | ✓ |
| 你决定 | 由 Claude 根据文件类型选择合适策略 | |

**User's choice:** 机翻+人工校对
**Notes:** 与 v1.6 里程碑目标一致 — 在质量可控的前提下提高效率

---

## SVG 替代方案

| Option | Description | Selected |
|--------|-------------|----------|
| ASCII art 示意图 | 用文本字符绘制简化的图表样式 | |
| 纯文字描述 | 用文字描述图表类型和含义，不保留任何视觉元素 | ✓ |
| 你决定 | 由 Claude 选择最合适的方案 | |

**User's choice:** 纯文字描述
**Notes:** 简化方案，避免 ASCII art 的排版维护成本

---

## CLAUDE.md 中文化

| Option | Description | Selected |
|--------|-------------|----------|
| 一起中文化 | CLAUDE.md 也中文化，与 README + docs 保持一致 | |
| 保持现状 | CLAUDE.md 保持中英混排，只翻译用户面文档 | ✓ |

**User's choice:** 保持现状
**Notes:** CLAUDE.md 是开发者工具配置，不在用户面文档范围

---

## book.toml 语言设置

| Option | Description | Selected |
|--------|-------------|----------|
| Phase 24 | Phase 24 翻译内容时顺便改 language = "zh" | ✓ |
| Phase 26 | Phase 26 建设多页站点时统一改 | |

**User's choice:** Phase 24
**Notes:** 语言配置跟随内容翻译在同一阶段完成

---

## Deferred Ideas

None
