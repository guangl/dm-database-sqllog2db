# Phase 44: 热路径与内存优化 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-24
**Phase:** 44-热路径与内存优化
**Areas discussed:** macOS 内存分析工具

---

## 内存分析工具选择

| Option | Description | Selected |
|--------|-------------|----------|
| jemalloc + 统计接口 | tikv-jemallocator 替换全局 allocator，通过 tikv-jemalloc-ctl 读取峰值堆分配。不依赖外部工具，CI 可集成，结果可比较。 | ✓ |
| Apple Instruments / Heaptrack via Docker | macOS 原生 Instruments 或 heaptrack（需 Docker/Linux VM）。可视化效果好，但 CI 难集成，结果不易自动比较。 | |
| 简化：只用 criterion 吞吐量指标 | Phase 44 成功标准里堆分配减少是必要条件，但如果记录方式灵活（注释说明 or 工具运行截图），可先跑通，后续补正式监控。 | |

**User's choice:** jemalloc + 统计接口
**Notes:** 优先选择可程序化、CI 友好的方案。tikv-jemallocator 作为 dev/bench 依赖，不影响 release binary（除非性能收益明显）。

---

## Claude's Discretion

- 具体优化手段（内联、预分配 buffer、避免 clone 等）由 profiling 结果决定
- 是否在 benchmark 中直接集成 jemalloc 统计，或作为独立测试

## Deferred Ideas

- SIMD 解析加速 → 过度工程
- mimalloc → 超出本次范围
