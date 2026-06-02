# Phase 61: Cross.toml SHA 固定 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-03
**Phase:** 61-cross-toml-sha
**Mode:** --auto (all choices auto-selected)
**Areas discussed:** SHA 获取方式, 注释格式

---

## SHA 获取方式

| Option | Description | Selected |
|--------|-------------|----------|
| docker manifest inspect | 使用 docker CLI 获取 digest | ✓ |
| GitHub Container Registry API | 使用 HTTP API | |

**Auto-selected:** docker manifest inspect (recommended default)
**Notes:** Cross.toml 中已有该命令说明，与现有文档一致

---

## 注释格式

| Option | Description | Selected |
|--------|-------------|----------|
| 仅记录日期 | `# Pinned YYYY-MM-DD` | |
| 日期 + 更新命令 | 包含如何更新的说明 | ✓ |

**Auto-selected:** 日期 + 更新命令 (recommended default)

---

## Claude's Discretion

- dry-run 验证方式（cross --dry-run 或文档化限制）
- docker 不可用时的备选命令（skopeo）

## Deferred Ideas

- 换用非 cross-rs 镜像 — Out of Scope (REQUIREMENTS.md)
- Renovate/Dependabot Docker 自动更新 — 后续里程碑
