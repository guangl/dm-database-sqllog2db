# Phase 48: 日志级别与运行提示 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-31
**Phase:** 48-日志级别与运行提示
**Areas discussed:** verbose/quiet 设计

---

## --verbose 标志重新定位

| Option | Description | Selected |
|--------|-------------|----------|
| 新增独立 --verbose 布尔标志，保留 -v 控制日志级别 | 两个功能各司其职 | |
| 复用 -v 为 --verbose，移除 debug/trace 功能 | 简化标志，损失调试能力 | ✓ |
| 只加 --quiet，不加 --verbose，保持 -v | 最小改动 | |

**User's choice:** 复用 -v 为 --verbose，移除 debug/trace 日志级别功能
**Notes:** 用户认为运行时输出控制比日志级别切换更有价值。移除 -vv trace 功能。

---

## verbose 与进度条的交互

| Option | Description | Selected |
|--------|-------------|----------|
| verbose 时禁用进度条，改用逐行输出 | 避免 stderr 干扰 | ✓ |
| verbose + 进度条共存，用 MultiProgress | 视觉丰富但已列入 Out of Scope | |
| verbose 时保留进度条，额外输出在其下方 | indicatif 可能遮挡部分输出 | |

**User's choice:** verbose 时禁用进度条，改用逐行输出（推荐）
**Notes:** MultiProgress 已在 PROJECT.md 列为 Out of Scope，不采用。

---

## --quiet 与进度条

| Option | Description | Selected |
|--------|-------------|----------|
| 创建时就不创建 ProgressBar | quiet=true 时完全跳过 ProgressBar::new() | ✓ |
| 创建后用 set_draw_target(hidden) 隐藏 | 接口不变但实现稍复杂 | |

**User's choice:** 创建时就不创建 ProgressBar（推荐）
**Notes:** 最干净，无 ANSI 控制码泄漏风险。

---

## Claude's Discretion

- verbose 模式下过滤器匹配详情的粒度（文件级 vs 记录级）由 planner 根据性能影响决定
- verbose 模式摘要的具体字段格式由 planner 设计

## Deferred Ideas

无
