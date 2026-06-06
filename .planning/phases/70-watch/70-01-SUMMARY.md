---
phase: 70-watch
plan: 01
subsystem: database
tags: [rusqlite, sqlite, tempfile, watch, offsets]

# Dependency graph
requires:
  - phase: 69-watch
    provides: "src/cli/watch.rs Phase 69 watch 实现（handle_watch, collect_watch_dirs 等）"
provides:
  - "tempfile 升级为生产依赖（[dependencies]），release 构建可使用"
  - "src/cli/watch/mod.rs（原 watch.rs 通过 git mv 迁移，历史完整保留）"
  - "src/cli/watch/offsets.rs：ensure_offset_table / load_offsets / save_offset 三函数"
  - "_watch_offsets SQLite 辅助表 DDL + 独立连接隔离（per D-05）"
affects: [70-02, 70-03]

# Tech tracking
tech-stack:
  added: ["tempfile = 3.27.0（从 dev-dependencies 提升至 dependencies）"]
  patterns:
    - "独立 rusqlite::Connection per 调用，避免 SqliteExporter EXCLUSIVE 锁冲突（per D-05）"
    - "save_offset 返回 ()，失败 log::warn! 不中断主流程（per D-07）"
    - "load_offsets 负值过滤（i64 >= 0 才 as u64），防止极大值 u64（per T-70-02/Pitfall 2）"
    - "首次运行表不存在静默返回空 map，不报错（per 首次运行正常状态）"

key-files:
  created:
    - "src/cli/watch/offsets.rs — _watch_offsets 辅助表读写，170 行含 5 个单元测试"
  modified:
    - "Cargo.toml — tempfile 从 dev-dependencies 移至 dependencies"
    - "src/cli/watch/mod.rs — 原 watch.rs git mv 迁移 + pub(super) mod offsets; 声明"

key-decisions:
  - "使用独立 Connection::open 而非共享连接，与 SqliteExporter EXCLUSIVE 模式隔离（per D-05）"
  - "save_offset 签名返回 ()，失败仅 warn 不 propagate，保证 watch 不因持久化失败中断（per D-07）"
  - "load_offsets 负值过滤防御性设计，对应 T-70-02 威胁模型"
  - "#[allow(dead_code)] 标注三函数——Wave 1 将调用，此阶段 pub(super) 无外部调用者"

patterns-established:
  - "offsets API 模式：每函数独立 open connection，不复用"
  - "cast 安全性注释：cast_sign_loss/cast_possible_wrap 附注业务语义说明"

requirements-completed:
  - WATCH-04

# Metrics
duration: 15min
completed: 2026-06-06
---

# Phase 70 Plan 01: Watch 基础设施 Summary

**tempfile 提升为生产依赖 + watch.rs 迁移为 watch/mod.rs + offsets.rs 提供 `_watch_offsets` SQLite 辅助表 DDL 与读写（ensure/load/save 三函数，5 个单元测试全部通过）**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-06-06T08:45:00Z
- **Completed:** 2026-06-06T08:57:52Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- 将 tempfile = "3.27.0" 从 `[dev-dependencies]` 提升至 `[dependencies]`，确保 `cargo build --release` 可链接（per D-11/RESEARCH critical finding）
- 通过 `git mv` 将 `src/cli/watch.rs` 迁移为 `src/cli/watch/mod.rs`，git 历史完整保留（`git log --follow` 可追踪）
- 新建 `src/cli/watch/offsets.rs`（170 行）实现 `_watch_offsets` 辅助表的 DDL 与读写，满足跨重启字节偏移持久化需求

## Task Commits

1. **Task 1: 提升 tempfile 为生产依赖并迁移 watch.rs → watch/mod.rs** - `5e7630f` (chore)
2. **Task 2: 实现 offsets.rs 三函数 + 5 个单元测试（TDD GREEN）** - `c6a35dd` (feat)

## Files Created/Modified

- `Cargo.toml` - 将 tempfile 从 dev-dependencies 移至 dependencies
- `src/cli/watch/mod.rs` - 原 watch.rs 通过 git mv 迁移，顶部新增 `pub(super) mod offsets;`
- `src/cli/watch/offsets.rs` - `_watch_offsets` 辅助表读写（170 行，含 5 个单元测试）

## Decisions Made

- 使用独立 `Connection::open` per 函数调用，与 `SqliteExporter` 的 `PRAGMA locking_mode = EXCLUSIVE` 模式隔离（per D-05）——如果复用连接会与 exporter 竞争锁
- `save_offset` 签名返回 `()`，失败仅 `log::warn!` 不 propagate——保证 watch 循环不会因为持久化失败而中断（per D-07）
- `load_offsets` 对 i64 负值做过滤（`if offset >= 0`），防止 `as u64` 产生极大值跳过所有 Modify 事件（per T-70-02/Pitfall 2）
- `#[allow(dead_code)]` 标注三个 `pub(super)` 函数——Wave 1 plan 02/03 才会在 mod.rs 中调用，此阶段无外部调用者

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] 修复 doc comment 中缺少 backticks 导致 clippy 报错**

- **Found during:** Task 1（提交时 cargo clippy 检查）
- **Issue:** offsets.rs doc comment `//! Phase 70: _watch_offsets 辅助表...` 中 `_watch_offsets` 和 `SqliteExporter` 未加 backticks，clippy `doc_markdown` 报 error
- **Fix:** 改为 `` `_watch_offsets` `` 和 `` `SqliteExporter` ``
- **Files modified:** src/cli/watch/offsets.rs
- **Verification:** `cargo clippy --all-targets -- -D warnings` 通过
- **Committed in:** 5e7630f（Task 1 commit）

**2. [Rule 1 - Bug] 修复 clippy cast_sign_loss / cast_possible_wrap 警告**

- **Found during:** Task 2（实现 offsets.rs 后 clippy 检查）
- **Issue:** `offset as u64`（从 i64 转）和 `offset as i64`（从 u64 转）触发 `-D warnings` 下的 cast 错误
- **Fix:** 添加 `#[allow(clippy::cast_sign_loss)]`（已有 `>= 0` 保证）和 `#[allow(clippy::cast_possible_wrap)]`（附注业务语义），并使用 `let...else` 重写两处 match 块
- **Files modified:** src/cli/watch/offsets.rs
- **Verification:** `cargo clippy --all-targets -- -D warnings` 通过
- **Committed in:** c6a35dd（Task 2 commit）

---

**Total deviations:** 2 auto-fixed（1 missing critical doc，1 clippy cast/lint）
**Impact on plan:** 均为编译质量门禁必须修复，无范围扩展。

## Issues Encountered

- `cargo fmt` 在占位 offsets.rs 尚未创建时 Task 1 提交失败——解决方案：先创建空占位文件再提交，后续 Task 2 填充实现（正常的顺序依赖处理）

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `offsets.rs` API 已就绪，Wave 1 的 Plan 02（增量处理）可直接调用 `ensure_offset_table` / `load_offsets` / `save_offset`
- `watch/mod.rs` 模块结构已建立，Plan 02/03 在同级新增功能无需修改 mod.rs 结构
- 全部 370 个单元测试通过，0 回归

---
*Phase: 70-watch*
*Completed: 2026-06-06*
