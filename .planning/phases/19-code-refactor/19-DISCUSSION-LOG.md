# Phase 19: 代码结构重构 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-17
**Phase:** 19-代码结构重构
**Areas discussed:** 文件拆分粒度, 字段投影共用逻辑, Exporter trait 统一范围, 可见性收紧范围

---

## 文件拆分粒度

| Option | Description | Selected |
|--------|-------------|----------|
| filters.rs 优先（1481 行） | 过滤器逻辑最复杂，拆分收益最高 | |
| config/mod.rs 优先（1418 行） | 影响 Phase 20 测试覆盖 | |
| 全部同等优先 | 5 个文件都超过 1000 行，统一拆分，不分先后 | ✓ |
| 只拆超 400 行的子职责部分 | 改动最小，不追求把大文件完全拆完 | |

**User's choice:** 全部同等优先

---

## 子模块行数目标

| Option | Description | Selected |
|--------|-------------|----------|
| 合理即可，不设硬性上限 | 按职责边界自然切分 | |
| 上限 400 行 | 与函数 40 行限制对应，模块可宽松些 | |
| 上限 300 行（ROADMAP 描述） | ROADMAP 原文"原超过 300 行的源文件" | ✓ |

**User's choice:** 上限 300 行

---

## 拆分后目录结构

| Option | Description | Selected |
|--------|-------------|----------|
| 就地拆为子模块（mod.rs + 子文件） | 通过 re-export 保持外部路径不变 | ✓ |
| 展平至同级目录（多个独立文件） | 目录层次不变浅 | |

**User's choice:** 就地拆为子模块

---

## 拆分时是否同时收紧可见性

| Option | Description | Selected |
|--------|-------------|----------|
| 对外接口不变，只重组内部 | 最安全 | |
| 拆分的同时收紧可见性 | 一次改动，避免二次扫描 | ✓ |

**User's choice:** 拆分的同时收紧可见性

---

## 字段投影共用逻辑位置

| Option | Description | Selected |
|--------|-------------|----------|
| 只提取共用类型/常量 | FIELD_NAMES 等常量保持原位 | |
| 提取共用投影辅助函数（如计算选定列名） | project_fields() 返回 &[&str]，CSV 和 SQLite 共用 | ✓ |
| 保持现状，不追求共用 | 两者实现实质不同 | |

**User's choice:** 提取共用投影辅助函数

---

## 共用辅助函数放置位置

| Option | Description | Selected |
|--------|-------------|----------|
| exporter/mod.rs（内联） | 不新增文件，但 mod.rs 已 756 行 | |
| exporter/projection.rs（新文件） | 职责清晰，库层和导出器都可引用 | ✓ |

**User's choice:** exporter/projection.rs（新文件）

---

## Exporter trait 特化分支清理范围

| Option | Description | Selected |
|--------|-------------|----------|
| 只清理默认实现中的冗余 match 分支 | export_one_normalized/preparsed 的退化调用链 | |
| DryRunExporter 整合进 ExporterKind | 消除独立 impl Exporter | |
| 两者都做 | 同时清理默认实现冗余和 DryRunExporter 特化 | ✓ |

**User's choice:** 两者都做

---

## Exporter trait 是否新增方法

| Option | Description | Selected |
|--------|-------------|----------|
| 不新增 trait 方法 | 只清理已有方法的特化冗余 | |
| 按需提升内部方法 | 如投影共用逻辑需要时提升相关方法 | ✓ |

**User's choice:** 按需提升内部方法

---

## 可见性收紧范围

| Option | Description | Selected |
|--------|-------------|----------|
| 全面收紧：所有不需要 pub 的均改为 pub(crate)/pub(super) | 系统性扫描全部源码 | ✓ |
| 重点收紧：只处理拆分文件内的漏出项 | 实际效果安全 | |
| 只收紧 filters.rs 中的字段（REFACTOR-04 明确目标） | 最小改动 | |

**User's choice:** 全面收紧

---

## 可见性收紧判断标准

| Option | Description | Selected |
|--------|-------------|----------|
| 测试用到则保留 pub | 跨模块测试访问的项保留 pub 或改 pub(crate) | |
| 只看跨模块访问（同模块内不算） | 同模块内用 pub(super)，跨模块用 pub(crate) | ✓ |

**User's choice:** 只看跨模块访问（同模块内不算）

---

## Claude's Discretion

- 子模块的具体命名（如 `filters/record.rs` vs `filters/meta.rs`）
- `run.rs` 的拆分边界（如按"预扫描/主循环/并行路径"拆）
- 是否顺带修复 CONCERNS.md 中低风险 tech debt（不引入行为变化的前提下）

## Deferred Ideas

- `ResumeState.processed` 改为 HashMap（性能影响只在数千文件场景，超出重构范畴）
- SQLite PRAGMA 崩溃安全性改进（行为变化，超出本 Phase 范围）
