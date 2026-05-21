# Phase 35: CLI --help 增强 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-21
**Phase:** 35-CLI --help 增强
**Areas discussed:** 示例内容, 示例放置位置, 帮助语言, 命令描述深度, 示例格式风格, value_hint分配, 管道示例时机, 配置文档引用

---

## 示例内容

| Option | Description | Selected |
|--------|-------------|----------|
| 2个场景 | 基本导出 + 自定义配置生成 | |
| 3个场景 | 导出 + 配置生成 + 配置验证 | |
| 4个场景 | 前3项 + 管道输入 | |
| 4-5个进阶 | 导出含过滤示例 + 配置生成 + 验证 + 管道 | ✓ |

**User's choice:** 4-5 scenarios, most comprehensive — include export with filters, config generation, validation, and pipe input

---

## 示例放置位置

| Option | Description | Selected |
|--------|-------------|----------|
| 仅顶层 | 全部示例在 `sqllog2db --help` | |
| 分层 | 顶层通用 + 子命令专属示例 | ✓ |
| 全量重复 | 顶层全放 + 子命令也全放 | |

**User's choice:** Top-level general examples + per-subcommand specific examples

---

## 帮助语言

| Option | Description | Selected |
|--------|-------------|----------|
| 英文 | 与代码注释一致，通用 | ✓ |
| 中文 | 与 v1.6 文档中文化一致 | |
| 中文为主 | 中文为主，关键术语保留英文 | |

**User's choice:** English

---

## 命令描述深度

| Option | Description | Selected |
|--------|-------------|----------|
| 只加示例 | 现有描述不变 | |
| 示例 + long_about | 深化子命令描述 | |
| 全面深化 | 示例 + long_about + help text + value_hint | ✓ |

**User's choice:** Examples + long_about + help text (but NO value_hint — see below)

---

## 示例格式风格

| Option | Description | Selected |
|--------|-------------|----------|
| Shell 风格 | `$ ` 前缀 + 注释 | |
| 纯命令 | 无前缀 + 说明在上方 | |
| cargo 风格 | 无 `$` 前缀，缩进说明 | ✓ |

**User's choice:** cargo/crates.io convention

---

## value_hint 分配

| Option | Description | Selected |
|--------|-------------|----------|
| 仅 --config | FilePath hint | |
| config + output | FilePath for both | |
| 全部参数 | 所有参数加 value_hint | |
| 不加 | 用户明确不要 | ✓ |

**User's choice:** Do NOT add value_hint — user explicitly declined

---

## 管道示例时机

| Option | Description | Selected |
|--------|-------------|----------|
| 现在加 | 提前展示 stdin 示例 | |
| 预留位置 | Phase 35 预留，Phase 37 补上 | ✓ |
| 不加 | 只展示当前功能 | |

**User's choice:** Reserve placement, add actual content in Phase 37

---

## 配置文档引用

| Option | Description | Selected |
|--------|-------------|----------|
| 不加 | --help 独立完整 | |
| 加一句 | `See config.toml` | |
| 加简短引用 + 段简介 | 列出 [csv]/[sqlite]/[pipeline] | ✓ |

**User's choice:** Brief reference with section descriptions

---

## Claude's Discretion

- Exact wording of help text and examples
- Specific example command arguments
- Exact placement of config section reference in help output

## Deferred Ideas

None
