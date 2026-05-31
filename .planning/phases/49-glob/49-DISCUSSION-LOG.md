# Phase 49: Glob 输入支持 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-31
**Phase:** 49-Glob 输入支持
**Areas discussed:** config 输入格式, CLI --input 设计

---

## config schema 变更策略

| Option | Description | Selected |
|--------|-------------|----------|
| path 保留 + 新增 inputs 数组共存 | 完全向后兼容 | |
| path 重命名为 input，支持字符串或数组 | 新标签名，serde untagged | |
| 破坏性变更：移除 path，只用 inputs 数组 | 强制升级 | ✓ |

**User's choice:** 破坏性变更：移除 path，只用 inputs 数组
**Notes:** 用户接受破坏性变更，配合旧键检测给出迁移 hint。

---

## 旧 path 键的处理方式

| Option | Description | Selected |
|--------|-------------|----------|
| 检测到 path 时告警 + 继续 | 平滑迁移但可能造成混乱 | |
| 直接错误：发现 path 就返回配置错误 | 明确且快速 | ✓ |
| 不处理，直接忽略 path 字段 | 用户可能丢失输入配置 | |

**User's choice:** 直接错误：发现 path 就返回配置错误
**Notes:** 配合 hint 提供迁移示例，用户必须主动迁移配置。

---

## CLI --input 标志设计

| Option | Description | Selected |
|--------|-------------|----------|
| 可重复 flag：--input f1.log --input 'dir/*.log' | clap append action，Unix 惯例 | ✓ |
| 单个 --input，逗号分隔列表 | 实现简单但逗号歧义 | |
| --input 覆盖 config inputs，两者不共存 | 简化优先级逻辑 | |

**User's choice:** 可重复 flag（推荐）
**Notes:** `clap::ArgAction::Append`，与 `-i f1.log -i f2.log` 等效。

---

## --input 与 config inputs 的优先级

| Option | Description | Selected |
|--------|-------------|----------|
| --input 覆盖 config inputs | CLI 参数始终优先 | ✓ |
| --input 与 config inputs 合并 | 更灵活但行为难预测 | |

**User's choice:** --input 覆盖 config inputs
**Notes:** 符合最小惊奇原则（CLI 参数覆盖配置文件）。

---

## Claude's Discretion

- `SqllogParser::new()` 接口如何调整以支持 `Vec<String>`（单参数改列表 vs 保留单参数接口在调用方合并）由 planner 决定

## Deferred Ideas

无
