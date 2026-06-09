# Phase 73: SQLite batch INSERT - Research

**Researched:** 2026-06-08
**Domain:** Rust / rusqlite / SQLite batch INSERT / criterion benchmark
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-01:** 新增配置字段 `multi_row_batch_size: usize`，默认 **64**（rusqlite 限 999 参数，15 列 × 64 = 960 < 999；2^6 便于 benchmark 对比不同档位）

**D-02:** 与现有 `batch_size: 10_000`（事务 COMMIT 间隔）严格区分命名；`multi_row_batch_size` 控制单次 INSERT 包含的行数，两者正交

**D-03:** `multi_row_batch_size = 1` 即退化为当前单行行为，可作为 benchmark 对比基准（baseline）

**D-04:** 在 `SqliteExporter` 中维护 `row_buffer: Vec<Vec<rusqlite::types::Value>>` 行缓冲区；全量（`FieldMask::ALL`）与投影路径统一走同一 row_buffer，不再分叉

**D-05:** `flush_batch()` 方法动态构建 `INSERT INTO t VALUES (?,?,...),(?,?,...)` SQL，调用 `conn.execute()`；SQL 缓存：针对当前 buffer 大小（1..64）预缓存对应 SQL 字符串，避免热路径字符串拼接

**D-06:** `finalize()` 中调用 `flush_batch()` 处理尾部不满批的行，确保无记录丢失

**D-07:** watch 路径（`trigger_full.rs`、`trigger_incremental.rs`）通过 `Exporter` trait 使用同一 `SqliteExporter`，自动获益于 batch INSERT，无需修改

**D-08:** watch 增量触发可能只处理少量记录（< 64 行）；`finalize()` 的 flush 兜底确保增量写入完整

**D-09:** 扩展 `benches/bench_sqlite.rs`，新增 benchmark group 对比：
- `multi_row_batch_size = 1`（单行，当前行为）
- `multi_row_batch_size = 64`（推荐默认）
- 可选：16、32 中间档位

**D-10:** 使用现有 `BenchmarkId` + `Throughput::Elements` 模式（与 bench_sqlite.rs 现有风格一致）

**D-11:** SQLITE-02 验收：criterion 报告中 `multi_row_batch_size=64` vs `=1` 的 throughput 差值即为"量化提升"证据；结果追加至 BENCHMARKS.md Phase 73 段落，对比 Phase 72 baseline

### Claude's Discretion

- SQL 语句预缓存的具体实现（可用 `HashMap<usize, String>` 或 lazy_static，planner 决定）
- 是否为 `multi_row_batch_size` 添加验证（> 0 且 <= 64），与现有 `batch_size` 验证风格一致

### Deferred Ideas (OUT OF SCOPE)

- WAL 调优
- 异步迁移（Phase 76）
- 内存优化（Phase 74）
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SQLITE-01 | SQLite 导出支持 multi-row batch INSERT（缓冲 N 条记录，一次执行 `INSERT INTO t VALUES (…),(…),…`，减少逐行调用开销） | D-04 row_buffer + D-05 flush_batch() 实现路径已明确 |
| SQLITE-02 | benchmark 可以量化 multi-row INSERT 相较于当前单行模式的吞吐量提升 | D-09/D-10/D-11 bench 扩展路径已明确；v1.20 baseline 已存档 |
</phase_requirements>

---

## Summary

Phase 73 的核心改动是在 `SqliteExporter` 中引入 `row_buffer`（`Vec<Vec<rusqlite::types::Value>>`），将逐行 `INSERT` 改为批量 `INSERT INTO t VALUES (?,?,...),(?,?,...)` × N。用户决策已在 discuss-phase 完整锁定（D-01 ～ D-11），本研究验证技术可行性、确认边界条件，并为 planner 提供精确的实现参考。

**关键约束：** rusqlite 0.39.0（bundled SQLite 3.x）的 `SQLITE_LIMIT_VARIABLE_NUMBER` 默认值为 999。15 列 × 64 行 = 960 参数，安全边界充裕。`multi_row_batch_size = 1` 退化为当前单行行为，可作为 benchmark 对比组。

**已验证：** 当前 395 个 lib 测试全部通过；`benches/baselines/` 中 v1.20 baseline 已完整存档（含 `sqlite_export` 和 `sqlite_single_row` group），Phase 73 执行后直接用 `--baseline v1.20` 对比即可量化提升（SQLITE-02）。

**Primary recommendation:** 按 D-04/D-05/D-06 在 `exporter.rs` + `impls.rs` + `sql_builder.rs` 三处集中修改，`write.rs` 中提取 `sqllog_to_values()` 工具函数统一行数据序列化，`flush_batch()` 通过 `params_from_iter` 传入 flattened `Vec<&Value>` 执行单条 multi-row INSERT。

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| 行缓冲区维护 | `SqliteExporter` struct | — | 缓冲是 exporter 内部状态，不穿透 Exporter trait |
| flush_batch() SQL 构建 | `sql_builder.rs` | `exporter.rs` | SQL 生成逻辑已集中在 sql_builder，保持一致性 |
| flush_batch() 执行 | `exporter.rs` 或 `impls.rs` | — | 需要访问 `self.conn`，属于 exporter 层 |
| 行数据序列化（Sqllog → Vec<Value>） | `write.rs` | — | 现有 `do_insert_preparsed` 已含此逻辑，提取为可复用函数 |
| config 字段 | `config/exporter.rs` | — | 现有 `SqliteExporterConfig` 扩展 |
| benchmark 对比 | `benches/bench_sqlite.rs` | — | 现有 bench 文件扩展新 group |
| watch 路径 | 无需修改（透明获益） | — | 通过 Exporter trait 自动使用新 SqliteExporter |

---

## Standard Stack

本 phase 不引入新 crate 依赖，全部使用现有库。

### Core（已锁定）

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| rusqlite | 0.39.0 [VERIFIED: crates.io registry] | SQLite 连接、prepared statement、params_from_iter | 项目现有依赖，bundled SQLite |
| criterion | 0.7 [VERIFIED: crates.io registry] | benchmark 框架，`BenchmarkId` + `Throughput::Elements` | 项目现有 bench 基础设施 |

> 注：crates.io 最新版 rusqlite=0.40.1、criterion=0.8.2，但项目 Cargo.toml 锁定 0.39.0 / 0.7，本 phase 不升级版本（非需求范围）。

### 无新依赖安装

本 phase 不新增任何 Cargo.toml 依赖项。

---

## Package Legitimacy Audit

本 phase 不安装任何外部包，跳过此节。

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Architecture Patterns

### System Architecture Diagram

```
Sqllog records (from parser)
        |
        v
export_one_preparsed()        ← 入口（Exporter trait）
        |
        v
sqllog_to_values()            ← 序列化为 Vec<Value>（全量/投影均走此路径）
        |
        v
row_buffer.push(row_values)   ← 追加到缓冲区
        |
        v
[buffer.len() >= multi_row_batch_size?]
        |YES                            |NO
        v                               v
flush_batch()              (继续接收下一条记录)
  构建 multi-row INSERT SQL
  params_from_iter(flattened values)
  conn.execute(sql, params)
  row_buffer.clear()
        |
        v
batch_commit_if_needed()      ← row_count 递增，事务 COMMIT 间隔（不变）
        |
        v
finalize()
  flush_batch()               ← 处理尾部不满批
  COMMIT
```

### Recommended Project Structure

变更仅涉及 exporter/sqlite 子模块，不新增目录：

```
src/
├── config/
│   └── exporter.rs          # 新增 multi_row_batch_size 字段 + 验证
└── exporter/
    └── sqlite/
        ├── exporter.rs      # 新增 row_buffer、multi_row_batch_size 字段；flush_batch() 方法
        ├── impls.rs         # 重构 export_one_preparsed：改为 push to buffer
        ├── sql_builder.rs   # 新增 build_multi_row_insert_sql(table, col_count, row_count)
        └── write.rs         # 新增 sqllog_to_values() 提取函数

benches/
└── bench_sqlite.rs          # 新增 bench_sqlite_multi_row_insert group
```

### Pattern 1: rusqlite params_from_iter 用于动态参数数量

**What:** `params_from_iter` 接受 `IntoIterator<Item: ToSql>`，适合在运行时确定参数数量的场景（如 multi-row INSERT 的 flattened values）。

**When to use:** SQL 语句中占位符数量在编译期未知（随 buffer 大小变化）时。

**Example:**

```rust
// Source: rusqlite docs.rs/0.39.0 + 项目现有 write.rs:67
use rusqlite::params_from_iter;
use rusqlite::types::Value;

// buffer: Vec<Vec<Value>>，每行 15 个字段
let flattened: Vec<&Value> = buffer.iter().flat_map(|row| row.iter()).collect();
conn.execute(&multi_row_sql, params_from_iter(flattened.iter().map(|v| v as &dyn rusqlite::types::ToSql)))?;
```

**注意：** `Value` 实现了 `ToSql`，但 `&Value` 也实现了 `ToSql`（通过 `impl<T: ToSql> ToSql for &T`）。flattened 引用切片可直接传入 `params_from_iter`。[VERIFIED: rusqlite 0.39.0 源码]

### Pattern 2: SQL 预缓存（HashMap<usize, String>）

**What:** 按行数预先生成并缓存多行 INSERT SQL 字符串，避免热路径每次重新拼接。

**When to use:** 当 `multi_row_batch_size = 64` 时，flush 始终生成 64-row SQL；仅最后一批（尾部）可能为 1..63 行——两种情况均可命中缓存。

**Example:**

```rust
// Source: 项目 sql_builder.rs 扩展
use std::collections::HashMap;

fn build_multi_row_insert_sql(table_name: &str, col_count: usize, row_count: usize) -> String {
    let one_row = format!("({})", vec!["?"; col_count].join(", "));
    let rows = vec![one_row.as_str(); row_count].join(", ");
    format!("INSERT INTO \"{table_name}\" VALUES {rows}")
}

// 在 SqliteExporter 中预缓存（flush_batch 调用前初始化）：
let mut sql_cache: HashMap<usize, String> = HashMap::new();
for n in 1..=multi_row_batch_size {
    sql_cache.insert(n, build_multi_row_insert_sql(&table_name, col_count, n));
}
```

### Pattern 3: batch_commit_if_needed 与 row_buffer 的正交关系

**What:** `batch_commit_if_needed()` 通过 `row_count` 控制 COMMIT 间隔；`row_buffer` 控制单次 INSERT 包含的行数。两者完全正交——每次 `flush_batch()` 后仍需对 buffer 中每行递增 `row_count` 并检查 COMMIT 间隔。

**When to use:** 始终。flush 后必须对 flushed 的每行执行 `batch_commit_if_needed`，否则事务 COMMIT 计数失效。

**实现方案：** flush_batch 执行 execute 后，按实际写入行数（`buffer.len()`）循环调用 `batch_commit_if_needed`，或直接在 flush_batch 内部对 flushed_count 递增 row_count 并批量检查。

### Anti-Patterns to Avoid

- **不要在 flush_batch 中使用 `prepare_cached`：** multi-row SQL 字符串随 batch size 变化，`prepare_cached` 基于 SQL 字符串的 LRU（容量 16）会在尾部 flush 时引入额外 prepare 开销。直接使用 `conn.execute()` 即可（statement cache 不适合动态 SQL）。[ASSUMED: 基于 rusqlite prepare_cached LRU 行为推断]
- **不要在热路径中每次构建 SQL 字符串：** 对所有可能的 batch size（1..=64）预缓存 SQL，避免 `flush_batch` 中字符串拼接（D-05 明确要求）。
- **不要将 row_buffer flush 与事务 COMMIT 耦合：** `flush_batch()` 只负责写 INSERT，`batch_commit_if_needed` 独立控制 COMMIT 节奏；两者耦合会破坏 COMMIT 计数语义。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 动态参数绑定 | 手动构建参数数组 | `rusqlite::params_from_iter` | 项目已使用（write.rs:67），类型安全，避免 unsafe |
| SQLite 参数数量检查 | 运行时 assert | 编译期常量（15 × 64 = 960 < 999）+ 配置验证（<= 64） | rusqlite 会在超限时返回 `SQLITE_RANGE` 错误，但预防性验证更早发现问题 |
| benchmark 吞吐量计算 | 手动计时 | `criterion::Throughput::Elements` | 项目已使用，自动输出 records/sec |

**Key insight:** multi-row INSERT 的核心复杂度在于 SQL 构建和参数 flattening，rusqlite 的 `params_from_iter` 完全覆盖后者；前者通过简单字符串拼接+缓存即可实现，无需额外依赖。

---

## Common Pitfalls

### Pitfall 1: 尾部 flush 遗漏

**What goes wrong:** 循环结束时 `row_buffer` 中仍有少于 `multi_row_batch_size` 的剩余记录，`finalize()` 未调用 `flush_batch()` 导致记录丢失。

**Why it happens:** flush 触发条件是 `buffer.len() >= multi_row_batch_size`，最后一批小于阈值时不触发。

**How to avoid:** `finalize()` 首先调用 `flush_batch()`（D-06 明确要求），在 COMMIT 前写入剩余记录。

**Warning signs:** 集成测试中输入 N 条记录（N 不整除 batch_size）但 DB 中 COUNT(*) < N。

### Pitfall 2: batch_commit_if_needed 计数失配

**What goes wrong:** `flush_batch()` 一次写入 M 行，但 `batch_commit_if_needed` 只被调用 1 次（仅递增 1），导致 `row_count` 失真，COMMIT 间隔失控。

**Why it happens:** 原单行路径每行调用一次；batch 路径写入 M 行后应循环递增 M 次。

**How to avoid:** flush 后对 flushed 的每行都执行 `row_count += 1` 并检查 COMMIT 条件，或在 flush 内部批量处理（flushed_count 次循环）。

**Warning signs:** `test_sqlite_batch_commit` 类测试在大 batch 下失败，或 DB 文件大小异常（事务未提交）。

### Pitfall 3: SQLITE_LIMIT_VARIABLE_NUMBER 超限

**What goes wrong:** 若允许 `multi_row_batch_size` 大于 66（15 × 67 = 1005 > 999），rusqlite 执行时返回 `SQLITE_RANGE` 错误。

**Why it happens:** SQLite 默认限制每条 SQL 语句最多 999 个绑定参数（SQLITE_MAX_VARIABLE_NUMBER）。[CITED: https://www.sqlite.org/limits.html]

**How to avoid:** config 验证中限制 `multi_row_batch_size <= 64`（15 × 64 = 960 < 999，D-01）。

**Warning signs:** 测试中 `execute()` 返回 `SqliteFailure` 含 "too many SQL variables"。

### Pitfall 4: 全量/投影路径分叉导致 buffer 路径不一致

**What goes wrong:** 仅在全量路径（`FieldMask::ALL`）应用 buffer，投影路径仍走单行 INSERT，结果不一致且 SQLITE-01 未完整实现。

**Why it happens:** 原 `do_insert_preparsed` 有全量快速路径和投影分支，如果新 buffer 逻辑只覆盖一条分支。

**How to avoid:** D-04 明确要求全量与投影路径统一走同一 `row_buffer`——提取 `sqllog_to_values()` 函数，两条路径均生成 `Vec<Value>` 后推入 buffer，消除分叉。

---

## Code Examples

### 多行 INSERT SQL 构建

```rust
// Source: 基于 src/exporter/sqlite/sql_builder.rs 扩展模式 [ASSUMED]
pub(super) fn build_multi_row_insert_sql(
    table_name: &str,
    col_count: usize,
    row_count: usize,
) -> String {
    debug_assert!(row_count > 0, "row_count must be > 0");
    let one_row = format!("({})", vec!["?"; col_count].join(", "));
    let value_rows = std::iter::repeat(one_row.as_str())
        .take(row_count)
        .collect::<Vec<_>>()
        .join(", ");
    format!("INSERT INTO \"{table_name}\" VALUES {value_rows}")
}
```

### flush_batch 核心逻辑（伪代码）

```rust
// Source: 基于 CONTEXT.md D-05 设计 + rusqlite params_from_iter 用法 [ASSUMED]
fn flush_batch(&mut self) -> Result<()> {
    if self.row_buffer.is_empty() {
        return Ok(());
    }
    let row_count = self.row_buffer.len();
    let col_count = self.ordered_indices.len();
    let sql = self
        .sql_cache
        .entry(row_count)
        .or_insert_with(|| build_multi_row_insert_sql(&self.table_name, col_count, row_count));
    let conn = self.conn_ref()?;
    let flattened: Vec<rusqlite::types::Value> = self
        .row_buffer
        .drain(..)
        .flatten()
        .collect();
    conn.execute(sql, rusqlite::params_from_iter(flattened.iter()))
        .map_err(|e| Self::db_err(format!("batch insert failed: {e}")))?;
    Ok(())
}
```

> **注意：** `self.sql_cache` 是 `HashMap<usize, String>`，在 `SqliteExporter::new()` 或 `initialize()` 中预填充所有 `1..=multi_row_batch_size` 条目（D-05 要求避免热路径拼接）。`row_buffer.drain(..)` 在 flush 后自动清空 buffer，避免额外 `clear()` 调用。

### sqllog_to_values 提取函数

```rust
// Source: 重构自 src/exporter/sqlite/write.rs do_insert_preparsed [ASSUMED]
pub(super) fn sqllog_to_values(
    sqllog: &Sqllog,
    normalized_sql: Option<&str>,
    field_mask: FieldMask,
    ordered_indices: &[usize],
) -> Vec<rusqlite::types::Value> {
    use rusqlite::types::Value;
    // 复用现有 do_insert_preparsed 中 let all: [Value; 15] = [...] 的构建逻辑
    // 返回 ordered_indices 投影后的 Vec<Value>
    // 全量路径（ordered_indices.len() == 15）直接返回全部 15 个字段
}
```

### Benchmark 新增 group（multi_row 对比）

```rust
// Source: 基于 benches/bench_sqlite.rs 现有模式扩展 [ASSUMED]
fn bench_sqlite_multi_row_insert(c: &mut Criterion) {
    let bench_dir = bench_common::bench_target_dir("bench_sqlite_multi_row");
    let sqllog_dir = bench_dir.join("sqllogs");
    fs::create_dir_all(&sqllog_dir).unwrap();

    let mut group = c.benchmark_group("sqlite_multi_row");
    group.sample_size(20);

    for &n in &[10_000usize, 50_000] {
        fs::write(sqllog_dir.join("bench.log"), bench_common::synthetic_log(n)).unwrap();
        for &batch_size in &[1usize, 16, 32, 64] {
            let cfg = make_config(&sqllog_dir, &bench_dir, 10_000, batch_size); // 新增 multi_row_batch_size 参数
            group.throughput(Throughput::Elements(n as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("n={n}"), format!("multi_row={batch_size}")),
                &cfg,
                |b, cfg| {
                    b.iter(|| {
                        handle_run(cfg, true, false, &Arc::new(AtomicBool::new(false)), None).unwrap();
                    });
                },
            );
        }
    }
    group.finish();
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| 逐行 INSERT（Phase 73 之前） | multi-row batch INSERT（Phase 73） | Phase 73 | 减少 SQLite VDBE 调用次数，降低 per-INSERT 固定开销 |
| 单一 `batch_size` 字段 | `batch_size`（事务间隔） + `multi_row_batch_size`（INSERT 行数） | Phase 73 | 两个正交维度分开调优 |

**Deprecated/outdated:**
- `do_insert_preparsed` 直接执行单行 INSERT 的热路径：被 `row_buffer.push` + `flush_batch` 替代（保留函数签名或重构为 `sqllog_to_values` 工具函数）

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `prepare_cached` 不适合动态 multi-row SQL（LRU 缓存 key 是 SQL 字符串，动态 SQL 会导致缓存抖动） | Architecture Patterns / Anti-Patterns | 风险低：如果 LRU 容量足够大且 batch size 固定，prepare_cached 也可工作；但 D-05 已明确用 HashMap 缓存 SQL 字符串 + `conn.execute()`，不依赖 prepare_cached |
| A2 | `flush_batch` 的具体签名和 SQL cache 初始化时机（new() vs initialize()） | Code Examples | 风险低：两种时机均可行，planner 根据 conn 可用性决定（sql_cache 不依赖 conn，可在 new() 中初始化） |
| A3 | `row_buffer.drain(..)` 在 flush 后清空 buffer（无需额外 clear()） | Code Examples | 风险低：Rust `Vec::drain` 行为确定，消费后 buffer 为空 |

**如此表为空：** 所有关键技术决策均来自 CONTEXT.md 锁定决策或代码库直接验证，假设仅限实现细节。

---

## Open Questions (RESOLVED)

1. **flush 后 batch_commit_if_needed 的调用方式**
   - What we know: 原路径每行调用一次；flush 后需对 M 行均递增计数
   - What's unclear: 是在 flush_batch 内部循环 M 次 `row_count += 1; if row_count % batch_size == 0 { COMMIT }`，还是在 impls.rs 调用 flush 后再循环
   - Recommendation: 在 `flush_batch()` 内部处理，返回 `flushed_count` 并在 exporter 层统一更新 `row_count`，保持 flush_batch 职责单一
   - **RESOLVED:** 在 `impls.rs` 中调用 `flush_batch()` 后外部循环 `for _ in 0..flushed { self.batch_commit_if_needed()?; }`，保持 `flush_batch` 职责单一（仅执行 INSERT，不管 COMMIT 节奏），与 Plan 01 Task 2 action 一致。

2. **sql_cache 初始化时机：new() vs initialize()**
   - What we know: `sql_cache` 构建只需要 `table_name`、`col_count`（`ordered_indices.len()`）和 `multi_row_batch_size`，不依赖 `conn`
   - What's unclear: `ordered_indices` 在 `from_config()` 后、`initialize()` 前可能被外部修改（见 tests.rs 中直接赋值 `exporter.ordered_indices = vec![...]`）
   - Recommendation: 在 `initialize()` 中初始化 sql_cache（此时 ordered_indices 已最终确定），与 `insert_sql = build_insert_sql(...)` 同步构建
   - **RESOLVED:** 在 `initialize()` 中预填充 sql_cache（`for n in 1..=self.multi_row_batch_size { self.sql_cache.insert(n, ...) }`），与 Plan 01 Task 2 action 一致；此时 `ordered_indices` 已最终确定，避免 `new()`/`from_config()` 阶段外部赋值导致 cache 与实际列数不符。

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust/Cargo | 编译、测试、benchmark | ✓ | 1.94.0 | — |
| rusqlite 0.39.0 (bundled) | SQLite 导出 | ✓ | 0.39.0 | — |
| criterion 0.7 | benchmark | ✓ | 0.7 | — |
| v1.20 baseline（benches/baselines/） | SQLITE-02 对比 | ✓ | sqlite_export + sqlite_single_row 均已存档 | — |

**Missing dependencies with no fallback:** 无

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test（`cargo test`）|
| Config file | Cargo.toml（`[dev-dependencies]`）|
| Quick run command | `cargo test --lib -p dm-database-sqllog2db sqlite 2>&1` |
| Full suite command | `cargo test --lib` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SQLITE-01 | multi-row batch INSERT 写入正确（N 条记录，DB COUNT(*) == N） | integration | `cargo test --lib test_sqlite_multi_row_basic` | ❌ Wave 0 |
| SQLITE-01 | 全量路径与投影路径结果等价（与单行模式逐字段对比） | integration | `cargo test --lib test_sqlite_multi_row_field_equality` | ❌ Wave 0 |
| SQLITE-01 | 空输入（0 条记录）不报错，DB 为空表 | integration | `cargo test --lib test_sqlite_multi_row_empty_input` | ❌ Wave 0 |
| SQLITE-01 | 尾部不满批（N 不整除 batch_size）记录无丢失 | integration | `cargo test --lib test_sqlite_multi_row_partial_tail` | ❌ Wave 0 |
| SQLITE-01 | multi_row_batch_size=1 等价于当前单行行为（COUNT 相等，字段相等） | integration | `cargo test --lib test_sqlite_multi_row_batch1_equals_single` | ❌ Wave 0 |
| SQLITE-02 | benchmark 输出含 multi_row vs single_row throughput 对比 | benchmark | `cargo bench --bench bench_sqlite sqlite_multi_row` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test --lib 2>&1 | tail -3`
- **Per wave merge:** `cargo test --lib && cargo clippy --all-targets -- -D warnings`
- **Phase gate:** `cargo test --lib && cargo clippy --all-targets -- -D warnings`，benchmark 输出记录至 BENCHMARKS.md

### Wave 0 Gaps

- [ ] `src/exporter/sqlite/tests.rs` 中新增 5 个 multi-row 正确性测试（覆盖 SQLITE-01 全部边界）
- [ ] `benches/bench_sqlite.rs` 中新增 `bench_sqlite_multi_row_insert` group + 更新 `make_config` 签名
- [ ] `src/config/exporter.rs` 中 `SqliteExporterConfig` 新增 `multi_row_batch_size` 字段后，`test_sqlite_from_config` 需更新

*(现有 395 个 lib 测试在修改前已全部通过，Wave 0 只需新增测试，不修改已有测试逻辑)*

---

## Security Domain

本 phase 无网络接口、无用户输入直接进入 SQL（参数化绑定），无认证/会话/加密需求。适用的 ASVS 类别：

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V5 Input Validation | 边际适用 | `multi_row_batch_size` config 验证（> 0 且 <= 64），与现有 `batch_size` 验证风格一致 |
| V6 Cryptography | 否 | — |
| SQL Injection | 不适用 | INSERT SQL 使用 rusqlite 参数化绑定（`params_from_iter`），无字符串拼接进用户数据 |

---

## Sources

### Primary (HIGH confidence)

- 项目源码（直接读取验证）：`src/exporter/sqlite/exporter.rs`, `impls.rs`, `write.rs`, `sql_builder.rs`, `tests.rs`, `src/config/exporter.rs`
- `benches/bench_sqlite.rs` — 现有 benchmark 风格（`BenchmarkId`, `Throughput::Elements`, `make_config`）
- `benches/BENCHMARKS.md` — Phase 5/72 数据，v1.20 baseline 存档确认
- `Cargo.toml` / `Cargo.lock` — rusqlite 0.39.0, criterion 0.7 版本确认
- `.planning/phases/73-sqlite-batch-insert/73-CONTEXT.md` — 用户锁定决策 D-01～D-11

### Secondary (MEDIUM confidence)

- [rusqlite 0.39.0 docs.rs](https://docs.rs/rusqlite/0.39.0/rusqlite/) — `params_from_iter` 函数签名与用法验证
- [SQLite limits documentation](https://www.sqlite.org/limits.html) — `SQLITE_MAX_VARIABLE_NUMBER` = 999（默认值）

### Tertiary (LOW confidence)

- 无

---

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH — Cargo.toml/Cargo.lock 直接验证，无新依赖
- Architecture: HIGH — 基于现有代码直接分析，决策已在 CONTEXT.md 锁定
- Pitfalls: HIGH — 来自代码逻辑推断（尾部 flush、COMMIT 计数、参数超限均有代码证据）
- Benchmark: HIGH — bench_sqlite.rs 现有模式完整，v1.20 baseline 已确认存档

**Research date:** 2026-06-08
**Valid until:** 2026-07-08（rusqlite/criterion 版本稳定，30 天有效期）
