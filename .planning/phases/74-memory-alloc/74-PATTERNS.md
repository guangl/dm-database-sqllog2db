# Phase 74: 内存与分配优化 - Pattern Map

**Mapped:** 2026-06-09
**Files analyzed:** 2
**Analogs found:** 2 / 2

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/pipeline/normalizer.rs` | utility (transform) | request-response (hot-loop lookup/insert) | `src/pipeline/normalizer.rs` 自身当前实现 | self — 改动目标即参照 |
| `src/exporter/csv/exporter.rs` | exporter / factory | request-response (CSV serialization init) | `src/exporter/csv/exporter.rs` 自身当前实现 | self — 改动目标即参照 |

两个文件均为原地修改，"最近似 analog"即文件自身的现有实现。下方 Pattern Assignments 直接给出
旧代码→新代码的精确对照，以及需要同步更新的调用点。

---

## Pattern Assignments

### `src/pipeline/normalizer.rs` — MEM-01 (utility, hot-loop lookup/insert)

**改动范围：** 3 处（类型定义 Line 12、PARAMS insert Lines 366-369、DML lookup Lines 386-388）

#### 类型定义 (Line 12)

旧代码：
```rust
pub type ParamBuffer = HashMap<(String, String), Arc<Vec<ParamValue>>>;
```

新代码：
```rust
/// 参数替换缓冲区：外层 key = `sess_id`，内层 key = `statement`。
///
/// 二级结构使查询路径可直接传入 `&str`（HashMap<String,_> 实现了 Borrow<str>），
/// 避免每条 DML 执行记录构造元组 key 时的两次 String::clone。
/// insert 路径（低频 PARAMS 记录）仍需 clone key，可接受。
pub type ParamBuffer = HashMap<String, HashMap<String, Arc<Vec<ParamValue>>>>;
```

#### PARAMS insert (Lines 366-369)

旧代码（当前 Lines 366-369）：
```rust
buffer.insert(
    (record.sess_id.clone(), record.statement.clone()),
    Arc::new(params),
);
```

新代码（entry API，覆盖语义与原 `insert` 一致）：
```rust
buffer
    .entry(record.sess_id.clone())
    .or_default()
    .insert(record.statement.clone(), Arc::new(params));
```

> 注意：直接使用 `entry().or_default().insert()`，无需先 `contains_key` 检查。
> clippy `map_entry` lint 认可此模式；覆盖语义与原 `buffer.insert()` 一致。

#### DML 查询 (Lines 386-388)

旧代码（当前 Lines 386-388）：
```rust
let key = (record.sess_id.clone(), record.statement.clone());

let params = buffer.get(&key)?.clone();
```

新代码（零分配 &str 查询，利用 `HashMap<String,_>: Borrow<str>`）：
```rust
let params = buffer
    .get(record.sess_id.as_str())?
    .get(record.statement.as_str())?
    .clone();
```

> `let key` 行直接删除；两次 `.get(&str)` 不需要任何 String 分配。

#### 新增单元测试（Wave 0 gap）

在 `src/pipeline/normalizer.rs::tests` 末尾追加：

```rust
#[test]
fn test_compute_normalized_nested_lookup_missing_statement() {
    // sess_id 存在但 statement 不存在时应返回 None（不 panic）
    let mut buffer: ParamBuffer = ParamBuffer::new();
    // 先插入 sess_id="s1", statement="stmt_a"
    buffer
        .entry("s1".to_string())
        .or_default()
        .insert("stmt_a".to_string(), Arc::new(vec![]));

    // 查询 sess_id="s1", statement="stmt_b"（不存在）
    let inner = buffer.get("s1");
    assert!(inner.is_some(), "outer key 应存在");
    let result = inner.unwrap().get("stmt_b");
    assert!(result.is_none(), "inner key 不存在时应返回 None");
}
```

---

### `src/exporter/csv/exporter.rs` — MEM-02 (exporter, CSV init)

**改动范围：** 1 处（`CsvExporter::new()` Line 46）

旧代码（Line 46）：
```rust
line_buf: Vec::with_capacity(2048),
```

新代码：
```rust
// 典型 DaMeng SQL + 字段开销约 1–4KB；writer.rs 的动态 reserve 兜底更长 SQL
line_buf: Vec::with_capacity(4096),
```

> `writer.rs:202-205` 的动态 reserve 逻辑保留不变（不修改）。

---

## Shared Patterns

### `HashMap<String, V>` 的 `&str` 零分配查询

**Source:** Rust std — `HashMap<K: Borrow<Q>, V>::get(&Q)`
**Apply to:** `src/pipeline/normalizer.rs` DML 查询路径

```rust
// 因为 String: Borrow<str>，以下调用无需创建 String
map.get("some_str_key")          // 直接传 &str 字面量
map.get(record.sess_id.as_str()) // 或 .as_str() 取引用
```

### `entry().or_default()` insert 惯用法

**Source:** Rust std HashMap entry API
**Apply to:** `src/pipeline/normalizer.rs` PARAMS insert 路径

```rust
outer_map
    .entry(owned_key.clone())
    .or_default()
    .insert(inner_key.clone(), value);
```

### `writer.rs` 动态 reserve 兜底（保留不变）

**Source:** `src/exporter/csv/writer.rs` Lines 202-205
**Apply to:** 不修改，仅确认 `line_buf` 容量增大后此逻辑仍正确兜底超长 SQL

```rust
let needed = 128 + sqllog.sql.len() + ns_len;
if line_buf.capacity() < needed {
    line_buf.reserve(needed - line_buf.len());
}
```

---

## Callsite Propagation（类型变更传播）

`ParamBuffer` 改为二级结构后，以下调用点需同步更新——仅当调用点直接操作 key 时：

| File | Line | 当前用法 | 是否需要修改 |
|------|------|----------|-------------|
| `src/cli/run/processor.rs` | 6 | `use … ParamBuffer;` | 否——类型别名，自动适应 |
| `src/cli/run/processor.rs` | 201 | `params_buffer.clear()` | 否——`clear()` 语义不变 |
| `src/cli/run/collector.rs` | 36 | `ParamBuffer::default()` | 否——`HashMap::default()` 适用于二级结构 |
| `src/cli/run/sequential.rs` | 72 | `ParamBuffer::default()` | 否——同上 |
| `src/cli/run/tests.rs` | 311 | `ParamBuffer::new()` | 否——`HashMap::new()` 适用于二级结构 |
| `src/cli/run/tests.rs` | 342-348 | `let buf_key = ("sess_gap1".to_string(), "stmt_gap1".to_string()); params_buffer.contains_key(&buf_key)` | **是**——需改为二级查询 |
| `src/cli/run/tests.rs` | 389 | `ParamBuffer::new()` | 否——同上 |

### `tests.rs:342-348` 需要更新的断言

旧代码（`tests.rs:342-348`）：
```rust
let buf_key = ("sess_gap1".to_string(), "stmt_gap1".to_string());
assert!(
    params_buffer.contains_key(&buf_key),
    "...",
    buf_key,
    params_buffer.keys().collect::<Vec<_>>()
);
```

新代码：
```rust
assert!(
    params_buffer
        .get("sess_gap1")
        .and_then(|inner| inner.get("stmt_gap1"))
        .is_some(),
    "passes=false+do_normalize=true 下 PARAMS 记录应写入 params_buffer，\
     但 key (sess_gap1, stmt_gap1) 不存在; outer keys={:?}",
    params_buffer.keys().collect::<Vec<_>>()
);
```

---

## No Analog Found

无。两个改动文件均有明确的自身现有实现作为参照，且 Rust std HashMap/Vec API 无需外部 analog。

---

## Metadata

**Analog search scope:** `src/pipeline/`, `src/exporter/csv/`, `src/cli/run/`
**Files scanned:** 7（`normalizer.rs`, `exporter.rs`, `writer.rs`, `processor.rs`, `collector.rs`, `sequential.rs`, `tests.rs`）
**Pattern extraction date:** 2026-06-09
