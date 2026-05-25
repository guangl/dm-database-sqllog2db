---
phase: 45-ci
plan: 01
subsystem: core
tags: [sqlite, parallel, rayon, wal, performance]

# Dependency graph
requires: []
provides:
  - SQLite 多文件并行解析路径 (src/cli/run/sqlite_parallel.rs)
  - WAL 模式切换 API (SqliteExporter::set_wal_mode)
affects: [benchmark-path-unchanged, csv-parallel-path-unchanged]

# Tech tracking
tech-stack:
  added: []
  patterns: [collect-merge-write, rayon-threadpool, wal-mode-per-parallel-path]

key-files:
  created:
    - src/cli/run/sqlite_parallel.rs
  modified:
    - src/cli/run/mod.rs
    - src/exporter/sqlite/mod.rs
    - src/exporter/mod.rs
    - src/cli/run/tests.rs

key-decisions:
  - "collect-merge-write 模式而非 channel：每线程收集 Vec，主线程顺序写入，实现简单无锁"
  - "WAL 模式局部启用：set_wal_mode 在 initialize() 之后调用，不修改 initialize_pragmas (OFF+OFF) 以保留 benchmark 基线可比性"
  - "set_wal_mode 需先 COMMIT、PRAGMA locking_mode=NORMAL、journal_mode=WAL、synchronous=NORMAL 再 BEGIN TRANSACTION（EXCLUSIVE 模式与 WAL 不兼容，且不能在事务中切换 journal_mode）"
  - "collect_log_file 中 PARAMS 被过滤的 else 分支仍调用 compute_normalized 更新 params_buf（mirror processor.rs 第 134-143 行）"

patterns-established:
  - "Pattern: parallel path = parallel_collect（rayon）+ merge_and_write（主线程顺序写入）"
  - "Pattern: WAL 模式切换 = COMMIT + locking_mode=NORMAL + journal_mode=WAL + BEGIN"

requirements-completed: [PERF-03]

# Metrics
duration: ~2h（含 worktree 失败重试 + clippy/WAL 修复）
completed: 2026-05-25
---

# Phase 45 Plan 01: SQLite 并行解析路径 Summary

**新建 `sqlite_parallel.rs` 实现 rayon 多文件并行解析 + WAL 模式，扩展 `mod.rs` 路由，追加 `test_sqlite_parallel_matches_sequential` 正确性验证。实现 PERF-03。**

## Performance

- **Duration:** ~2h
- **Completed:** 2026-05-25
- **Tasks:** 3
- **Files modified:** 5（1 新建 + 4 修改）

## Accomplishments

- 新建 `src/cli/run/sqlite_parallel.rs`（209 行，4 个函数均 ≤40 行）：
  - `collect_log_file`：线程内解析单文件，含 PARAMS 双分支 params_buf 更新
  - `process_record`：record 处理逻辑，镜像 processor.rs 行为
  - `parallel_collect`：rayon ThreadPool 并行收集
  - `merge_and_write`：主线程顺序写入 SQLite（WAL 模式）
  - `process_sqlite_parallel`：对外入口，签名与 process_csv_parallel 对称
- `src/exporter/sqlite/mod.rs`：`set_wal_mode` 方法（COMMIT + locking_mode=NORMAL + WAL + BEGIN），`initialize_pragmas` OFF+OFF 不变
- `src/exporter/mod.rs`：`set_sqlite_wal_mode` 包装（非 SQLite 时 no-op）
- `src/cli/run/mod.rs`：`use_csv_parallel` / `use_sqlite_parallel` 双路由分支
- `src/cli/run/tests.rs`：`test_sqlite_parallel_matches_sequential`（含 PARAMS + normalized_sql 断言）

## Task Commits

1. **Task 1+2+3 (合并提交)** - `7f58ef3` (feat(45-01))

## Key Fix: WAL Mode Transaction Conflict

`initialize()` 内部执行 `BEGIN TRANSACTION`，导致 `set_wal_mode()` 无法在事务中切换 journal_mode。
同时 `PRAGMA locking_mode = EXCLUSIVE` 与 WAL 不兼容。

修复：`set_wal_mode()` 执行 `COMMIT; PRAGMA locking_mode=NORMAL; PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; BEGIN TRANSACTION;`

## Deviations from Plan

- 两个 worktree executor agent 因达到用量限制失败，改为 orchestrator 直接执行
- `set_wal_mode` 实现比 Plan 更复杂（需处理 EXCLUSIVE + 事务冲突），但不违背设计意图

## Next Phase Readiness

- PERF-03 达成：多文件 SQLite 导出自动走并行路径，test_sqlite_parallel_matches_sequential 通过
- benchmark 路径完全隔离（initialize_pragmas OFF+OFF 未动）

---
*Phase: 45-ci*
*Completed: 2026-05-25*
