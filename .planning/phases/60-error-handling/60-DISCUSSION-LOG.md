# Phase 60: 错误处理路径统一 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-03
**Phase:** 60-error-handling
**Mode:** --auto (all choices auto-selected)
**Areas discussed:** map_err 替换策略, unwrap/expect 处理方式, From impl 位置

---

## map_err 替换策略

| Option | Description | Selected |
|--------|-------------|----------|
| 全部替换为 `?` | 所有 map_err 改为 `?` + From | |
| 按需保留 | 携带上下文的 map_err 保留，仅类型转换的替换为 `?` | ✓ |

**Auto-selected:** 按需保留 (recommended default)
**Notes:** 携带 `path`/`reason` 字段的 FileError/ExportError 构造无法用 From 表达，必须保留 map_err

---

## unwrap/expect 处理方式

| Option | Description | Selected |
|--------|-------------|----------|
| 全部重构为 Option/Result | 消除所有 unwrap | |
| 加注释 | 不可失败的加注释，可失败的替换 | ✓ |

**Auto-selected:** 加注释 (recommended default)
**Notes:** `write!(String)` 等确实不可失败，注释比重构更清晰

---

## From impl 位置

| Option | Description | Selected |
|--------|-------------|----------|
| 分散到各模块 | 每个模块维护自己的 From | |
| 集中在 src/error.rs | 已有结构，保持集中 | ✓ |

**Auto-selected:** 集中在 src/error.rs (recommended default)

---

## Claude's Discretion

- logging.rs:60 的注释内容
- 整理顺序（parallel → logging → unwrap 注释）

## Deferred Ideas

- 引入 anyhow/color-eyre — 后续里程碑
- 合并 FileError/ExportError::WriteFailed 变体 — 接口重构
