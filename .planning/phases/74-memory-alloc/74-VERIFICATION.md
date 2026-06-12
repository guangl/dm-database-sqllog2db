---
phase: 74-memory-alloc
verified: 2026-06-09T06:00:00Z
status: human_needed
score: 9/9 must-haves verified
overrides_applied: 0
human_verification:
  - test: "运行 criterion benchmark 与 v1.20 baseline 对比"
    expected: "CSV 导出吞吐量不退化（per 74-VALIDATION.md Manual-Only Verifications）"
    why_human: "需要执行 `CRITERION_HOME=benches/baselines cargo bench --bench bench_csv -- --baseline v1.20`，需要 v1.20 baseline 已存档；无法在静态代码检查中验证"
---

# Phase 74: memory-alloc Verification Report

**Phase Goal:** Eliminate per-record allocation overhead in the hot path — specifically the 2x String::clone in the normalizer (MEM-01) and frequent reallocation of line_buf in the CSV exporter (MEM-02).
**Verified:** 2026-06-09T06:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | normalizer DML 查询路径不再为构造 key 而 clone sess_id / statement | VERIFIED | `normalizer.rs:390-393` 使用 `.get(record.sess_id.as_str())?.get(record.statement.as_str())?`，无 `String::clone`；旧 `let key = (record.sess_id.clone(), ...)` 已消除 |
| 2 | PARAMS insert 路径改用 entry().or_default().insert() 模式且语义与原 insert 一致 | VERIFIED | `normalizer.rs:370-373` 确认 `buffer.entry(record.sess_id.clone()).or_default().insert(record.statement.clone(), Arc::new(params))` |
| 3 | cli/run/tests.rs 的断言更新为二级 get().and_then().is_some() 形式 | VERIFIED | `tests.rs:342-350` 确认 `params_buffer.get("sess_gap1").and_then(|inner| inner.get("stmt_gap1")).is_some()`；旧元组 `contains_key` 已删除 |
| 4 | sess_id 存在但 statement 不存在时 compute_normalized 不 panic 且返回 None | VERIFIED | `normalizer.rs:644-658` 测试 `test_compute_normalized_nested_lookup_missing_statement` 存在并通过（27/27 normalizer tests passed） |
| 5 | cargo test 全套通过，无任何回归 | VERIFIED | 全套 940 个测试通过（409 lib + 440 lib + 3 + 87 integration + 1 + 7 watch），0 failed |
| 6 | cargo clippy --all-targets -- -D warnings 通过，无新增警告 | VERIFIED | clippy 退出码 0，0 warnings |
| 7 | CsvExporter::new() 中 line_buf 初始容量从 2048 提升到 4096 | VERIFIED | `exporter.rs:47` 确认 `line_buf: Vec::with_capacity(4096)` |
| 8 | 新容量上方有注释说明依据 | VERIFIED | `exporter.rs:46` 确认注释 `// 典型 DaMeng SQL + 字段开销约 1–4KB；writer.rs 的动态 reserve 兜底更长 SQL` |
| 9 | writer.rs:202-205 动态 reserve 逻辑保持不变（兜底超长 SQL） | VERIFIED | `writer.rs:202-205` 确认 `if line_buf.capacity() < needed { line_buf.reserve(needed - line_buf.len()); }` 保持不变 |

**Score:** 9/9 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|---------|--------|---------|
| `src/pipeline/normalizer.rs` | ParamBuffer 二级类型 + compute_normalized 零分配查询 + 边界单元测试 | VERIFIED | Line 16: `HashMap<String, HashMap<String, Arc<Vec<ParamValue>>>>`；Lines 390-393: 零分配 `.get(&str)` 查询链；Lines 644-658: 边界测试存在且通过 |
| `src/cli/run/tests.rs` | 二级 HashMap 断言更新 | VERIFIED | Lines 342-350: 使用 `.get("sess_gap1").and_then(|inner| inner.get("stmt_gap1")).is_some()`，旧 tuple key 已删除 |
| `src/exporter/csv/exporter.rs` | CsvExporter::new() 的 line_buf 预热容量 4096 + 注释 | VERIFIED | Line 46-47: 注释 + `Vec::with_capacity(4096)` |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `normalizer.rs compute_normalized DML 分支` | ParamBuffer 内层 HashMap | `.get(sess_id.as_str())?.get(statement.as_str())?` | WIRED | Lines 390-393 确认双层 `?` 链，Borrow<str> 零分配 |
| `normalizer.rs PARAMS 分支` | ParamBuffer 外层 entry | `buffer.entry(sess_id.clone()).or_default().insert(statement.clone(), Arc::new(params))` | WIRED | Lines 370-373 确认 entry + or_default 模式 |
| `exporter.rs CsvExporter::new()` | Vec<u8> line_buf 初始容量 | `Vec::with_capacity(4096)` | WIRED | Line 47 确认 |

---

### Data-Flow Trace (Level 4)

MEM-01 是内存分配路径优化（HashMap 类型重构），不涉及从数据源到渲染的数据流链；
MEM-02 是 Vec 预分配容量调整，属于内部性能参数。
两处改动均不属于"渲染动态数据的组件"——Level 4 Data-Flow Trace 不适用于本 phase。

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| normalizer 边界测试通过 | `cargo test --lib pipeline::normalizer::tests` | 27 passed, 0 failed | PASS |
| cli/run 集成测试通过（含二级 HashMap 断言） | `cargo test --lib cli::run::tests` (包含在 409 lib 中) | 409 passed, 0 failed | PASS |
| 全套测试无回归 | `cargo test` | 940+ passed, 0 failed | PASS |
| clippy 无新增 lint | `cargo clippy --all-targets -- -D warnings` | 0 warnings, exit 0 | PASS |

---

### Probe Execution

本 phase 无 probe-*.sh 文件，跳过。

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| MEM-01 | 74-01-PLAN.md | normalizer 热路径 HashMap key 不再每条记录重复 clone String | SATISFIED | `ParamBuffer` 类型改为二级结构，DML 查询改用 `&str`（Borrow<str>），消除每条记录 2 次 `String::clone`；commit 8179893 |
| MEM-02 | 74-02-PLAN.md | CSV line_buf 初始容量按典型 SQL 长度预热，减少 Vec grow 次数 | SATISFIED | `Vec::with_capacity(4096)` 替代 2048，含注释说明依据；commit 8c54845 |

REQUIREMENTS.md 追溯表中 MEM-01 / MEM-02 均映射到 Phase 74，两个需求均有计划文件覆盖，
无孤儿需求（Phase 74 未声明 BENCH、SQLITE、STRUCT、ASYNC 等其他 ID）。

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|---------|--------|
| — | — | — | — | — |

扫描结果：
- `grep -nE 'TBD|FIXME|XXX'` — 0 匹配（无未解决的 debt 标记）
- `grep -n 'unsafe'` normalizer.rs — 0 匹配（无新增 unsafe）
- `grep -nE 'return null|return \[\]|placeholder|TODO|HACK'` 三个修改文件 — 0 关键匹配
- 无空实现、无硬编码空数据、无仅 console.log 的处理函数

---

### Human Verification Required

#### 1. Benchmark 吞吐量对比（不退化）

**Test:** 在具有 v1.20 baseline 的机器上运行：
`CRITERION_HOME=benches/baselines cargo bench --bench bench_csv -- --baseline v1.20`

**Expected:** CSV 导出吞吐量不低于 v1.20 baseline（per 74-VALIDATION.md `Manual-Only Verifications`）

**Why human:** 需要 criterion baseline 已在 benches/baselines/ 下存档；benchmark 需要稳定硬件环境和实际日志文件；无法通过静态代码分析验证吞吐量

---

### Gaps Summary

无代码层面 gap。全部 9 项 must-have 通过代码级验证，两项需求（MEM-01 / MEM-02）均完整实现。
唯一待确认项是 benchmark 吞吐量不退化，属于手动验证项。

---

_Verified: 2026-06-09T06:00:00Z_
_Verifier: Claude (gsd-verifier)_
