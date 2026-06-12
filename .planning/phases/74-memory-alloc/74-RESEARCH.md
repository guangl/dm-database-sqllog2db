# Phase 74: 内存与分配优化 - Research

**Researched:** 2026-06-09
**Domain:** Rust 堆分配优化 — HashMap 结构重构 + Vec 预热容量
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**MEM-01: HashMap key 消除策略**
- D-01: 将 `ParamBuffer` 改为二级结构：`HashMap<String, HashMap<String, Arc<Vec<ParamValue>>>>`
  - 查询路径：`buffer.get(record.sess_id.as_str())?.get(record.statement.as_str())?.clone()` — 零分配
  - insert 路径：`buffer.entry(record.sess_id.clone()).or_default().insert(record.statement.clone(), Arc::new(params))`
- D-02: 不使用自定义 `Borrow` impl；不使用 `Arc<str>` key
- D-03: 改动目标：`normalizer.rs:386` 的 `let key = (record.sess_id.clone(), record.statement.clone())` + `buffer.get(&key)` 模式；`normalizer.rs:367-369` 的 PARAMS insert 路径随之改用 `entry` API

**MEM-02: line_buf 初始容量预热**
- D-04: 将 `CsvExporter::new()` 中的 `line_buf: Vec::with_capacity(2048)` 改为 `Vec::with_capacity(4096)`
  - 注释：`// 典型 DaMeng SQL + 字段开销约 1–4KB；writer.rs 的动态 reserve 兜底更长 SQL`
- D-05: 不修改 `writer.rs` 的动态 reserve 逻辑（保留不变）

**测试要求**
- D-06: 两项优化均须有对应测试确保行为不变（MEM-01 扩展 normalizer tests，MEM-02 依赖现有集成测试）
- D-07: `cargo clippy --all-targets -- -D warnings` + `cargo test` 全部通过，无新增 unsafe

### Claude's Discretion
- PARAMS insert 路径的 `entry` API 是否先 `contains_key` 检查（性能与可读性权衡）
- 单元测试是否额外覆盖二级 HashMap 的空 inner map 场景（`sess_id` 存在但 `statement` 不存在）

### Deferred Ideas (OUT OF SCOPE)
- heaptrack/massif 峰值内存 profiling（PROF-02）
- flamegraph CPU 热点分析（PROF-01）
- normalizer PARAMS 记录 insert 路径的进一步优化（intern pool）
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| MEM-01 | normalizer 热路径 HashMap key 不再每条记录重复 clone String | 二级 HashMap 查询路径零分配，insert 路径 clone 仅发生在低频 PARAMS 记录 |
| MEM-02 | CSV line_buf 初始容量按典型 SQL 长度预热，减少 Vec grow 次数 | 4096 字节覆盖典型 DaMeng SQL 范围，writer.rs 动态 reserve 兜底超长 SQL |
</phase_requirements>

---

## Summary

本 phase 执行两项精准的低风险内存优化，无需新增外部依赖，改动范围极小。

**MEM-01** 消除 `compute_normalized` 热路径中每条执行记录的 2 次 `String::clone`（`sess_id` + `statement`）。当前实现以 `(String, String)` 元组为 key，每次 `buffer.get(&key)` 都必须先构造 key（即 clone 两个字符串）。改为二级 HashMap 后，查询路径可直接用 `record.sess_id.as_str()` 和 `record.statement.as_str()` 作为 `&str` 参数——`HashMap` 实现了 `Borrow<str>` 所以 `.get(&str)` 无需分配。PARAMS insert 路径（低频）仍需 clone 以拥有 key，改用 `entry().or_default().insert()` 模式，语义清晰无额外开销。

**MEM-02** 将 `line_buf` 初始容量从 2048 提升到 4096 字节，使典型 DaMeng SQL 语句（含 INSERT/UPDATE + WHERE 子句 + 各字段值）在绝大多数情况下不触发 `Vec::grow`。动态 `reserve` 逻辑（`writer.rs:202-205`）保持不变，正确处理超长 SQL。

两处改动均有测试覆盖，与既有测试基础完全兼容，`cargo bench --bench bench_csv` 可量化验证无吞吐量退化。

**Primary recommendation:** 严格按照 CONTEXT.md 决策执行，改动量极小（各约 5-10 行），全部在现有测试框架内验证。

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| PARAMS 记录缓存（MEM-01） | Pipeline（normalizer.rs） | CLI run 层（processor.rs 调用方） | buffer 在 normalizer 中定义和维护，processor 只是传递引用 |
| CSV 行序列化（MEM-02） | Exporter（csv/exporter.rs, csv/writer.rs） | — | line_buf 在 CsvExporter 内部，writer.rs 负责动态 reserve |
| 类型定义变更传播 | normalizer.rs（ParamBuffer 定义） | processor.rs、collector.rs、sequential.rs、tests.rs（使用方） | ParamBuffer 类型别名变更后，所有 use 点自动适应 |

---

## Standard Stack

### Core（项目已有，无需新增）

| Library | Version | Purpose | 说明 |
|---------|---------|---------|------|
| `std::collections::HashMap` | Rust std | 二级 HashMap 结构 | [VERIFIED: Rust std] 原生支持 `.get(&str)` via `Borrow<str>` |
| `std::sync::Arc` | Rust std | `Arc<Vec<ParamValue>>` 热路径 clone O(1) | [VERIFIED: Rust std] 已在 normalizer.rs 使用 |

### 不需要安装任何新包

本 phase 零新依赖：所有优化使用标准库已有能力。[VERIFIED: CONTEXT.md D-07]

---

## Package Legitimacy Audit

> 本 phase 不安装任何外部包。

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Architecture Patterns

### System Architecture Diagram（改动范围）

```
[Sqllog record]
     │
     ▼
compute_normalized()          ← normalizer.rs:355
     │
     ├─── PARAMS record? ──► buffer.entry(sess_id).or_default()
     │                            .insert(statement, Arc::new(params))
     │                            (低频 insert，clone 不可避免)
     │
     └─── DML record? ─────► buffer
                               .get(sess_id.as_str())  ← 零分配 &str 查询
                               ?.get(statement.as_str()) ← 零分配 &str 查询
                               ?.clone()               ← Arc clone，O(1)
                                    │
                                    ▼
                             apply_params_into(sql, &params, ...)
                                    │
                                    ▼
                             CsvExporter::export_one_preparsed()
                                    │
                             line_buf (capacity: 4096)  ← MEM-02
                                    │
                             write_record_preparsed()
                                    │
                             动态 reserve（writer.rs:202-205，保留不变）
                                    │
                             BufWriter<File>.write_all()
```

### 改动文件清单

```
src/
├── pipeline/
│   └── normalizer.rs        # MEM-01: ParamBuffer 类型 + compute_normalized 改动
└── exporter/
    └── csv/
        └── exporter.rs      # MEM-02: line_buf 容量 2048 → 4096 + 注释
```

### Pattern 1: 二级 HashMap 零分配查询

**What:** `HashMap<String, HashMap<String, V>>` 利用 `Borrow<str>` 允许 `&str` 查询，避免构造 key 时的 String clone。

**When to use:** key 由多个 String 字段组合而成，且查询频率远高于 insert 频率（此场景：每条执行记录查询 1 次，每个 PARAMS 记录 insert 1 次）。

**Example:**
```rust
// Source: Rust std HashMap docs — Borrow<str> for &String
// 旧写法：每次查询 clone 两个 String
let key = (record.sess_id.clone(), record.statement.clone());
let params = buffer.get(&key)?.clone();

// 新写法：零分配查询（&str 满足 Borrow<str>）
let params = buffer
    .get(record.sess_id.as_str())?
    .get(record.statement.as_str())?
    .clone();
```

**为何可行：** `HashMap<String, V>` 实现了 `impl<K: Borrow<Q>> get(&Q)`，所以 `&str` 可直接作为查询 key，无需创建 `String`。[VERIFIED: Rust std]

### Pattern 2: entry().or_default() insert 路径

**What:** 对二级 HashMap 的 insert 使用 `entry` API，避免双次查询。

**Example:**
```rust
// Source: Rust std HashMap entry API
buffer
    .entry(record.sess_id.clone())
    .or_default()
    .insert(record.statement.clone(), Arc::new(params));
```

**注意：** PARAMS insert 路径的 `clone()` 不可避免（需要拥有 key），但此路径频率远低于执行记录查询路径，在整体性能中可忽略。[ASSUMED: 基于 DaMeng 日志典型结构——每个 PARAMS 记录对应多条 DML 执行记录]

### Pattern 3: Vec 预热容量

**What:** 对于大小可预估的序列化缓冲区，在构造时直接设置合理初始容量，避免热路径 grow。

**Example:**
```rust
// 典型 DaMeng SQL + 字段开销约 1–4KB；writer.rs 的动态 reserve 兜底更长 SQL
line_buf: Vec::with_capacity(4096),
```

**为何选 4096：** 典型 DaMeng SQL（INSERT/UPDATE/SELECT + 带 WHERE 条件 + 多个字段值）约 500–3000 字节，加上 CSV 其他字段（ts、sess_id、username 等固定开销约 128 字节）总长多数不超过 4096。`writer.rs:202-205` 的动态 reserve 正确处理超长 SQL。[CITED: CONTEXT.md D-04]

### Anti-Patterns to Avoid

- **为每次查询构造 String key：** 无论是 clone 还是 `format!`，都会触发堆分配。改用 `&str` 查询。
- **在查询路径中 `Arc::from(&str)`：** 即使 Arc 本身是共享的，`Arc::from(&str)` 在每次调用时仍分配一个新 Arc（包含 str 数据的拷贝）。这是 CONTEXT.md D-02 明确排除 `Arc<str>` key 方案的原因。
- **移除 writer.rs 的动态 reserve：** 固定容量不能覆盖所有场景，必须保留动态 reserve 作为兜底。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 零分配 HashMap 查询 | 自定义 Borrow impl 或 interning pool | `HashMap<String, HashMap<String, V>>` + `&str` 查询 | 标准库已原生支持，自定义实现增加复杂度 [VERIFIED: Rust std] |
| Arc 引用计数 clone | 手动 refcount 或 unsafe 裸指针 | `Arc<Vec<ParamValue>>` | 已在代码中使用，热路径 clone 是 O(1) 原子操作 |

---

## Common Pitfalls

### Pitfall 1: 调用方 ParamBuffer 类型别名传播

**What goes wrong:** `ParamBuffer` 是 `pub type` 别名，定义在 `normalizer.rs:12`。改为二级结构后，所有使用 `ParamBuffer` 的地方（`processor.rs:6`、`collector.rs:6`、`sequential.rs:72`、`tests.rs:283`）自动适应新类型——但若任何地方有 `ParamBuffer::new()` 且期望构造元组 key，需同步更新。

**Why it happens:** `tests.rs:311` 有 `ParamBuffer::new()`，类型别名变更后此处无需修改（`HashMap::new()` 对二级结构同样成立），但使用 `ParamBuffer::insert((key_tuple, ...))` 的用法必须改成 `.entry().or_default().insert()`。

**How to avoid:** 修改 `ParamBuffer` 定义后立即 `cargo build`，让编译器在所有调用点报错指引修改。

**Warning signs:** `E0308 expected tuple (String, String), found String` 编译错误。

### Pitfall 2: entry().or_default() 的 clippy lint

**What goes wrong:** Rust clippy 有 `clippy::map_entry` lint，若使用 `if !contains_key { insert }` 模式而非 `entry` 会报警。

**How to avoid:** 直接使用 `entry().or_default().insert()`，clippy 认可此模式，无警告。

**Note:** CONTEXT.md 的 "Claude's Discretion" 提到是否先 `contains_key` 检查。建议直接用 `entry` 无需 `contains_key`——对于 PARAMS 记录 insert，覆盖旧值（后来的 PARAMS 覆盖前面的）是正确行为（同一 sess+stmt 的最新 PARAMS 有效）。

### Pitfall 3: 二级 HashMap clear() 语义变化

**What goes wrong:** `processor.rs:201` 有 `params_buffer.clear()`，在文件边界重置 buffer。改为二级结构后，`clear()` 清除外层 HashMap（同时 drop 所有 inner HashMap），语义与原来一致——无需修改。

**How to avoid:** 验证 `process_log_file` 中的 `params_buffer.clear()` 调用（`processor.rs:201`）在二级结构下行为正确（它确实正确，outer HashMap clear 会 drop 所有 inner entries）。

### Pitfall 4: 测试中直接构造 ParamBuffer

**What goes wrong:** `tests.rs:311` 有：
```rust
let mut params_buffer: ParamBuffer = ParamBuffer::new();
```
类型别名变更后此行继续编译（`HashMap::new()` 适用于二级结构），但后续的 `params_buffer.insert(...)` 若使用了元组 key 则需更新。

**How to avoid:** 检查 `tests/` 和 `src/cli/run/tests.rs` 中所有手动构造 `ParamBuffer` 并 insert 的代码，统一改成二级 HashMap API。

---

## Code Examples

### 完整 MEM-01 改动（normalizer.rs）

```rust
// Source: normalizer.rs — 当前实现（行 12, 366-369, 386-388）

// 【旧】类型定义
pub type ParamBuffer = HashMap<(String, String), Arc<Vec<ParamValue>>>;

// 【新】类型定义
pub type ParamBuffer = HashMap<String, HashMap<String, Arc<Vec<ParamValue>>>>;

// 【旧】PARAMS insert（normalizer.rs:366-369）
buffer.insert(
    (record.sess_id.clone(), record.statement.clone()),
    Arc::new(params),
);

// 【新】PARAMS insert
buffer
    .entry(record.sess_id.clone())
    .or_default()
    .insert(record.statement.clone(), Arc::new(params));

// 【旧】DML 查询（normalizer.rs:386-388）
let key = (record.sess_id.clone(), record.statement.clone());
let params = buffer.get(&key)?.clone();

// 【新】DML 查询（零分配）
let params = buffer
    .get(record.sess_id.as_str())?
    .get(record.statement.as_str())?
    .clone();
```

### 完整 MEM-02 改动（exporter.rs）

```rust
// Source: src/exporter/csv/exporter.rs — CsvExporter::new()（行 46）

// 【旧】
line_buf: Vec::with_capacity(2048),

// 【新】
// 典型 DaMeng SQL + 字段开销约 1–4KB；writer.rs 的动态 reserve 兜底更长 SQL
line_buf: Vec::with_capacity(4096),
```

---

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| `HashMap<(String, String), V>` — 查询 clone 2 次 | `HashMap<String, HashMap<String, V>>` — 查询零分配 | 热路径每条执行记录节省 2 次 String heap 分配 |
| `Vec::with_capacity(2048)` | `Vec::with_capacity(4096)` | 减少典型 SQL 大小的 Vec grow 次数 |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | PARAMS 记录频率远低于 DML 执行记录（insert 路径低频可接受 clone） | Standard Stack / Pattern 1 | 若 PARAMS 与 DML 1:1，则 insert clone 频率与查询 clone 相当，优化效果减半——但功能正确性不受影响 |

---

## Open Questions

1. **二级 HashMap inner map 空 session 场景测试**
   - What we know: CONTEXT.md 的 "Claude's Discretion" 提到是否额外覆盖 `sess_id` 存在但 `statement` 不存在的场景
   - What's unclear: 现有测试 `normalizer.rs::tests` 中 `compute_normalized` 相关测试是否已覆盖此边界
   - Recommendation: planner 决定是否添加——建议添加 1 个专项 unit test，成本极低，收益是明确验证二级 HashMap 的 `None` 传播正确

2. **entry() vs contains_key() 的选择**
   - What we know: CONTEXT.md 标记为 Claude's Discretion
   - What's unclear: 是否存在同一 sess+stmt 有多个 PARAMS 记录的场景（覆盖 vs 保留旧值语义）
   - Recommendation: 直接 `entry().or_default().insert()`（覆盖语义），与原 `buffer.insert()` 行为一致

---

## Environment Availability

> Step 2.6: SKIPPED — 本 phase 为纯代码修改，零外部工具依赖。

环境确认：
- Rust 1.94.0 [VERIFIED: `rustc --version`]
- cargo 1.94.0 [VERIFIED: `cargo --version`]
- 现有 `cargo test`（408 tests, 全部 pass）[VERIFIED: 本次运行]

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness + criterion |
| Config file | Cargo.toml（`[dev-dependencies]`） |
| Quick run command | `cargo test` |
| Full suite command | `cargo test && cargo clippy --all-targets -- -D warnings` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| MEM-01 | 二级 HashMap 查询结果与旧实现完全一致 | unit | `cargo test -p dm-database-sqllog2db pipeline::normalizer::tests` | ✅ 已有，扩展即可 |
| MEM-01 | `sess_id` 存在但 `statement` 不存在返回 `None` | unit | `cargo test -p dm-database-sqllog2db pipeline::normalizer::tests::test_nested_lookup_missing_statement` | ❌ Wave 0 新增 |
| MEM-01 | `compute_normalized` 在 PARAMS→DML 完整流程中行为不变 | integration | `cargo test --test integration` | ✅ 已有 |
| MEM-02 | CSV 导出内容与 line_buf 容量无关（内容一致性） | integration | `cargo test --test integration` | ✅ 已有 |

### Sampling Rate

- **Per task commit:** `cargo test --lib`
- **Per wave merge:** `cargo test && cargo clippy --all-targets -- -D warnings`
- **Phase gate:** 全量 test + clippy 绿灯后执行 `cargo bench --bench bench_csv` 验证无吞吐量退化

### Wave 0 Gaps

- [ ] `src/pipeline/normalizer.rs` 中添加 `test_compute_normalized_nested_lookup_missing_statement` 测试（REQ MEM-01 边界）

---

## Security Domain

> 本 phase 不涉及认证、输入验证、加密或访问控制。改动为纯内存分配优化（类型别名 + Vec 容量），无安全影响。安全域不适用。

---

## Sources

### Primary (HIGH confidence)
- Rust std `HashMap` — `Borrow<str>` 支持 `&str` 查询（[VERIFIED: Rust std，rustc 1.94.0 本地验证编译通过]）
- `src/pipeline/normalizer.rs` — 直接读取改动目标代码（行 12, 355–388）
- `src/exporter/csv/exporter.rs` — 直接读取 `line_buf: Vec::with_capacity(2048)`（行 46）
- `src/exporter/csv/writer.rs` — 验证动态 reserve 逻辑存在（行 202-205）
- `src/cli/run/processor.rs` — 验证所有 `ParamBuffer` 使用点（传播影响分析）
- `.planning/phases/74-memory-alloc/74-CONTEXT.md` — 所有锁定决策来源

### Secondary (MEDIUM confidence)
- `src/cli/run/collector.rs`、`sequential.rs`、`tests.rs` — 确认 `ParamBuffer` 使用范围

### Tertiary (LOW confidence)

_无 LOW confidence 条目。_

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — 零新依赖，全部 Rust std，本地编译验证
- Architecture: HIGH — 直接读取源码，改动点明确
- Pitfalls: HIGH — 基于代码分析，非推测

**Research date:** 2026-06-09
**Valid until:** 2026-07-09（Rust std API 极稳定；30 天有效期保守估计）
