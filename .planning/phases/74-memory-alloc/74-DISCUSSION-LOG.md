# Phase 74: 内存与分配优化 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-09
**Phase:** 74-内存与分配优化
**Mode:** --auto (autonomous, no user prompts)
**Areas discussed:** MEM-01 HashMap key 消除策略, MEM-02 line_buf 初始容量

---

## MEM-01: HashMap key 消除策略

| Option | Description | Selected |
|--------|-------------|----------|
| 二级 HashMap | `HashMap<String, HashMap<String, Arc<Vec<ParamValue>>>>` — lookup 完全零分配，最简实现 | ✓ |
| 自定义 Borrow impl | 保持平铺 HashMap，实现自定义 Borrow trait — 代码复杂，需 unsafe 或 nightly | |
| `Arc<str>` key | 改 key 为 `Arc<str>` — 查询时 `Arc::from(&str)` 仍分配新 Arc，不彻底 | |

**Auto-selected:** 二级 HashMap（推荐默认）
**Notes:** 二级 HashMap 是三种方案中最简单且效果最彻底的：lookup 路径完全不分配，insert 路径（PARAMS 记录）仍 clone 但频率远低于执行记录热路径。自定义 Borrow impl 复杂且 std 不支持 `(String,String): Borrow<(&str,&str)>`；`Arc<str>` 方案因 `Arc::from(&str)` 的 copy-on-create 语义仍会分配。

---

## MEM-02: line_buf 初始容量

| Option | Description | Selected |
|--------|-------------|----------|
| 4096 字节 | 覆盖典型 DaMeng SQL（1–4KB），减少冷启动 grow | ✓ |
| 保持 2048 | 仅加注释，不改数值 — 无法避免中等 SQL 的首次 grow | |
| 8192 字节 | 保守值，大多数 SQL 肯定覆盖，但初始内存略多 | |

**Auto-selected:** 4096 字节（推荐默认）
**Notes:** ROADMAP success criteria #2 提到"如 512 字节"作为下限示例，但实际 DaMeng SQL 通常 1–4KB，4096 是更实用的平衡点。writer.rs 的动态 reserve（`needed = 128 + sql.len() + ns_len`）已正确处理超出初始容量的情况，保留不变。

---

## Claude's Discretion

- PARAMS insert 路径 `entry` API 的具体实现细节（`entry().or_default()` vs 先 `contains_key` 检查）
- 是否为二级 HashMap 的 inner map 为空的边界场景补充单元测试

## Deferred Ideas

- heaptrack/massif 峰值内存 profiling（PROF-02）— 需真实大文件环境，Future phase
- flamegraph CPU 热点分析（PROF-01）— Future phase
- normalizer PARAMS insert 路径 intern pool 优化 — 投入产出比低，defer
