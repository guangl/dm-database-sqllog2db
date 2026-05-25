# Phase 43: Parser 新 API 适配与 Filter 重构 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-24
**Phase:** 43-Parser 新 API 适配与 Filter 重构
**Areas discussed:** Filter 重构粒度

---

## Filter 模块重构方式

| Option | Description | Selected |
|--------|-------------|----------|
| 函数边界：同文件内独立函数 | prescan.rs 和 filter_processor.rs 已有一定分离。在 compiled.rs/mod.rs 内把 pre-scan 相关方法与 main-pass 方法用注释块区隔清楚即可，不拆子模块。 | ✓ |
| 模块边界：拆 pipeline/filters/prescan.rs 子模块 | 把编译过滤器里的 pre-scan 逻辑单独提取为 pipeline/filters/prescan.rs，职责更明确，但需要调整 pub/pub(crate) 可见性。 | |

**User's choice:** 函数边界：同文件内独立函数
**Notes:** 保持模块结构不变，通过注释 section 和函数命名清晰表达边界，降低重构风险。

---

## Claude's Discretion

- 注释 section 格式（`// === Pre-scan ===` 风格）
- FilterBuilder 是否用于全量替代 CompiledMetaFilters（以"减少冗余"为准，不强制）

## Deferred Ideas

- AsyncLogParser 异步接口 → 超出本 milestone 范围
- FilterBuilder 全量迁移 → 仅删冗余即可
