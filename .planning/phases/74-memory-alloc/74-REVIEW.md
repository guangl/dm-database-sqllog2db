---
phase: 74-memory-alloc
reviewed: 2026-06-09T00:00:00Z
depth: standard
files_reviewed: 3
files_reviewed_list:
  - src/pipeline/normalizer.rs
  - src/cli/run/tests.rs
  - src/exporter/csv/exporter.rs
findings:
  critical: 0
  warning: 3
  info: 2
  total: 5
status: issues_found
---

# Phase 74: Code Review Report

**Reviewed:** 2026-06-09
**Depth:** standard
**Files Reviewed:** 3
**Status:** issues_found

## Summary

本次 Phase 74 变更包含两处优化：MEM-01（`ParamBuffer` 从 `HashMap<(String,String),V>` 重构为嵌套 `HashMap<String,HashMap<String,V>>`，消除热路径 key 构造时的两次 `String::clone`）和 MEM-02（`line_buf` 初始容量从 2048 提升到 4096）。

整体实现逻辑正确，嵌套查找路径（`get(sess_id).get(stmt)`）如设计所述避免了 key 克隆，API 边界一致。发现3个 WARNING 和2个 INFO，无 BLOCKER。

---

## Warnings

### WR-01: `write_record_preparsed` 容量预留计算在 `clear()` 后冗余且存在 `reserve` 语义混淆

**File:** `src/exporter/csv/writer.rs:196-204`（间接关联 MEM-02，与 `line_buf` 初始容量升级相关）

**Issue:**

```rust
line_buf.clear();                          // len → 0，capacity 不变
let needed = 128 + sqllog.sql.len() + ns_len;
if line_buf.capacity() < needed {
    line_buf.reserve(needed - line_buf.len());   // line_buf.len() == 0 此处恒为 0
}
```

`clear()` 后 `len` 恒为 0，因此 `needed - line_buf.len()` 始终等于 `needed`。`Vec::reserve(n)` 的语义是确保 *剩余* 容量 `capacity - len >= n`，即 `capacity - 0 >= needed`，所以等价于确保 `capacity >= needed`——与手动检查 `capacity < needed` 的条件语义重复。两者结合虽然逻辑正确，但多出一次手动检查，可读性差，且让维护者误以为存在特殊的"跳过 reserve"优化路径。

**Fix:**

```rust
line_buf.clear();
let needed = 128 + sqllog.sql.len() + ns_len;
// reserve 自带 capacity 检查，不需要手动 if 包裹
line_buf.reserve(needed);
```

---

### WR-02: `open_for_write` 在 `append=false` 且 `overwrite=false` 时静默截断文件

**File:** `src/exporter/csv/exporter.rs:56-65`

**Issue:**

`from_config` 中配置映射逻辑：

```rust
if config.append {
    e.write_mode = WriteMode::Append;
} else if config.overwrite {
    e.write_mode = WriteMode::Truncate;
}
// 若 append=false 且 overwrite=false → write_mode 保持初始值 WriteMode::Truncate
```

当用户配置 `overwrite = false` 且 `append = false` 时，`write_mode` 保持默认值 `WriteMode::Truncate`，`open_for_write` 仍会调用 `.truncate(true)`，悄悄覆盖已有文件，与用户期望的"两者均关闭"语义相反。虽然 `validate()` 未必阻止此配置，但这是静默数据破坏行为，应当在 `from_config` 或 `validate()` 处明确处理。

**Fix:**

在 `CsvExporterConfig::validate()` 中增加互斥验证：

```rust
pub fn validate(&self) -> Result<()> {
    if self.file.trim().is_empty() { /* ... */ }
    if !self.overwrite && !self.append {
        return Err(Error::Config(ConfigError::InvalidValue {
            field: "exporter.csv".to_string(),
            value: "overwrite=false, append=false".to_string(),
            reason: "must set either overwrite=true or append=true".to_string(),
        }));
    }
    Ok(())
}
```

或者在 `from_config` 末尾加断言/文档说明此 fallback 的语义属有意设计。

---

### WR-03: `test_compute_normalized_nested_lookup_missing_statement` 测试并未通过 `compute_normalized` 执行，是空洞测试

**File:** `src/cli/run/tests.rs:644-658`

**Issue:**

该测试名称为 `test_compute_normalized_nested_lookup_missing_statement`，但实际上并没有调用 `compute_normalized`。它仅对 `HashMap` 的手工插入和 `get` 进行验证，等同于测试标准库而非业务逻辑。如果 `compute_normalized` 内部的 `buffer.get(sess_id)?.get(stmt)?` 被意外改成 panic 路径，本测试也不会失败。

Phase 74 的 MEM-01 核心变更（嵌套 map 查找路径不触发 clone）完全没有回归保护，测试覆盖存在空洞。

**Fix:**

将测试改为调用 `compute_normalized`，构造 `record.sess_id="s1"`, `record.statement="stmt_b"`（不存在的内层 key），验证返回 `None` 且不 panic：

```rust
#[test]
fn test_compute_normalized_nested_lookup_missing_statement() {
    use crate::pipeline::normalizer::{ParamBuffer, compute_normalized};
    use dm_database_parser_sqllog::Sqllog;
    use std::sync::Arc;

    let mut buffer = ParamBuffer::new();
    buffer.entry("s1".to_string()).or_default()
        .insert("stmt_a".to_string(), Arc::new(vec![]));

    let record = Sqllog {
        sess_id: "s1".to_string(),
        statement: "stmt_b".to_string(),
        tag: Some("SEL".to_string()),
        sql: "SELECT ?".to_string(),
        // ... 其余字段填默认值
    };
    let mut scratch = Vec::new();
    let result = compute_normalized(&record, "SELECT ?", &mut buffer, None, &mut scratch);
    assert!(result.is_none(), "stmt_b 不存在时应返回 None");
}
```

---

## Info

### IN-01: `ParamBuffer` 无上界，长期运行时可能无限增长

**File:** `src/pipeline/normalizer.rs:16`

**Issue:**

`ParamBuffer` 是一个 `HashMap<String, HashMap<String, Arc<Vec<ParamValue>>>>`，在 `compute_normalized` 的 PARAMS 分支会持续向其插入条目（第371-373行），但没有任何逐出策略。单次 `run` 命令处理完整日志文件后，map 内所有 `sess_id + statement` 组合都会被保留至处理结束。对于 `watch` 模式（持续处理）或超大日志文件（数百万唯一 session/statement 组合），此结构会无限增长占用内存。

CLAUDE.md 中描述的设计目标是"constant memory regardless of file size"，目前仅对 `sequential` 路径成立（`params_buffer` 在文件边界 `clear()`），`collector.rs` 中的每文件 `params_buf` 也是本地作用域，但 `watch` 模式复用同一 `ParamBuffer` 的场景需确认。

**Fix:** 记录已知限制，或在文档注释中明确说明 `ParamBuffer` 应在每个"处理单元"边界被 `clear()`。

---

### IN-02: `write_record` 函数是 `write_record_preparsed` 的纯转发包装，可删除

**File:** `src/exporter/csv/writer.rs:237-261`

**Issue:**

`write_record` 函数（第237-261行）是 `write_record_preparsed` 的逐参数转发，没有任何额外逻辑，两者实现完全相同。文档注释说这是"兼容路径"，但 API 中已全面使用 `write_record_preparsed`。额外的公开入口增加维护面，且 `pub(in crate::exporter::csv)` 可见性表明它不是外部 API。

**Fix:** 确认无外部调用后，用 `type alias` 或直接将调用方改为调用 `write_record_preparsed`，删除此包装函数。

---

_Reviewed: 2026-06-09_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
