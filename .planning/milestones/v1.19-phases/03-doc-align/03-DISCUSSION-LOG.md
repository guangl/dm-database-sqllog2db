# Phase 3: 文档与验证对齐 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-07
**Phase:** 3-文档与验证对齐
**Mode:** --auto (fully autonomous)
**Areas discussed:** VALIDATION.md 重建方式, README 新增内容结构, --help 示例补充

---

## VALIDATION.md 重建方式

| Option | Description | Selected |
|--------|-------------|----------|
| 从 SUMMARY.md 重建 | 从各 phase SUMMARY.md 的 self-check/requirements-completed 内容重建，标记 status: complete | ✓ |
| 重新运行验证 | 实际运行命令并记录输出（更准确但耗时） | |

**Auto-selection:** 从 SUMMARY.md 重建（recommended default）
**Notes:** Phase 67/68/69/70 均有完整 SUMMARY.md，self-check: PASSED 已记录，无需重跑命令。Phase 70 需新建 VALIDATION.md。

---

## README 新增内容结构

| Option | Description | Selected |
|--------|-------------|----------|
| 功能特性更新 + 新增说明段落 | 更新 CLI 条目（5 命令），在适当位置新增 watch/init --interactive/进度选项说明 | ✓ |
| 新建独立"子命令参考"章节 | 结构更清晰但 diff 更大 | |

**Auto-selection:** 功能特性更新 + 新增说明段落（recommended default）
**Notes:** minimal diff 原则，与现有 README 风格保持一致。

---

## --help 示例补充

| Option | Description | Selected |
|--------|-------------|----------|
| watch: quiet 示例; validate: verbose 示例 | 各补充 1 个，达到 ≥2 要求；stats 已有 3 个不改动 | ✓ |
| 补充更多示例 | watch/validate 各加 2-3 个 | |

**Auto-selection:** 各补充 1 个（recommended default）
**Notes:** stats `after_help` 在 opts.rs 已有 3 个示例（default top-20、--top 5、时间范围过滤），满足 DOC-05 要求。

---

## Claude's Discretion

- README 新增内容的具体位置（章节内顺序）由 planner 决定
- VALIDATION.md 任务粒度（按 plan vs 按 requirement）由 planner 参考 Phase 01/02 格式决定

## Deferred Ideas

- README 英文版
- CHANGELOG.md v1.19 条目（留 planner 判断是否属于 DOC-04 范围）
- run/init --help 示例进一步丰富（超出 DOC-05 要求）
