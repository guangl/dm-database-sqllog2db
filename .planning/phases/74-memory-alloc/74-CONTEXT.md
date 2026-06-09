# Phase 74: 内存与分配优化 - Context

**Gathered:** 2026-06-09
**Status:** Ready for planning

<domain>
## Phase Boundary

消除 normalizer 热路径中每条记录的重复 String clone（MEM-01），并将 CSV `line_buf` 初始容量预热至合理值（MEM-02），在不引入新 unsafe 或重量级依赖的前提下降低堆分配压力。

本 phase 不涉及 profiling 分析（PROF-01/02）、异步迁移（Phase 76）、并行路径重构（Phase 75）。

</domain>

<decisions>
## Implementation Decisions

### MEM-01: HashMap key 消除策略

[auto] Q: "如何消除 `compute_normalized` 中每条记录的 HashMap key String clone？" → Selected: "二级 HashMap" (推荐默认)

- **D-01:** 将 `ParamBuffer` 改为二级结构：
  ```rust
  pub type ParamBuffer = HashMap<String, HashMap<String, Arc<Vec<ParamValue>>>>;
  ```
  查询路径完全零分配：
  ```rust
  let params = buffer.get(record.sess_id.as_str())?.get(record.statement.as_str())?.clone();
  ```
  insert 路径仅在 PARAMS 记录时 clone（频率远低于执行记录，可接受）：
  ```rust
  buffer.entry(record.sess_id.clone())
        .or_default()
        .insert(record.statement.clone(), Arc::new(params));
  ```
- **D-02:** 不使用自定义 `Borrow` impl（复杂，需 nightly 特性或 unsafe）；不使用 `Arc<str>` key（`Arc::from(&str)` 仍会在每次查询时分配新 Arc，不解决问题）
- **D-03:** 当前 `normalizer.rs:386` 的 `let key = (record.sess_id.clone(), record.statement.clone())` + `buffer.get(&key)` 模式是改动目标；PARAMS insert 路径（`normalizer.rs:367-369`）也随之更新为 `entry` API

### MEM-02: line_buf 初始容量预热

[auto] Q: "line_buf 初始容量应设为多少？" → Selected: "4096 字节" (推荐默认)

- **D-04:** 将 `CsvExporter::new()` 中的 `line_buf: Vec::with_capacity(2048)` 改为 `Vec::with_capacity(4096)`
  - 理由：典型 DaMeng SQL 语句（含 INSERT/SELECT + WHERE 条件 + 标识符）常在 1–4KB 范围；4096 覆盖绝大多数记录的首次写入，避免冷启动 Vec grow
  - `writer.rs:202-205` 的动态 `reserve` 机制保留不变，它是正确的兜底（处理超过初始容量的 SQL）
  - 代码注释说明：`// 典型 DaMeng SQL + 字段开销约 1–4KB；writer.rs 的动态 reserve 兜底更长 SQL`
- **D-05:** 不修改 `writer.rs` 的动态 reserve 逻辑（已正确处理任意大小 SQL，无需改动）

### 测试要求

- **D-06:** 两项优化均须有对应测试确保行为不变：
  - MEM-01：参数替换结果与优化前完全一致（可扩展现有 `normalizer.rs::tests` 中的用例）
  - MEM-02：CSV 导出内容不变（现有集成测试覆盖此路径，无需额外新增，但需全量跑通）
- **D-07:** `cargo clippy --all-targets -- -D warnings` + `cargo test` 全部通过，无新增 unsafe

### Claude's Discretion

- PARAMS insert 路径的 `entry` API 是否先 `contains_key` 检查 — planner 决定（性能与可读性权衡）
- 单元测试是否额外覆盖二级 HashMap 的空 inner map 场景（`sess_id` 存在但 `statement` 不存在）

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Normalizer 核心路径
- `src/pipeline/normalizer.rs` — `ParamBuffer` 类型定义（Line 12）、`compute_normalized`（Line 355）、PARAMS insert（Line 367-369）、lookup（Line 386-388）——改动目标
- `src/pipeline/mod.rs` — Pipeline 入口，`compute_normalized` 的调用上下文

### CSV 导出路径
- `src/exporter/csv/exporter.rs` — `CsvExporter::new()` Line 46，`line_buf: Vec::with_capacity(2048)` — MEM-02 改动目标
- `src/exporter/csv/writer.rs` — `write_record_preparsed` Line 196-205，动态 reserve 逻辑（保留不变）

### 需求与验收标准
- `.planning/ROADMAP.md` §"Phase 74: 内存与分配优化"（Line 978）— Goal、Success Criteria 1–4
- `.planning/REQUIREMENTS.md` §MEM-01、MEM-02

### 前序参考
- `.planning/phases/73-sqlite-batch-insert/73-CONTEXT.md` — Phase 73 性能优化模式参考（batch INSERT 决策）
- `.planning/phases/72-bench-baseline/72-CONTEXT.md` — v1.20 criterion baseline，Phase 74 优化后可用 `--baseline v1.20` 验证无退化

</canonical_refs>

<code_context>
## Existing Code Insights

### 改动目标
- `ParamBuffer`（`normalizer.rs:12`）：当前 `HashMap<(String, String), Arc<Vec<ParamValue>>>`，改为 `HashMap<String, HashMap<String, Arc<Vec<ParamValue>>>>`
- `compute_normalized` lookup（`normalizer.rs:386`）：`let key = (record.sess_id.clone(), record.statement.clone())` + `buffer.get(&key)?.clone()` — 每条执行记录 2 次 String clone
- `compute_normalized` insert（`normalizer.rs:367-369`）：`buffer.insert((record.sess_id.clone(), ...)` — PARAMS 记录 insert，clone 不可避免但频率低
- `CsvExporter::new()`（`exporter.rs:46`）：`line_buf: Vec::with_capacity(2048)` — 预热容量偏小

### 保持不变
- `write_record_preparsed` 的动态 reserve 逻辑（`writer.rs:202-205`）：正确处理任意大小 SQL，无需改动
- `Arc<Vec<ParamValue>>` 包装：热路径 `.clone()` 已是 O(1) 原子操作，保留不变（D-03 中 `params.clone()` 不是问题）
- 参数替换算法（`apply_params_into`）：不涉及分配优化，不改动

### 已有测试覆盖
- `src/pipeline/normalizer.rs::tests`：`parse_params`、`apply_params`、`count_placeholders` 全覆盖
- `tests/` 集成测试：CSV 导出内容一致性测试（MEM-02 的隐式回归保障）

</code_context>

<specifics>
## Specific Ideas

- ROADMAP success criteria #1 提到"改用 Arc<str> 或调整生命周期"，但 `Arc<str>` lookup 仍需 `Arc::from(&str)` 分配，二级 HashMap 更彻底消除分配
- ROADMAP success criteria #2 提到"如 512 字节"作为示例，实际选 4096 更符合 DaMeng SQL 典型长度
- 二级 HashMap 的 `entry().or_default()` 模式与 Rust 标准库惯例一致，clippy 无警告

</specifics>

<deferred>
## Deferred Ideas

- heaptrack/massif 峰值内存 profiling（PROF-02）— Future phase，需要真实大文件环境
- flamegraph CPU 热点分析（PROF-01）— Future phase
- normalizer PARAMS 记录 insert 路径的进一步优化（intern pool）— 投入产出比低，defer

</deferred>

---

*Phase: 74-memory-alloc*
*Context gathered: 2026-06-09*
