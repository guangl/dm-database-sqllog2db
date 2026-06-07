# Phase 66: 兼容性验证与测试 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-04
**Phase:** 66-compat
**Areas discussed:** 集成测试结构、顺序基线构建、config.toml 验证

---

## 集成测试结构（COMPAT-02）

| Option | Description | Selected |
|--------|-------------|----------|
| 顺序拼合 vs 并行，行集合排序对比 | 各文件顺序运行取数据行，合并排序；并行运行取数据行排序；断言相等 | ✓ |
| 字节级完全一致断言 | 要求顺序与并行输出完全相同（包括行顺序） | |

**Auto-selected:** 排序后行集合对比
**Notes:** 并行路径文件间行顺序不确定（文件级 rayon work-stealing），字节级对比不可行。排序后集合对比是正确策略，ROADMAP SC1 已明确"忽略文件间行顺序"。

---

## 顺序基线构建

| Option | Description | Selected |
|--------|-------------|----------|
| 各文件单独运行，读取各 CSV，合并行 | 每个文件单独 handle_run，读取输出，汇总数据行 | ✓ |
| Append 模式拼接 | 多次运行 append 到同一文件 | |

**Auto-selected:** 各文件单独运行后合并
**Notes:** Append 模式有多余 header 处理复杂性，各自读取后合并更清晰。

---

## Config.toml 格式验证（COMPAT-03）

| Option | Description | Selected |
|--------|-------------|----------|
| 现有 init 测试覆盖 + 补充"无新字段"断言 | test_init_template_has_csv_*_comment 已有，补充 grep 不含 parallel/jobs | ✓ |
| 全文件 diff 基线对比 | 与 v1.16 基线文件逐字节对比 | |

**Auto-selected:** 现有测试 + 轻量补充断言
**Notes:** 全文件 diff 需维护基线文件，成本高。轻量断言（不含并行相关新字段）足够满足 SC3。

---

## Claude's Discretion

- 测试数据量：每文件 20 条（2×20=40 条总量），比 10 条更能暴露行顺序问题
- 测试命名遵循现有模式：`test_parallel_csv_content_matches_sequential` 等

## Deferred Ideas

- 内存基准测试 — 可选，不列入必要工作
- property-based 测试（随机记录集并行 vs 顺序）— 超出范围
