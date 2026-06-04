# Phase 64: CSV 并行路径基础设施 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-04
**Phase:** 64-csv
**Areas discussed:** 实现方案（channel vs temp-file）、自动切换条件、内存模型

---

## 实现方案选择

| Option | Description | Selected |
|--------|-------------|----------|
| Channel 写入线程 | 每个 rayon 线程通过 mpsc channel 发送记录给唯一写入线程，避免临时文件 | |
| temp-file 方案（现有） | 每线程独立写临时 CSV，最终按顺序拼接 | ✓ |

**Auto-selected:** temp-file 方案（现有实现）
**Notes:** `src/cli/run/parallel.rs` 已完整实现 temp-file 方案，ROADMAP 中 "channel" 是设计意图示例，不要求强制实现。temp-file 方案已通过 Phase 59/60 代码审查和修复。

---

## 自动切换条件

| Option | Description | Selected |
|--------|-------------|----------|
| 现有条件已满足 SC1/SC4 | `jobs > 1 && len > 1 && !stdin && csv.is_some()` | ✓ |
| 需要修改切换逻辑 | 添加额外条件或配置项 | |

**Auto-selected:** 现有条件已满足
**Notes:** mod.rs 切换逻辑与 SC1/SC4 完全对应，无需改动。

---

## 内存模型

| Option | Description | Selected |
|--------|-------------|----------|
| 接受逐文件收集（现有） | 每线程 Vec 缓冲单文件记录，写入临时 CSV 后释放 | ✓ |
| 改为流式写入 | 边解析边写入临时 CSV，避免 Vec 缓冲 | |

**Auto-selected:** 接受现有模型
**Notes:** 对 300MB 文件（~1M 条记录），Vec 缓冲可接受。流式改造留后续里程碑评估。

---

## Claude's Discretion

- Phase 64 执行重点为验证（cargo test/clippy），而非修改代码
- 若发现任何 SC 不满足，报告而非自行修改架构

## Deferred Ideas

- channel 写入线程架构 — 更低内存，但复杂度高，留后续按需评估
- per-file 进度显示 — Phase 65 负责
