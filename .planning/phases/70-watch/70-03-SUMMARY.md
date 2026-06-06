---
phase: 70-watch
plan: 03
subsystem: cli-watch
tags: [rust, watch, sqlite, integration-test, tempfile, offsets, tdd]

# Dependency graph
requires:
  - phase: 70-01
    provides: "offsets.rs API (ensure_offset_table / load_offsets / save_offset)"
  - phase: 70-02
    provides: "trigger_full_file + trigger_incremental + WatchLoopState pub(crate)"
provides:
  - "tests/watch_incremental.rs: WATCH-03 + WATCH-04 集成测试套件（4 个 #[test]，257 行）"
  - "WatchLoopState: pub struct + pub new() + pub getters (trigger_count/total_stats/file_offsets)"
  - "trigger_full_file / trigger_incremental: pub(crate) -> pub（支持集成测试直接调用）"
  - "record_offset_after_trigger: ensure_offset_table 防御性调用（绕过 handle_watch 启动路径时的正确性保障）"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "TempDir 每测试独立隔离（per T-70-08），cargo test 并行无路径冲突"
    - "ProgressBar::new_spinner() + ProgressDrawTarget::hidden() 避免测试输出污染"
    - "OpenOptions::append(true) 实现不重写文件的增量写入 helper"
    - "rusqlite::Connection 直接读取 _watch_offsets 模拟重启后 load_offsets（pub(super) 不可见时的测试替代）"
    - "record_offset_after_trigger 调用 ensure_offset_table 确保绕过 handle_watch 时 _watch_offsets 表存在"

key-files:
  created:
    - "tests/watch_incremental.rs — WATCH-03/WATCH-04 集成测试套件，257 行，4 个 #[test]"
  modified:
    - "src/cli/watch/mod.rs — WatchLoopState pub化 + 3个getter + pub trigger_* + ensure_offset_table 防御性调用"

key-decisions:
  - "WatchLoopState 升级为 pub struct + pub new() 以支持集成测试直接构造（per D-14）"
  - "record_offset_after_trigger 中添加 ensure_offset_table 调用，使 trigger_full_file 在绕过 handle_watch 启动逻辑时也能正确工作"
  - "file_offsets() getter 使用 #[allow(dead_code)] 因 bin crate 中不使用但集成测试需要"
  - "WATCH-04 重启模拟通过直接查询 _watch_offsets 表实现（load_offsets 是 pub(super) 不可见）"

requirements-completed:
  - WATCH-03
  - WATCH-04

# Metrics
duration: 20min
completed: 2026-06-06
---

# Phase 70 Plan 03: WATCH-03/WATCH-04 集成测试 Summary

**tests/watch_incremental.rs（257 行，4 个 #[test]）提供 WATCH-03 追加不重复与 WATCH-04 重启 offset 恢复的端到端集成测试证据；WatchLoopState 升级为 pub 支持直接调用；record_offset_after_trigger 添加 ensure_offset_table 防御性调用**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-06-06T09:26:00Z
- **Completed:** 2026-06-06T09:46:00Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- 将 `WatchLoopState`、`trigger_full_file`、`trigger_incremental` 全部升级为 `pub`，暴露 3 个 getter（`trigger_count`、`total_stats`、`file_offsets`）支持集成测试直接调用
- 新建 `tests/watch_incremental.rs`（257 行）：4 个 helpers（`write_test_log_records`、`build_sqlite_config`、`count_rows`、`build_pb`）+ 4 个 `#[test]`
- **WATCH-03 核心验证**：`test_watch_03_incremental_appends_only_new_rows` — 写 10 条→全文触发→追加 5 条→增量触发→SQLite count = 15（不重复）
- **WATCH-04 核心验证**：`test_watch_04_offset_persists_across_restart` — 写 10 条→触发→销毁 state→从 `_watch_offsets` 恢复 offset→追加 7 条→增量触发→SQLite count = 17（不重复）
- **D-02 验证**：`test_watch_03_no_new_bytes_skips` — 无新字节时 `trigger_incremental` 不增加 `trigger_count`
- 修复 `record_offset_after_trigger` 中 `save_offset` 调用前缺少 `ensure_offset_table` 的正确性 bug（Rule 2 auto-fix）

## Task Commits

1. **Task 1: pub-ify + helpers + smoke test + Rule 2 fix** - `1500c07` (feat)

## Files Created/Modified

- `tests/watch_incremental.rs` — 集成测试套件（257 行，4 个 #[test]）
- `src/cli/watch/mod.rs` — WatchLoopState pub化 + getter + trigger_* pub + ensure_offset_table 防御性调用

## Decisions Made

- `WatchLoopState` 改为 `pub struct` 加 `#[derive(Debug)]`（pub 结构体需要 Debug）
- `file_offsets()` getter 加 `#[allow(dead_code)]` 因 bin crate 中不使用，但测试中是关键断言
- WATCH-04 中重启模拟通过直接查询 SQLite `_watch_offsets` 表实现，避免调用 `pub(super) load_offsets`
- `record_offset_after_trigger` 添加 `ensure_offset_table` 幂等调用，修复直接调用 `trigger_full_file`（绕过 `handle_watch` 启动）时的正确性问题

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] trigger_full_file + trigger_incremental 中 clippy #[must_use] + #[derive(Debug)] 警告**

- **Found during:** Task 1 clippy 检查
- **Issue:** pub 化 `WatchLoopState` 后，clippy 要求 `#[derive(Debug)]` 和 getter 方法加 `#[must_use]`；`new()` 方法也需要 `#[must_use]`；doc comment 中 `SQLite` 缺 backtick
- **Fix:** 添加 `#[derive(Debug)]`、`#[must_use]` 注解，修正 doc comment
- **Committed in:** 1500c07

**2. [Rule 2 - Missing Critical] record_offset_after_trigger 缺少 ensure_offset_table 调用**

- **Found during:** Task 1 集成测试运行（WATCH-04 测试失败）
- **Issue:** `test_watch_04_offset_persists_across_restart` 调用 `trigger_full_file` 后，`_watch_offsets` 表不存在（因为测试绕过了 `handle_watch` 的启动逻辑），导致后续 `stmt.prepare("SELECT path, byte_offset FROM _watch_offsets")` 报 "no such table"
- **Fix:** 在 `record_offset_after_trigger` 中添加 `ensure_offset_table` 幂等调用（失败只 warn 不中断，per D-07）
- **Files modified:** src/cli/watch/mod.rs
- **Committed in:** 1500c07

**3. [Rule 1 - Bug] 测试文件中多处 doc comment 缺 backtick + let-else + redundant closure**

- **Found during:** Task 1 clippy 检查（测试文件）
- **Issue:** `DaMeng`、`SQLite`、`WatchLoopState` 未加 backtick；`match Connection::open` 应改为 `let...else`；`filter_map(|r| r.ok())` 应改为 `filter_map(std::result::Result::ok)`；`i64 as u64` 触发 cast_sign_loss
- **Fix:** 逐一修复 doc comment + 改写 let-else + 使用方法引用 + 添加 `#[allow(clippy::cast_sign_loss)]` 注释
- **Committed in:** 1500c07

---

**Total deviations:** 3 auto-fixed（2 clippy 质量门禁，1 正确性 bug）
**Impact on plan:** 均为必须修复的正确性/质量约束，无范围扩展。

## Verification Results

```
cargo test --test watch_incremental   # 4 passed; 0 failed
cargo test                             # 376/407 unit + 4 integration passed; 0 failed
cargo build --release                  # PASSED
cargo clippy --all-targets -- -D warnings  # PASSED
cargo fmt --check                      # PASSED
grep test_watch_03_incremental_appends_only_new_rows tests/watch_incremental.rs  # found
grep test_watch_04_offset_persists_across_restart tests/watch_incremental.rs     # found
wc -l tests/watch_incremental.rs       # 257 lines (>= 100)
```

## Known Stubs

None — 全部测试均使用真实 SQLite DB（`TempDir` + `rusqlite::Connection`），无 mock 或占位数据。

## Threat Flags

无新增威胁面 — 测试文件仅在 `[dev-dependencies]` 编译路径下运行，不影响生产二进制的攻击面。

## Self-Check

- [x] `tests/watch_incremental.rs` exists (257 lines)
- [x] Commit `1500c07` exists
- [x] 4 `#[test]` functions: smoke_test_helpers_compile, test_watch_03_incremental_appends_only_new_rows, test_watch_04_offset_persists_across_restart, test_watch_03_no_new_bytes_skips
- [x] All tests pass: `cargo test --test watch_incremental` → 4 passed
- [x] No regressions: full `cargo test` → all passed

## Self-Check: PASSED

---
*Phase: 70-watch*
*Completed: 2026-06-06*
