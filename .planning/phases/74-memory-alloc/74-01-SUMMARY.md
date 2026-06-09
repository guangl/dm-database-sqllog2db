---
phase: 74-memory-alloc
plan: "01"
subsystem: pipeline
tags: [rust, memory-optimization, hashmap, normalizer, borrow]

requires:
  - phase: none

provides:
  - "ParamBuffer 二级 HashMap<String, HashMap<String, Arc<Vec<ParamValue>>>> 类型"
  - "compute_normalized DML 查询路径零分配 &str 查询"
  - "PARAMS insert 路径 entry().or_default() 模式"
  - "test_compute_normalized_nested_lookup_missing_statement 边界单元测试"

affects:
  - 74-memory-alloc/74-02 (MEM-02 csv exporter buf)

tech-stack:
  added: []
  patterns:
    - "HashMap<String,V>::get(&str) 零分配查询 via Borrow<str>"
    - "entry().or_default().insert() 替代 insert((k1,k2), v) 避免 clippy map_entry"

key-files:
  created: []
  modified:
    - src/pipeline/normalizer.rs
    - src/cli/run/tests.rs

key-decisions:
  - "ParamBuffer 从扁平 HashMap<(String,String),V> 改为二级 HashMap 使查询路径可用 &str 零分配"
  - "DML 热路径删除 let key = (sess_id.clone(), statement.clone()) 消除两次 String::clone"
  - "PARAMS insert 路径用 entry().or_default().insert() 覆盖语义与原 insert 一致且符合 clippy map_entry"

patterns-established:
  - "Pattern 1: 二级 HashMap 热路径查询 - buffer.get(k1.as_str())?.get(k2.as_str())?.clone()"
  - "Pattern 2: 二级 HashMap insert - buffer.entry(k1.clone()).or_default().insert(k2.clone(), v)"

requirements-completed: [MEM-01]

duration: 15min
completed: "2026-06-09"
---

# Phase 74 Plan 01: ParamBuffer 二级化消除热路径 String::clone Summary

**将 ParamBuffer 从扁平元组 key 重构为二级 HashMap，DML 查询路径改用 Borrow<str> 零分配 &str 查询（MEM-01）**

## Performance

- **Duration:** 15 min
- **Started:** 2026-06-09T02:40:00Z
- **Completed:** 2026-06-09T02:55:00Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- `ParamBuffer` 类型从 `HashMap<(String, String), Arc<Vec<ParamValue>>>` 重构为 `HashMap<String, HashMap<String, Arc<Vec<ParamValue>>>>`
- `compute_normalized` DML 查询路径消除了每条执行记录两次 `String::clone`，改用 `.get(sess_id.as_str())?.get(statement.as_str())?` 零分配查询
- PARAMS insert 路径改用 `entry().or_default().insert()` 模式，与 clippy `map_entry` lint 兼容
- 追加了 `test_compute_normalized_nested_lookup_missing_statement` 边界测试（sess_id 存在但 statement 不存在时不 panic 且返回 None）
- `cli/run/tests.rs` 中过时的 `(String, String)` 元组 key 断言更新为二级查询形式

## Task Commits

每个任务独立提交：

1. **Task 1: ParamBuffer 二级化 + 边界单元测试** - `8179893` (feat)
2. **Task 2: 修复 cli/run/tests.rs 中过时的 tuple key 断言** - `f39d96f` (fix)

## Files Created/Modified

- `src/pipeline/normalizer.rs` - ParamBuffer 类型定义、PARAMS insert、DML lookup 三处改造 + 新增边界测试
- `src/cli/run/tests.rs` - 删除旧 `buf_key` 元组变量，改用二级 HashMap 查询断言

## Decisions Made

- 选择二级 HashMap 而非 newtype wrapper：实现最简洁，编译期保证 `&str` 查询兼容性，无需引入新类型
- DML 查询链 `?` 操作符：`sess_id` 不存在或 `statement` 不存在均返回 `None`，语义清晰无 panic 风险
- 不引入任何新 crate 依赖：纯标准库 HashMap + Borrow<str> 特性，零额外依赖

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] tests.rs 编译错误阻塞 Task 1 测试运行**
- **Found during:** Task 1 GREEN 阶段验证
- **Issue:** `tests.rs:344` 的旧 `contains_key(&(String,String))` 在 ParamBuffer 类型改变后无法编译，导致 `cargo test --lib pipeline::normalizer::tests` 不可运行
- **Fix:** 提前执行 Task 2 的 tests.rs 修改（删除 `buf_key` 元组，改为二级 `.get().and_then().is_some()` 断言）
- **Files modified:** `src/cli/run/tests.rs`
- **Verification:** 编译通过，所有 409 个测试全绿
- **Committed in:** f39d96f（Task 2 提交，与计划 Task 2 内容一致）

---

**Total deviations:** 1 auto-fixed (Rule 3 阻塞性编译错误)
**Impact on plan:** Task 2 的改动在 Task 1 验证阶段提前触发，两个任务内容完全符合计划，无额外范围扩展。

## Issues Encountered

- 旧 `ParamBuffer` 类型改变后 `tests.rs` 的编译错误在 Task 1 验证时触发，属于计划预期的联动修改（Task 2 正是为此准备的），通过提前执行 Task 2 解决。

## User Setup Required

None - 无需外部服务配置。

## Next Phase Readiness

- MEM-01 完整实现：`ParamBuffer` 二级化 + 零分配查询 + 全套测试通过
- 准备进入 Plan 02（MEM-02）：`CsvExporter::new()` 的 `line_buf` 初始容量从 2048 扩展至 4096

---
*Phase: 74-memory-alloc*
*Completed: 2026-06-09*
