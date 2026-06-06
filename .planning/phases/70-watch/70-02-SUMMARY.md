---
phase: 70-watch
plan: 02
subsystem: cli-watch
tags: [rust, watch, sqlite, incremental, tempfile, offsets]

# Dependency graph
requires:
  - phase: 70-01
    provides: "offsets.rs API (ensure_offset_table / load_offsets / save_offset) + watch/mod.rs 模块结构"
provides:
  - "WatchLoopState 携带 file_offsets: HashMap<PathBuf, u64> 和 sqlite_db_url: Option<String>"
  - "pub(crate) trigger_full_file — Create 事件全量处理 + 持久化初始 offset"
  - "pub(crate) trigger_incremental — Modify(Data(Content)) 事件增量处理，Seek + NamedTempFile"
  - "pub(super) read_bytes_to_tempfile — 从 start_offset 读取字节到临时文件"
  - "handle_watch 启动时 load_offsets 实现跨重启恢复（per D-06）"
affects: [70-03]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "EventKind::Create(_) → trigger_full_file（保持用户 config，per D-10）"
    - "EventKind::Modify(Data(Content)) → trigger_incremental（强制 append=true，per D-09）"
    - "首次 Modify 无记录时记录基线 = 当前文件大小，跳过处理（per D-04）"
    - "new_size <= start_offset 快速跳过（per D-02）"
    - "read_bytes_to_tempfile：SeekFrom::Start + read_to_end + Builder::new().prefix('sqllog2db-watch-')"
    - "handle_run 返回后才调用 save_offset，避免 SqliteExporter EXCLUSIVE 锁冲突（per Pitfall 4）"
    - "path.canonicalize().unwrap_or_else(|_| ...) 规范化路径，防止相对/绝对路径不一致（per T-70-07）"
    - "所有函数体 ≤31 行，满足 CLAUDE.md ≤40 行约束（提取 4 个辅助函数）"

key-files:
  created: []
  modified:
    - "src/cli/watch/mod.rs — WatchLoopState 扩展 + trigger_full_file + trigger_incremental + 4 个单元测试"

key-decisions:
  - "WatchLoopState 改为 pub(crate) 以匹配 pub(crate) trigger_full_file/trigger_incremental 可见性"
  - "拆分 record_offset_after_trigger + update_status_bar + resolve_incremental_offset + build_incremental_cfg 辅助函数，满足 ≤40 行约束"
  - "trigger_incremental 通过 resolve_incremental_offset 返回 Option<u64>，None 表示跳过（首次见到或无新字节）"
  - "测试直接测试 read_bytes_to_tempfile（pub(super)）避免真实 handle_run 运行开销"

# Metrics
duration: 25min
completed: 2026-06-06
---

# Phase 70 Plan 02: WatchLoopState 扩展 + 增量处理实现 Summary

**WatchLoopState 扩展携带 file_offsets/sqlite_db_url，trigger_full_file 与 trigger_incremental 实现 Create/Modify 事件路由与 Seek+NamedTempFile 增量读取，handle_watch 启动时 load_offsets 跨重启恢复，20 个单元测试全部通过**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-06-06T09:00:00Z
- **Completed:** 2026-06-06T09:25:00Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- `WatchLoopState` 新增 `file_offsets: HashMap<PathBuf, u64>` 和 `sqlite_db_url: Option<String>` 字段，`new()` 接受 `init_offsets` 参数
- `handle_watch` 启动时若有 SQLite 导出器，调用 `ensure_offset_table` + `load_offsets` 预填 state（per D-06，跨重启恢复）
- `handle_event` 重构为接受 `&mut WatchLoopState` 单一参数，按 `EventKind::Create` / `EventKind::Modify(Data(Content))` 分支路由
- `trigger_full_file`（pub(crate)）：全量处理 + handle_run 返回后持久化文件大小为 offset（per Pitfall 4，锁已释放）
- `trigger_incremental`（pub(crate)）：首次见到记录基线跳过（per D-04）、无新字节快速跳过（per D-02）、Seek+NamedTempFile 增量读取（per D-01），强制 append=true（per D-09）
- `read_bytes_to_tempfile`（pub(super)）：可独立测试的辅助函数，使用 `Builder::new().prefix("sqllog2db-watch-").suffix(".log").tempfile()`
- 删除旧 `process_log_path` 和 TODO 注释（Phase 70 通过 file_offsets 解决了 WR-03 问题）
- 提取 4 个辅助函数满足 CLAUDE.md ≤40 行约束：`record_offset_after_trigger`、`update_status_bar`、`resolve_incremental_offset`、`build_incremental_cfg`
- 新增 4 个单元测试（Task 2 TDD GREEN）

## Task Commits

1. **Task 1: 扩展 WatchLoopState + 重构 handle_event + 加载启动 offsets** - `cf27bff` (feat)
2. **Task 2: trigger_incremental + read_bytes_to_tempfile 单元测试** - `5c756a0` (test)
3. **Task 2: 提取辅助函数确保函数体 ≤40 行** - `21cbe9b` (refactor)

## Files Created/Modified

- `src/cli/watch/mod.rs` — WatchLoopState 扩展 + 新增函数 + 重构 + 4 个新单元测试（共 20 个 watch 模块测试全部通过）

## Decisions Made

- `WatchLoopState` 改为 `pub(crate)` 以允许 `pub(crate)` 函数在参数中使用该类型（rust 可见性规则）
- `trigger_incremental` 通过 `resolve_incremental_offset` 返回 `Option<u64>`，`None` 表示应跳过，避免嵌套 if-else
- 测试直接测试 `read_bytes_to_tempfile`（pub(super)）而非 `trigger_incremental` 全路径，避免真实 `handle_run` 运行开销（测试 I/O 逻辑与状态逻辑分离）
- `build_incremental_cfg` 提取为独立辅助，D-09 `append=true` + `overwrite=false` 的强制逻辑有单一位置

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 修复多处 doc comment 缺少 backtick 导致 clippy `-D warnings` 报错**

- **Found during:** Task 1 提交时的 pre-commit hook clippy 检查
- **Issue:** 新增函数 doc comments 中 `EventKind`、`trigger_full_file`、`start_offset`、`new_size`、`NamedTempFile`、`handle_run`、`tmp_file` 等标识符未加 backtick
- **Fix:** 统一加上 backtick
- **Files modified:** src/cli/watch/mod.rs
- **Committed in:** cf27bff、5c756a0

**2. [Rule 1 - Bug] 修复 clippy let-else 与 if-let 重写建议**

- **Found during:** Task 1 提交时的 clippy 检查
- **Issue:** `match state.file_offsets.get(...)` 块在 None 分支有 early return，clippy 建议改为 `let..else`；`match metadata` 同理
- **Fix:** 改为 `let Ok(metadata) = ... else { ... }` 和 `let Some(&start_offset) = ... else { ... }` 形式
- **Files modified:** src/cli/watch/mod.rs
- **Committed in:** cf27bff

**3. [Rule 1 - Bug] 修复 trigger_incremental u64 as usize 直接 cast 触发 clippy 截断警告**

- **Found during:** Task 2 测试提交时的 clippy 检查
- **Issue:** `&content[start_offset as usize..]` 触发 `clippy::cast_possible_truncation`
- **Fix:** 改为 `usize::try_from(start_offset).expect("offset fits in usize")`
- **Files modified:** src/cli/watch/mod.rs
- **Committed in:** 5c756a0

**4. [Rule 2 - Missing Critical] 提取辅助函数满足 CLAUDE.md ≤40 行约束**

- **Found during:** Task 2 完成后函数行数验证
- **Issue:** `trigger_full_file`（46 行）、`trigger_incremental`（50 行）、`run_incremental_handle_run`（49 行）超过 40 行
- **Fix:** 提取 `record_offset_after_trigger`、`update_status_bar`、`resolve_incremental_offset`、`build_incremental_cfg` 4 个辅助函数，全部函数体降至 ≤31 行
- **Files modified:** src/cli/watch/mod.rs
- **Committed in:** 21cbe9b

---

**Total deviations:** 4 auto-fixed（3 clippy 质量门禁，1 CLAUDE.md ≤40 行拆分）
**Impact on plan:** 均为必须修复的质量约束，无范围扩展。

## Verification Results

```
cargo clippy --all-targets -- -D warnings    # PASSED
cargo build --release                         # PASSED
cargo test --lib cli::watch::                # 20 passed; 0 failed
grep -c 'pub(crate) fn trigger_full_file|...'  # 2 (correct)
grep 'append = true'                          # line 458 in sqlite_cfg (D-09)
grep 'new_size <= start_offset'               # line 391 (D-02)
```

## Known Stubs

None — `trigger_full_file` 和 `trigger_incremental` 均调用真实的 `handle_run`，未使用 mock 或占位。

## Threat Flags

无新增威胁面（threat model 已在 PLAN.md 中覆盖 T-70-04 至 T-70-07）。

---

*Phase: 70-watch*
*Completed: 2026-06-06*
