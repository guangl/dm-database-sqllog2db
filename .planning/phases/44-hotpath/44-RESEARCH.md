# Phase 44: 热路径与内存优化 - Research

**Researched:** 2026-05-24
**Domain:** Rust 热路径优化、内存分析（jemalloc）、criterion benchmark
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** 使用 `tikv-jemallocator` 替换全局 allocator，通过 `tikv-jemalloc-ctl` 读取峰值堆分配统计。
- **D-02:** jemalloc 统计接口在测试或 benchmark 中集成，输出 peak heap 数值，与 v1.10 基线对比（可 diff 验证）。
- **D-03:** jemalloc 仅作为 dev/bench 依赖使用，release binary 保持原有 allocator（不强制要求用 jemalloc 替换生产 allocator，除非性能收益明显）。
- **D-04:** 使用 `cargo flamegraph`（profile=flamegraph，已在 Cargo.toml 中定义）做热路径定位，或通过 criterion benchmark 结合 profiling 注释定位瓶颈。
- **D-05:** 优先考虑以下已知热路径：字符串分配（`String` clone/format）、`Vec` 重分配、正则匹配开销。
- **D-06:** 不引入新的 `unsafe` 代码；如有特殊情况，必须有注释说明安全性理由。
- **D-07:** 所有现有测试（`cargo test`）必须继续通过，无功能回归。
- **D-08:** `cargo bench --bench bench_csv` 显示吞吐量高于 1.55M records/sec（criterion 输出"Performance has improved"或绝对值超越）。
- **D-09:** jemalloc 统计显示处理 1GB+ 文件时峰值堆分配低于 v1.10 基线（具体基线值由研究员从 benches/baselines/ 确认）。

### Claude's Discretion
- 具体优化手段（内联展开、预分配 buffer、避免 clone 等）由分析结果决定
- 是否在 bench_csv.rs / bench_parser.rs 中直接集成 jemalloc 统计（或作为独立测试）

### Deferred Ideas (OUT OF SCOPE)
- SIMD 解析加速 → 过度工程，不在本 milestone 范围
- 多线程 allocator（如 mimalloc）→ 超出本次优化范围
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PERF-01 | 解析热路径优化后，criterion benchmark 显示单线程吞吐量提升（相对 v1.10 基线 1.55M records/sec 有可量化改善） | 已识别具体分配热点；优化方案可量化；v1.10 基线已从 baselines/ 确认 |
| PERF-02 | 处理 1GB+ 日志文件时，jemalloc 统计显示峰值堆分配明显减少 | tikv-jemallocator/tikv-jemalloc-ctl 均在 crates.io 确认可用；peak 测量策略已明确 |
</phase_requirements>

---

## Summary

Phase 44 目标是在已有 LTO+strip+panic=abort 的 release profile 基础上，进一步在**应用层代码**找到可消除的分配开销和拷贝开销。通过深入读取热路径源码，识别出三处具体的 `String` clone 集中点（全在 `src/pipeline/normalizer.rs` 的 `compute_normalized` 函数），以及 `BufWriter` 容量与 CLAUDE.md 记录不一致的问题。

**v1.10 基线确认（phase33 tag）：** 合成 benchmark csv_export/10000 中位数 2.104ms（4.75 M records/s）。jemalloc 峰值堆基线目前无历史记录，需在 Wave 0 先行采集。

**D-08 约束解读：** 当前合成 benchmark 已超过 1.55M records/s 绝对阈值（4.75 M/s vs 1.55 M/s）。PERF-01 的实际约束是"相对 v1.10 有可量化改善"——即 criterion 相比 phase33 baseline 输出"Performance has improved"。

**Primary recommendation:** 先采集 jemalloc 峰值堆基线（Wave 0），再实施 `compute_normalized` 中的 clone 消除优化（Wave 1），最后验收两个 requirements。

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| ParamBuffer clone 消除 | Pipeline/Normalizer | — | `compute_normalized` 是分配热点所在层 |
| jemalloc 峰值统计 | Bench / Test | — | D-03 要求不影响生产 allocator |
| BufWriter 容量调优 | CSV Exporter | — | 直接影响 write 系统调用频率 |
| 热路径 profiling | [profile.flamegraph] | samply | `[profile.flamegraph]` 已在 Cargo.toml 定义 |
| criterion baseline 保存 | Bench 基础设施 | — | 保存新 phase44 baseline 供 PERF-01 对比 |

---

## Standard Stack

### Core（已在项目中，无需新增）

| 库 | 当前版本 | 用途 |
|----|---------|------|
| `criterion` | 0.7 | 性能 benchmark（已有）|
| `itoa` | 1.0 | 零分配整数格式化（已有）|
| `memchr` | 2 | SIMD 字节搜索（已有）|
| `samply` | 0.13.1 | flamegraph profiling（已有，`cargo install samply`）|

### 需新增的 Dev Dependencies

| 库 | 版本 | 用途 | 备注 |
|----|------|------|------|
| `tikv-jemallocator` | 0.6.1 | jemalloc 全局 allocator（bench/dev 用）| D-01，仅 dev-dep |
| `tikv-jemalloc-ctl` | 0.6.1 | 读取 jemalloc 统计（allocated bytes）| D-01/D-02 |

### 可选 Dev Dependencies（Claude's Discretion）

| 库 | 版本 | 用途 | 风险 |
|----|------|------|------|
| `rustc-hash` | 2.1.2 | `FxHashMap`，替换 `ParamBuffer` 的 `HashMap` 以降低哈希开销 | 低——rustc 自身使用；key 为 `(String, String)`，无密码学需求 |

**安装命令（Cargo.toml dev-dep 段）：**
```toml
[dev-dependencies]
tikv-jemallocator = "0.6.1"
tikv-jemalloc-ctl = "0.6.1"
```

---

## Package Legitimacy Audit

> slopcheck 在当前环境不可用，以下包均通过 `cargo search` 在 crates.io 注册表直接验证。

| Package | Registry | 验证方式 | slopcheck | Disposition |
|---------|----------|---------|-----------|-------------|
| `tikv-jemallocator` 0.6.1 | crates.io | `cargo search tikv-jemallocator` 输出 `"0.6.1"` | N/A | `[VERIFIED: crates.io]` — TiKV 项目维护，主流 Rust jemalloc 封装 |
| `tikv-jemalloc-ctl` 0.6.1 | crates.io | `cargo search tikv-jemalloc-ctl` 输出 `"0.6.1"` | N/A | `[VERIFIED: crates.io]` — 同上维护者，配套控制接口 |
| `rustc-hash` 2.1.2 | crates.io | `cargo search rustc-hash` 输出 `"2.1.2"` | N/A | `[VERIFIED: crates.io]` — rustc 本身使用，极高可信度 |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

*slopcheck 不可用，所有包通过 crates.io 直接验证（`cargo search` 输出与版本一致）。*

---

## 热路径代码审查：具体分配点

### 已确认的分配热点（从源码直接读取）

#### 热点 H-1：`compute_normalized` 中 PARAMS 记录的 key clone
**文件：** `src/pipeline/normalizer.rs` 第 350 行
```rust
// 当前代码（每条 PARAMS 记录 2 次 String clone）
buffer.insert((record.sess_id.clone(), record.statement.clone()), params);
```
**影响：** 每条 PARAMS 记录产生 2 次 `String::clone()` 堆分配（`sess_id` + `statement`）。

**优化方案：** 使用 `buffer.entry(key).or_insert(params)` 的 borrow 风格，但 `HashMap::insert` 需要拥有 key。真正的消除方式：
- 方案 A：先 `buffer.contains_key(&(sess_id_ref, stmt_ref))` — 但 `HashMap` 标准实现不支持借用 tuple key
- 方案 B：用 `Arc<str>` 作为 key，clone 只复制引用计数 [ASSUMED — 需实测效果]
- 方案 C：用格式化字符串合并 key（`format!("{}:{}", sess_id, statement)`）——一次分配代替两次，但不减少总分配 [ASSUMED]

#### 热点 H-2：`compute_normalized` 中 DML 记录的 key clone
**文件：** `src/pipeline/normalizer.rs` 第 367 行
```rust
// 当前代码（每条有 params 的 DML 记录 2 次 String clone）
let key = (record.sess_id.clone(), record.statement.clone());
```
**优化方案：** 使用借用 key 查找（`HashMap::get` 接受 `Borrow<K>`）。标准 `HashMap<(String, String), V>` 支持通过 `(&str, &str)` 借用查找，因为 `(String, String)` 实现了 `Borrow<(String, String)>` 但**不直接支持** `(&str, &str)` 的 borrow lookup。

实际可行路径：
```rust
// 用一个临时 String 作为 key，避免 tuple borrow 限制
// 或使用 .get_key_value() 的 raw entry API
// HashMap raw_entry（nightly-only）不可用
// 最简方案：接受 2 次 clone，聚焦其他优化
```
[ASSUMED — 需评估 HashMap raw entry 或改用 FxHashMap + 单字符串 key]

#### 热点 H-3：`compute_normalized` 中 `params` 的 Vec clone
**文件：** `src/pipeline/normalizer.rs` 第 369 行
```rust
// 当前代码（克隆整个 params Vec<ParamValue>）
let params = buffer.get(&key)?.clone();
```
**影响：** 每条匹配 DML 记录克隆一个 `Vec<ParamValue>`，包含 `Quoted(String)` 和 `Bare(String)` 内部的字符串。
**优化方案：** 将 `buffer` 改为存储 `Arc<Vec<ParamValue>>`，clone 时只复制 Arc 引用计数。
```rust
// 优化后（仅 Arc clone，O(1)）
let params = buffer.get(&key)?.clone(); // Arc<Vec<...>> 的 clone
apply_params_into(pm_sql, &params, colon_style, scratch);
```

#### 热点 H-4：`BufWriter` 容量不一致问题
**文件：** `src/exporter/csv/mod.rs` 第 124 行
```rust
let mut writer = BufWriter::with_capacity(2 * 1024 * 1024, file); // 2 MB
```
**CLAUDE.md 记录：** "16MB `BufWriter`"——与代码实际值不符。当前代码为 2MB。

CLAUDE.md 的 16MB 描述可能是历史残留（或曾经有过更大的 buffer 配置）。当前实际是 2MB。

**潜在优化：** 增大 BufWriter 可减少系统调用次数，但效果取决于 OS 页缓存是否已经足够高效。16MB 对于 1GB+ 文件处理可能更优，但需实测。[ASSUMED — 需 criterion 实测]

#### 热点 H-5：`write_record_preparsed` 中 `line_buf` 初始容量
**文件：** `src/exporter/csv/mod.rs` 第 51 行
```rust
line_buf: Vec::with_capacity(2048),
```
`line_buf` 在 `CsvExporter` 初始化时分配 2048 字节。实际 SQL 行可能更长。当 SQL 超过 2048 字节时触发重分配。

**当前缓解：** `write_record_preparsed` 第 41-44 行有动态容量检查：
```rust
let needed = 128 + sql_len + ns_len;
if line_buf.capacity() < needed {
    line_buf.reserve(needed - line_buf.len());
}
```
这确保了每条记录的分配次数最多为 O(log n)，但首次超过 2048 时仍需 realloc。对大 SQL 记录频繁的工作负载有效。

---

## v1.10 基线数据确认

### PERF-01 对应基线（合成 benchmark）

| Benchmark | v1.10 (phase33 tag) 中位数 | 吞吐量 |
|-----------|--------------------------|--------|
| `csv_export/1000` | 236,287 ns | 4.23 M records/s |
| `csv_export/10000` | 2,104,371 ns | **4.75 M records/s** |
| `csv_export/50000` | 10,459,958 ns | 4.78 M records/s |

**来源：** `benches/baselines/csv_export/*/phase33/estimates.json` 直接读取 `median.point_estimate`。[VERIFIED: 项目内 JSON 文件]

PERF-01 验收标准：`cargo bench --bench bench_csv` 对比 phase33 baseline，criterion 输出"Performance has improved"（即中位数下降且 p < 0.05）。

### PERF-02 对应基线（jemalloc 峰值堆）

**当前状态：无历史记录。** benches/baselines/ 中不含堆分配统计数据。需要在 Wave 0 以 jemalloc 测量当前（v1.10）峰值堆分配，将该数值作为 Wave 1 优化的对比基准。

**测量策略：**
```rust
// 1. 启用 jemalloc 作为 allocator（仅在 test/bench cfg 下）
#[cfg(any(test, bench))]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

// 2. 读取 epoch 强制刷新统计，再读 allocated
use tikv_jemalloc_ctl::{epoch, stats};
let e = epoch::mib().unwrap();
let allocated = stats::allocated::mib().unwrap();

e.advance().unwrap();                   // 刷新统计
let before = allocated.read().unwrap(); // 记录起点

// ... 处理 N 条记录 ...

e.advance().unwrap();
let after = allocated.read().unwrap();  // 记录终点
let delta = after.saturating_sub(before); // 此次处理的净分配量
```

注意：`stats::allocated` 是累计值（非当前），`delta` 反映处理过程中的**总堆分配量**，不是 RSS 峰值。这是在不用 heaptrack 的情况下最接近"堆分配压力"的指标。[ASSUMED — 若需要真正峰值需用 valgrind/heaptrack]

### parser_throughput 基线（Phase 42 采集）

| Benchmark | v1.0 (parser_throughput) 中位数 | 吞吐量 |
|-----------|--------------------------------|--------|
| `parser_throughput/1000` | 510,138 ns | 1.96 M records/s |
| `parser_throughput/10000` | 5,020,844 ns | 1.99 M records/s |
| `parser_throughput/50000` | 25,635,792 ns | 1.95 M records/s |

**来源：** `benches/baselines/parser_throughput/*/v1.0/estimates.json`。[VERIFIED: 项目内 JSON 文件]

---

## Architecture Patterns

### 系统架构图（热路径数据流）

```
LogParserBuilder (mmap + 解析)
    ↓ Sqllog struct (owned Strings: ts, sess_id, statement, sql, ...)
    ↓ 
processor.rs 热循环 (per record):
    ├─ pipeline.is_empty()? → 快速路径（无过滤器开销）
    ├─ pipeline.run_with_meta(&record) → CompiledMetaFilters (regex match)
    └─ compute_normalized(&record, sql, params_buffer, ...)
           ├─ PARAMS 记录: buffer.insert((sess_id.clone(), stmt.clone()), params)  ← H-1
           └─ DML 记录:   key=(sess_id.clone(), stmt.clone())  ← H-2
                          params = buffer.get(&key)?.clone()    ← H-3
                          apply_params_into(sql, &params, ..., scratch)
                          → None or Some(&str) pointing into scratch
    ↓
ExporterManager::export_one_preparsed
    └─ CsvExporter::export_one_preparsed
           └─ write_record_preparsed (line_buf Vec<u8>)
                  └─ BufWriter<File> (2MB capacity)  ← H-4
```

### 优化优先级排序

| 热点 | 预期收益 | 实现复杂度 | 优先级 |
|------|---------|-----------|--------|
| H-3：params Vec clone → Arc | 高（每条有参数 DML 记录 1 次 Vec clone） | 低（类型改变，无逻辑变化） | P1 |
| H-1/H-2：key String clone | 中（每条 PARAMS/DML 记录 2 次 String clone） | 中（需评估 HashMap key 策略） | P2 |
| H-4：BufWriter 容量调整 | 中（减少 write() 系统调用） | 低（单行改动） | P3 |
| FxHashMap 替换 | 低（哈希函数优化，但 key 为 String） | 低（类型别名替换）| P4 |

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 全局 allocator 替换 | 自定义 global_allocator 实现 | `tikv-jemallocator` | D-01 锁定 |
| jemalloc 统计读取 | 手动 syscall / mallinfo | `tikv-jemalloc-ctl` | 类型安全封装，已有 API |
| Flamegraph 生成 | 手写 perf_event | `cargo flamegraph` / `samply` | 已安装，`[profile.flamegraph]` 已定义 |
| CSV 转义 | 手写字节逐个检查 | `write_csv_escaped`（已有）| 已使用 `memchr::memchr` 优化 |

---

## Common Pitfalls

### Pitfall 1：jemalloc `cfg` 条件错误导致生产 binary 被替换
**What goes wrong:** 将 `#[global_allocator]` 放在不带 `cfg` 的位置，导致 release binary 也使用 jemalloc。
**Why it happens:** Rust `[dev-dependencies]` 不会自动隔离使用点，需手动 `#[cfg(test)]` 或 feature flag。
**How to avoid:** 使用 `cfg(any(test, bench))` 或专门的 `bench_memory` binary，不要在 `src/main.rs` 无条件注册。
**Warning signs:** `cargo build --release` 链接 `libjemalloc.a`。

### Pitfall 2：criterion baseline 比较路径错误
**What goes wrong:** `CRITERION_HOME` 未设置，criterion 不能找到 phase33 baseline，无法输出"Performance has improved"。
**Why it happens:** `benches/baselines/` 是自定义 baseline 目录，需显式指定。
**How to avoid:**
```bash
CRITERION_HOME=benches/baselines cargo bench --bench bench_csv -- --baseline phase33
```
**Warning signs:** criterion 输出不显示 "change:" 段落。

### Pitfall 3：HashMap borrow key 问题
**What goes wrong:** 尝试用 `(&str, &str)` 借用查找 `HashMap<(String, String), V>` 失败编译。
**Why it happens:** `(String, String)` 没有实现 `Borrow<(&str, &str)>`（Rust 标准库限制）。
**How to avoid:** 要么接受 clone（现状），要么将 key 改为单个 `String`（如 `"sess:stmt"`），要么用 raw entry API（nightly-only）。

### Pitfall 4：`Arc<Vec<ParamValue>>` 导致 `apply_params_into` 签名变化
**What goes wrong:** `apply_params_into` 接受 `&[ParamValue]`，`Arc<Vec<ParamValue>>` 通过 Deref 可自动转换为 `&[ParamValue]`，无需修改签名。
**Why it happens:** 实际上这里**不是**陷阱——Arc deref 正确工作。但要注意 `ParamBuffer` 的类型别名需同步修改。

### Pitfall 5：jemalloc epoch 未刷新导致读数陈旧
**What goes wrong:** 直接读 `stats::allocated` 不调用 `epoch::advance()`，返回上次 epoch 的旧数据。
**How to avoid:** 每次读取前务必调用 `epoch::advance().unwrap()`。

---

## Code Examples

### jemalloc 统计读取模式
```rust
// 在 bench 或 test 文件顶部（cfg 隔离）
#[cfg(any(test, bench))]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

// 测量函数
#[cfg(any(test, bench))]
fn measure_alloc_delta<F: FnOnce()>(f: F) -> usize {
    use tikv_jemalloc_ctl::{epoch, stats};
    let e = epoch::mib().unwrap();
    let a = stats::allocated::mib().unwrap();
    e.advance().unwrap();
    let before = a.read().unwrap();
    f();
    e.advance().unwrap();
    let after = a.read().unwrap();
    after.saturating_sub(before)
}
```
[ASSUMED — API 形态基于 CONTEXT.md D-01 specifics 和 tikv-jemalloc-ctl crate 文档描述]

### Arc<Vec<ParamValue>> 替换 clone 模式
```rust
// 修改前
pub type ParamBuffer = HashMap<(String, String), Vec<ParamValue>>;
// 热路径中
let params = buffer.get(&key)?.clone(); // 深拷贝整个 Vec

// 修改后
use std::sync::Arc;
pub type ParamBuffer = HashMap<(String, String), Arc<Vec<ParamValue>>>;
// 热路径中
let params = buffer.get(&key)?.clone(); // 仅复制 Arc 引用计数（原子操作）
apply_params_into(pm_sql, &params, colon_style, scratch); // &Arc 自动 Deref 为 &[ParamValue]
```

### criterion baseline 保存与对比
```bash
# 保存当前 v1.10 baseline（优化前）
CRITERION_HOME=benches/baselines cargo bench --bench bench_csv -- --save-baseline phase44-before

# 实施优化后对比
CRITERION_HOME=benches/baselines cargo bench --bench bench_csv -- --baseline phase44-before
```

---

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| `box dyn Exporter`（虚表分发）| `ExporterKind` 枚举（静态分发）| 已优化，热路径可内联 |
| `BufWriter` 默认容量（8KB）| 2MB 显式容量 | 减少 write() 调用次数 |
| `parse_meta` 每条记录调用 | parser 2.0.0 已物化所有字段 | 不需要再在应用层解析 |
| 每条记录 `Vec<ParamValue>` clone | **待优化** → Arc 浅拷贝 | Phase 44 目标 |

**已废弃/历史信息：**
- CLAUDE.md 记录的"16MB BufWriter"：当前代码实际为 **2MB**（`2 * 1024 * 1024`）。CLAUDE.md 描述为历史残留，应以代码为准。

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `Arc<Vec<ParamValue>>` clone 比 `Vec<ParamValue>` clone 性能显著更好 | 热点 H-3 | 若 params Vec 平均只有 2-3 个元素，Arc 原子操作开销可能抵消收益 |
| A2 | `tikv-jemalloc-ctl` 的 `stats::allocated` 反映的累计分配量足以量化 PERF-02 | Code Examples | 若评审要求"峰值 RSS"而非"总分配量"，需 valgrind/heaptrack（当前环境不可用）|
| A3 | `jemalloc` 统计接口 API 为 `epoch::mib().read()` 形式 | Code Examples | 若 tikv-jemalloc-ctl 0.6.1 API 有变化，需查阅 crate 文档调整 |
| A4 | FxHashMap 对 `(String, String)` key 有明显哈希性能提升 | Standard Stack | String key 哈希开销相对于 String clone 可能微不足道 |
| A5 | BufWriter 从 2MB 增加到 16MB 可减少系统调用 | 热点 H-4 | 若 OS 已做 write coalescing，效果可能为零 |

---

## Open Questions

1. **jemalloc 在 macOS（Apple Silicon）上的 cfg 条件**
   - What we know: `tikv-jemallocator` 支持 macOS，但部分功能可能有 feature flag 限制
   - What's unclear: 是否需要 `features = ["background_threads"]` 或其他 feature
   - Recommendation: Wave 0 尝试最小 feature（无额外 feature），编译失败再调整

2. **PERF-01 的 baseline 名称约定**
   - What we know: 现有 baselines 使用 `phase33`、`v1.0`、`baseline` 等不规则命名
   - What's unclear: Phase 44 应使用 `phase44-before` 还是 `v1.11-before`
   - Recommendation: 使用 `phase44` 命名以与 ROADMAP 阶段对应

3. **是否集成 jemalloc 到 bench_csv.rs 还是独立测试**
   - What we know: CONTEXT.md D-02 说"在测试或 benchmark 中集成"
   - What's unclear: criterion benchmark 中混入堆统计代码会影响测量精度
   - Recommendation: 独立 `#[test]` 函数（`cargo test -- --nocapture`），不集成到 criterion benchmark

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo flamegraph` | D-04 profiling | ✓ | 已安装 | samply（已安装 0.13.1）|
| `samply` | D-04 profiling | ✓ | 0.13.1 | cargo flamegraph |
| `heaptrack` | PERF-02 峰值堆 | ✗ | — | jemalloc stats（D-01）|
| `[profile.flamegraph]` | 调试符号构建 | ✓ | 已在 Cargo.toml 定义 | — |
| `sqllogs/` 真实日志目录 | 真实文件 benchmark | ✗ | — | 合成 benchmark（已有）|

**Missing dependencies with no fallback:** 无（heaptrack 不可用但 D-01 已锁定使用 jemalloc 替代）

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | criterion 0.7 + cargo test |
| Config file | `benches/BENCHMARKS.md`（使用说明）|
| Quick run command | `cargo test` |
| Full suite command | `CRITERION_HOME=benches/baselines cargo bench --bench bench_csv -- --baseline phase44` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PERF-01 | csv_export 吞吐量提升 | benchmark | `CRITERION_HOME=benches/baselines cargo bench --bench bench_csv -- --baseline phase44-before` | ✅ benches/bench_csv.rs |
| PERF-02 | 峰值堆分配减少 | unit test | `cargo test test_jemalloc_peak -- --nocapture` | ❌ Wave 0 新增 |

### Wave 0 Gaps

- [ ] `tests/jemalloc_peak.rs` 或在 `benches/bench_csv.rs` 中添加 jemalloc 统计函数 — 覆盖 PERF-02
- [ ] Cargo.toml 添加 `tikv-jemallocator` 和 `tikv-jemalloc-ctl` 到 `[dev-dependencies]`
- [ ] 保存 phase44-before baseline: `CRITERION_HOME=benches/baselines cargo bench --bench bench_csv -- --save-baseline phase44-before`

---

## Project Constraints (from CLAUDE.md)

- 函数不超过 40 行（`compute_normalized` 当前为 64 行——已超出，但本次优化不要求重构函数体积，只要不让它更长）
- `cargo clippy --all-targets -- -D warnings` 必须通过
- `cargo fmt` 必须通过
- `cargo test` 必须全部通过（D-07 对应）
- 不引入新的 `unsafe` 代码（D-06）
- BufWriter 当前为 2MB（CLAUDE.md 写的 16MB 是历史记录，应以代码为准）

---

## Sources

### Primary (HIGH confidence)
- `src/pipeline/normalizer.rs`（直接代码审查）— H-1/H-2/H-3 分配热点
- `src/exporter/csv/mod.rs`（直接代码审查）— H-4 BufWriter 容量
- `benches/baselines/csv_export/*/phase33/estimates.json`（直接读取）— v1.10 baseline 数值
- `benches/BENCHMARKS.md`（直接读取）— Phase 42 parser 基线

### Secondary (MEDIUM confidence)
- `cargo search tikv-jemallocator` / `tikv-jemalloc-ctl` / `rustc-hash` — crates.io 版本确认
- BENCHMARKS.md Phase 10 分析（samply profiling）— 热点分布历史数据

### Tertiary (LOW confidence / ASSUMED)
- `tikv-jemalloc-ctl` API 形态（基于 CONTEXT.md specifics + crate 描述推断，未实际编译验证）
- `Arc<Vec<ParamValue>>` 实际性能收益（理论分析，未实测）

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — crates.io 直接确认版本
- Architecture: HIGH — 源码直接审查，非推断
- Pitfalls: MEDIUM — 部分来自 Rust 类型系统知识（HashMap borrow 限制），部分为 ASSUMED
- Baseline values: HIGH — JSON 文件直接读取计算

**Research date:** 2026-05-24
**Valid until:** 2026-06-24（criterion/jemalloc API 稳定，30 天内有效）
