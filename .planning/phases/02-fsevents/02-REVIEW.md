---
phase: 02-fsevents
reviewed: 2026-06-07T10:00:00Z
depth: standard
files_reviewed: 3
files_reviewed_list:
  - tests/watch_incremental.rs
  - src/cli/run/tests.rs
  - src/cli/run/filter_processor.rs
findings:
  critical: 2
  warning: 3
  info: 2
  total: 7
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-06-07T10:00:00Z
**Depth:** standard
**Files Reviewed:** 3
**Status:** issues_found

## Summary

审查了三个文件：`tests/watch_incremental.rs`（集成测试），`src/cli/run/tests.rs`（单元测试），`src/cli/run/filter_processor.rs`（过滤器管道构建）。

在 `filter_processor.rs` 的 `build_pipeline` 中发现一处逻辑缺陷——当 `indicators`/`sql` 过滤器配置存在但预扫描未匹配到任何事务时，所有记录会被导出（过滤失效）。`tests/watch_incremental.rs` 的测试辅助函数 `count_rows` 存在 SQL 注入风险。测试整体质量较高，断言覆盖合理，但存在数个可靠性和可维护性问题。

## Critical Issues

### CR-01: `build_pipeline` 在 indicators/sql 过滤器无命中时完全失效

**File:** `src/cli/run/filter_processor.rs:9`

**Issue:** `build_pipeline` 仅在 `f.include.has_filters() || f.exclude.has_filters()` 时才添加 `FilterProcessor`。当用户配置了 `[filter.indicators]` 或 `[filter.sql]` 但预扫描（prescan）未找到任何匹配的事务 ID 时：
1. `scan_for_trxids_by_transaction_filters` 返回空列表
2. `merge_found_trxids` 检查 `trxids.is_empty()` 为真，提前返回，`include.trxids` 保持 `None`
3. `include.has_filters()` 返回 `false`
4. `build_pipeline` 不添加任何 `FilterProcessor`
5. 所有记录都通过空 pipeline，**全量导出**

期望行为：当 `indicators`/`sql` 过滤器有效配置但无匹配事务时，应导出 0 条记录。

**Fix:**
```rust
pub(super) fn build_pipeline(cfg: &Config) -> Pipeline {
    let mut pipeline = Pipeline::new();
    if let Some(f) = cfg.filter.as_ref() {
        if f.enable && (f.include.has_filters() || f.exclude.has_filters()) {
            pipeline.add(Box::new(FilterProcessor::from_feature(f)));
        } else if f.enable && f.has_transaction_filters() {
            // indicators/sql 过滤器已通过 prescan 转换为 trxids；
            // 若 trxids 为空（无命中），需要添加一个拒绝所有记录的过滤器
            // 而不是让全部记录通过。
            pipeline.add(Box::new(RejectAllProcessor));
        }
    }
    pipeline
}
```

或者更简洁地，在 `merge_found_trxids` 中当列表为空时插入一个 sentinel（空 HashSet），使 `include.has_filters()` 在有 indicators/sql 过滤器时返回 true：

```rust
pub(crate) fn merge_found_trxids(&mut self, trxids: Vec<String>) {
    if !self.enable {
        return;
    }
    // 即使 trxids 为空，也要插入空 HashSet 表示"已预扫描但无命中"，
    // 触发 FilterProcessor 在主扫描中拒绝所有记录
    self.include
        .trxids
        .get_or_insert_with(TrxidSet::default)
        .extend(trxids);
}
```

---

### CR-02: `count_rows` 辅助函数存在 SQL 注入漏洞（测试代码）

**File:** `tests/watch_incremental.rs:70`

**Issue:** `count_rows` 使用 `format!` 宏将 `table` 参数拼接进 SQL 字符串：

```rust
let query = format!("SELECT COUNT(*) FROM \"{table}\"");
```

虽然所有调用点目前都传入硬编码字符串 `"sqllog_records"`，但该模式在测试代码中依然是不良示例。若将来有测试使用动态或含特殊字符的表名（如 `sqllog"; DROP TABLE sqllog_records; --`），SQLite 的双引号转义仍可能被绕过。测试工具函数应使用参数化查询。

**Fix:**
```rust
fn count_rows(db_path: &Path, table: &str) -> i64 {
    let Ok(conn) = Connection::open(db_path) else {
        return 0;
    };
    // 使用硬编码表名替换，完全避免注入风险
    // 或者对表名进行白名单验证
    let query = format!("SELECT COUNT(*) FROM \"{}\"", table.replace('"', ""));
    conn.query_row(&query, [], |row| row.get::<_, i64>(0))
        .unwrap_or(0)
}
```

## Warnings

### WR-01: `test_watch_04` 的断言 `new_size > offset_after_full` 存在竞态

**File:** `tests/watch_incremental.rs:219-220`

**Issue:** `offset_after_full`（第 172 行）在 `trigger_full_file` 返回后立即捕获，此时 `log_path` 尚未追加 7 条新记录（写入发生在第 202 行）。断言 `new_size > offset_after_full` 实际上只是在验证"写入 7 条后文件确实变大了"，而不是验证 offset 恢复是否正确。这个断言不会失败，但也不测试预期属性。若文件系统缓存延迟导致 `metadata().len()` 返回旧值（不太可能但理论上可能），测试会虚通过。

更有意义的断言是验证 `state2.file_offsets` 中的 offset 等于 `new_size`。

**Fix:**
```rust
// 验证 state2 的 offset 更新为最新文件大小（这才是真正需要验证的属性）
let new_size = std::fs::metadata(&log_path).unwrap().len();
let canonical_log2 = log_path.canonicalize().unwrap();
let recorded_offset = state2.file_offsets().get(&canonical_log2).copied();
assert_eq!(
    recorded_offset,
    Some(new_size),
    "增量触发后 state2.file_offsets 应记录最新文件大小"
);
assert!(new_size > offset_after_full, "文件应已增长");
```

---

### WR-02: `filter_processor.rs` 中 `build_or_group` 映射关系缺少覆盖 `has_filters()` 条件的测试

**File:** `src/cli/run/filter_processor.rs:9`

**Issue:** `build_pipeline` 的触发条件是 `f.include.has_filters() || f.exclude.has_filters()`，但 `make_feature` 测试辅助函数（第 155 行）不通过 `FiltersFeature::default()` 展开，而是手动构造，这意味着 `indicators` 和 `sql` 字段始终为 `Default::default()`。没有任何 `filter_processor.rs` 内部测试验证 `build_pipeline` 在仅有 `indicators` 或 `sql` 过滤器时的行为，从而掩盖了 CR-01 描述的缺陷。

**Fix:** 添加测试，覆盖 `indicators`-only 和 `sql`-only 配置传入 `build_pipeline` 后管道状态的验证。

---

### WR-03: `test_watch_03_incremental_appends_only_new_rows` 中 `start_id` 参数有歧义

**File:** `tests/watch_incremental.rs:127`

**Issue:** `write_test_log_records(&log_path, 10, 5)` 第一个参数是 `start_id`，决定记录中的 trxid、sess_id 等字段。由于 trxid 值直接拼入日志行，当 `start_id=10` 时所有记录的 trxid 均从 `10` 开始，与第一批 `start_id=0` 的记录在 trxid 上连续。在有事务过滤器时这种设计可能导致测试假阳性：trxid 连续不等于行不重复。

当前测试不使用事务过滤器，所以没有实际 bug，但测试的语义假设"ID 不同即内容不同，插入不重复"没有被显式断言验证。建议注释说明 `start_id` 为何需要与前一批不重叠。

**Fix:** 在 `write_test_log_records` 的调用处添加注释，明确 `start_id` 必须与已有记录不重叠，以保证 SQLite 中无重复行（若表有唯一约束）或结果行数可预期。

## Info

### IN-01: `tests/watch_incremental.rs` 中 `use std::sync::atomic::AtomicBool` 直接引用而未 `use std::sync::atomic::Ordering`

**File:** `tests/watch_incremental.rs:17`

**Issue:** 文件顶部只导入了 `AtomicBool`，未导入 `Ordering`。`Ordering` 在测试中未直接使用（`AtomicBool::new(true)` 不需要 Ordering），但如果将来扩展测试需要手动 `store`/`load`，需要记住补充导入。这是对比 `src/cli/run/tests.rs` 第 766 行中 `use std::sync::atomic::{AtomicBool, Ordering}` 的不一致之处（那个测试文件在局部 `use` 中导入了 Ordering）。纯代码一致性问题。

**Fix:** 若将来无需 `Ordering`，保持现状；若测试扩展则与其他测试文件保持一致的 `use` 风格。

---

### IN-02: `filter_processor.rs` 单元测试与集成测试中大量重复的 `make_record` 构造

**File:** `src/cli/run/filter_processor.rs:136-153`
**File:** `src/cli/run/tests.rs:275-306`

**Issue:** `filter_processor.rs` 内部测试的 `make_record` 与 `tests.rs` 中的 `Sqllog` 字面量构造均包含相同的模板字段（`ep: 0`, `exectime: 0.0`, 等）。两处构造方式不同但无实质差异，增加维护成本。这是测试代码重复，不影响正确性。

**Fix:** 考虑将 `make_record` 移入 `#[cfg(test)]` 的公共测试工具模块，供多个测试文件复用。

---

_Reviewed: 2026-06-07T10:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
