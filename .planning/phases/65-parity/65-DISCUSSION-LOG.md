# Phase 65: 行为等价性保障 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-04
**Phase:** 65-parity
**Areas discussed:** IO-01（BufReader/mmap）、verbose 逐文件输出、等价性验证策略

---

## IO-01 BufReader 缓冲区

| Option | Description | Selected |
|--------|-------------|----------|
| 在 collector.rs 包装 BufReader | 自定义 64KB BufReader 包装库的 file 读取 | |
| 确认 mmap 已满足，无需修改 | 库使用 memmap2::Mmap，效果等于无限缓冲区 | ✓ |

**Auto-selected:** 确认 mmap 已满足
**Notes:** dm-database-parser-sqllog 的 LogParser::from_path 使用 Mmap::map，完全绕过 BufReader。mmap 减少系统调用的效果远超 64KB BufReader。IO-01 已由库架构满足。

---

## Verbose 逐文件输出（PARALLEL-05）

| Option | Description | Selected |
|--------|-------------|----------|
| 在 run_parallel_tasks 添加 eprintln | verbose=true 时每任务开始输出文件路径 | ✓ |
| 保持现有"N files in parallel"汇总输出 | 不修改，接受缺少逐文件 verbose | |

**Auto-selected:** 添加 per-file eprintln
**Notes:** SC3 明确要求"输出每个文件的处理进度"，需要修改 `run_parallel_tasks` 并透传 verbose 参数。

---

## 等价性验证策略（PARALLEL-03/04）

| Option | Description | Selected |
|--------|-------------|----------|
| 代码审查（共享 collector 架构保证） | 确认并行/顺序调用同一函数，等价性由架构保证 | ✓ |
| 运行时对比测试 | 同输入并行 vs 顺序对比，验证字节一致 | |

**Auto-selected:** 代码审查确认（运行时对比留 Phase 66）
**Notes:** collector::collect_log_file 和 CsvExporter 被并行/顺序路径共享使用，等价性有架构保证。运行时对比是 Phase 66 集成测试的工作。

---

## Claude's Discretion

- verbose 参数传递方式：直接传 bool 参数（与 sqlite_parallel.rs 对称），不通过 Config
- SC4 BufReader 记录方式：在 PLAN 任务中添加分析记录，不需要在代码中添加注释

## Deferred Ideas

- MultiProgress 多行进度条 — 过度工程，留后续里程碑
- 运行时内存基准对比 — Phase 66 若有余力可加
