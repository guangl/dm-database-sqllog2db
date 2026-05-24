# Phase 41: 依赖升级与 Parser 库适配 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-24
**Phase:** 41-依赖升级与 Parser 库适配
**Areas discussed:** Parser 版本目标策略

---

## Parser 版本策略

| Option | Description | Selected |
|--------|-------------|----------|
| 直接升到 2.0.0 | Phase 41 完成基础升级 + 编译通过，Phase 43 再做深度 API 适配。符合里程碑节奏。 | ✓ |
| 只升到最高 1.x patch | 保守策略：先确认 1.x 有更新的 patch/minor，仅做 cargo update 级别的升级，Phase 43 API 重构空间会小很多。 | |
| 先调研再决定 | 读 2.0.0 changelog/文档，确认 API 变更范围后再定策略。研究员阶段再做。 | |

**User's choice:** 直接升到 2.0.0
**Notes:** 两阶段策略：Phase 41 升级编译，Phase 43 深度适配。

---

## Claude's Discretion

- 如果 2.0.0 有编译级 breaking changes，做最小化适配使编译通过，记录 TODO 供 Phase 43 参考。

## Deferred Ideas

- AsyncLogParser tokio 异步接口 → 超出本 milestone 范围
- 利用 FilterBuilder 全量重构 → Phase 43
