# Phase 44: 热路径与内存优化 - Context

**Gathered:** 2026-05-24
**Status:** Ready for planning

<domain>
## Phase Boundary

通过 profiling 定位并优化热路径瓶颈，使单线程 CSV 导出吞吐量超越 v1.10 基线（1.55M records/sec）；同时使用 jemalloc 统计接口量化并减少处理 1GB+ 文件时的峰值堆分配。不引入新 unsafe 代码（或有文档注释说明安全性）。

</domain>

<decisions>
## Implementation Decisions

### 内存分析工具
- **D-01:** 使用 `tikv-jemallocator` 替换全局 allocator，通过 `tikv-jemalloc-ctl` 读取峰值堆分配统计。
- **D-02:** jemalloc 统计接口在测试或 benchmark 中集成，输出 peak heap 数值，与 v1.10 基线对比（可 diff 验证）。
- **D-03:** jemalloc 仅作为 dev/bench 依赖使用，release binary 保持原有 allocator（不强制要求用 jemalloc 替换生产 allocator，除非性能收益明显）。

### 性能 Profiling
- **D-04:** 使用 `cargo flamegraph`（profile=flamegraph，已在 Cargo.toml 中定义）做热路径定位，或通过 criterion benchmark 结合 profiling 注释定位瓶颈。
- **D-05:** 优先考虑以下已知热路径：字符串分配（`String` clone/format）、`Vec` 重分配、正则匹配开销。

### 优化约束
- **D-06:** 不引入新的 `unsafe` 代码；如有特殊情况，必须有注释说明安全性理由（满足 Phase 44 验收标准 #4）。
- **D-07:** 所有现有测试（`cargo test`）必须继续通过，无功能回归。

### 验收量化
- **D-08:** `cargo bench --bench bench_csv` 显示吞吐量高于 1.55M records/sec（criterion 输出"Performance has improved"或绝对值超越）。
- **D-09:** jemalloc 统计显示处理 1GB+ 文件时峰值堆分配低于 v1.10 基线（具体基线值由研究员从 benches/baselines/ 确认）。

### Claude's Discretion
- 具体优化手段（内联展开、预分配 buffer、避免 clone 等）由分析结果决定
- 是否在 bench_csv.rs / bench_parser.rs 中直接集成 jemalloc 统计（或作为独立测试）

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 热路径代码（必读）
- `src/cli/run/processor.rs` — 单文件处理主循环，最可能的热路径
- `src/cli/run/mod.rs` — 整体调度逻辑，parallel 路径判断
- `src/exporter/mod.rs` — ExporterManager，CSV 写入路径
- `src/pipeline/mod.rs` — Pipeline::process，过滤器调用链

### 性能基础设施
- `benches/BENCHMARKS.md` — v1.10 基线说明，`CRITERION_HOME` 用法
- `benches/baselines/` — 已有基线数据（JSON）
- `Cargo.toml` — `[profile.flamegraph]` 已定义，`criterion` dev-dep 已有

### Requirements
- `.planning/REQUIREMENTS.md` §PERF-01, §PERF-02

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `[profile.flamegraph]` — 已在 Cargo.toml 定义，`debug=true strip=none`，直接可用
- `benches/baselines/` — 历史基线 JSON，用于对比

### Established Patterns
- 16MB BufWriter 已在 CSV 导出中使用（CLAUDE.md 记录）
- `itoa` crate 零分配整数格式化（已用）
- `pipeline.is_empty()` fast path 已有 — 优化时不要破坏此路径

### Integration Points
- Phase 42 新增的 bench_parser.rs 可作为 parser 热路径优化的量化工具
- Phase 43 重构后的 filter 边界可能暴露新的优化点

</code_context>

<specifics>
## Specific Ideas

- jemalloc 统计读取方式：`tikv_jemalloc_ctl::epoch::mib().read()` + `stats::allocated::mib().read()` 获取当前分配量
- 如果需要量化"处理 1GB+ 文件"，可用 bench_csv.rs 的 real-file benchmark（需要 sqllogs/ 目录）

</specifics>

<deferred>
## Deferred Ideas

- SIMD 解析加速 → 过度工程，不在本 milestone 范围
- 多线程 allocator（如 mimalloc）→ 超出本次优化范围

</deferred>

---

*Phase: 44-热路径与内存优化*
*Context gathered: 2026-05-24*
