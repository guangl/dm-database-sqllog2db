# Phase 42: Criterion 基准测试基础设施 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-24
**Phase:** 42-Criterion 基准测试基础设施
**Areas discussed:** Parser benchmark 位置

---

## Parser 原始解析速度场景位置

| Option | Description | Selected |
|--------|-------------|----------|
| 新增 bench_parser.rs | 独立文件，专测 dm-database-parser-sqllog 的解析吞吐量（不含导出）。目标: records/sec 或 MB/s，不依赖外部文件。 | ✓ |
| 折入 bench_filters.rs | 在现有 bench_filters.rs 里加 parser_raw 组，保持文件数不变。bench_filters 的 no_pipeline 现在测的是 parse+CSV export，不是纯解析。 | |

**User's choice:** 新增 bench_parser.rs
**Notes:** bench_filters.rs 的 no_pipeline 场景包含 CSV 导出，与"parser 原始解析速度"语义不同，需独立文件。

---

## Claude's Discretion

- bench_parser.rs 内部 benchmark group 命名
- 是否覆盖多规模（1K/10K/100K records）

## Deferred Ideas

- 真实文件 benchmark 扩展 → 已在现有 bench 中作为可选，不扩展
- CI 集成 → Phase 45
