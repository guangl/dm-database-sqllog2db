---
phase: 02-fsevents
fixed_at: 2026-06-07T10:30:00Z
review_path: .planning/phases/02-fsevents/02-REVIEW.md
iteration: 1
findings_in_scope: 7
fixed: 5
skipped: 2
status: partial
---

# Phase 02: Code Review Fix Report

**Fixed at:** 2026-06-07T10:30:00Z
**Source review:** .planning/phases/02-fsevents/02-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 7
- Fixed: 5
- Skipped: 2

## Fixed Issues

### CR-01: `build_pipeline` 在 indicators/sql 过滤器无命中时完全失效

**Files modified:** `src/pipeline/filters/mod.rs`, `src/pipeline/filters/types.rs`, `src/cli/run/filter_processor.rs`
**Commit:** 961cf34
**Applied fix:**
1. `merge_found_trxids`：移除 `trxids.is_empty()` 提前返回，改为始终调用 `get_or_insert_with(TrxidSet::default).extend(trxids)`，使预扫描无命中时也插入空集合作为 sentinel
2. `IncludeFilters::has_filters()`：将 trxids 判断从 `is_some_and(|s| !s.is_empty())` 改为 `is_some()`，使空集合 sentinel 也返回 true
3. `FilterProcessor::from_feature` 的 `has_meta_filters`：同样改为 `trxid_set.is_some()`
4. `FilterProcessor::process_with_meta`：将空集合的 trxids 由跳过检查改为拒绝所有记录（`trxids.is_empty() || !trxids.contains(...)`）
5. 更新 `test_merge_found_trxids_empty_list` 测试以反映新行为契约

---

### CR-02: `count_rows` 辅助函数存在 SQL 注入漏洞

**Files modified:** `tests/watch_incremental.rs`
**Commit:** 314414e
**Applied fix:** 在 count_rows 中对表名调用 `table.replace('"', "")` 剥离双引号，防御性处理 SQL 注入风险

---

### WR-01: `test_watch_04` 的断言 `new_size > offset_after_full` 存在竞态

**Files modified:** `tests/watch_incremental.rs`
**Commit:** fee8f0a
**Applied fix:** 在现有断言 `new_size > offset_after_full` 之前添加对 `state2.file_offsets()` 的断言，验证增量触发后 file_offsets 记录的 offset 等于当前文件大小，真正验证 offset 恢复的正确性

---

### WR-02: `filter_processor.rs` 中缺少覆盖 `has_filters()` 条件的测试

**Files modified:** `src/cli/run/filter_processor.rs`
**Commit:** fb4690f
**Applied fix:** 新增两个测试：
- `test_empty_trxid_sentinel_rejects_all_records`：验证 trxids 为空集合（预扫描无命中 sentinel）时 FilterProcessor 拒绝所有记录
- `test_nonempty_trxid_set_filters_correctly`：验证 trxids 非空时正确按 trxid 过滤记录

---

### WR-03: `test_watch_03_incremental_appends_only_new_rows` 中 `start_id` 参数有歧义

**Files modified:** `tests/watch_incremental.rs`
**Commit:** 495a791
**Applied fix:** 在 Phase 2 的 `write_test_log_records(&log_path, 10, 5)` 调用处添加注释，明确 `start_id=10` 必须与 Phase 1 的 `start_id=0` 不重叠，以确保生成不同的 trxid/exec_id 字段，避免重复行或行数统计不可预期

## Skipped Issues

### IN-01: `tests/watch_incremental.rs` 中 `AtomicBool` 已导入但未导入 `Ordering`

**File:** `tests/watch_incremental.rs:17`
**Reason:** skipped: 经检查文件中不需要 `Ordering`（所有 AtomicBool 仅用 `::new(true/false)` 初始化），与审查建议一致——无需 `Ordering` 时保持现状；保持导入最小化优于机械对齐其他文件的导入风格

---

### IN-02: `filter_processor.rs` 单元测试与集成测试中 `make_record` 构造重复

**File:** `src/cli/run/filter_processor.rs:136-153`, `src/cli/run/tests.rs:275-306`
**Reason:** skipped: 推迟（deferred）。提取到共享测试工具模块需要新建 `#[cfg(test)]` 公共模块并调整可见性，属于较大重构，超出本次修复范围。不影响正确性，留待后续重构 sprint 处理。

---

_Fixed: 2026-06-07T10:30:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
