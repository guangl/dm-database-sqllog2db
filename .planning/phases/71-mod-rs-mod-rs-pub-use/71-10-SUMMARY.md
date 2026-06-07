---
phase: 71-mod-rs-mod-rs-pub-use
plan: 10
subsystem: cli/watch
tags: [refactor, module-split, watch]
dependency_graph:
  requires: []
  provides: [watch/mod.rs-split]
  affects: [src/cli/watch/mod.rs, src/cli/watch/handler.rs, src/cli/watch/state.rs, src/cli/watch/debounce.rs, src/cli/watch/dirs.rs, src/cli/watch/append.rs, src/cli/watch/watcher.rs, src/cli/watch/event.rs, src/cli/watch/trigger_full.rs, src/cli/watch/trigger_incremental.rs, src/cli/watch/status.rs, src/cli/watch/tests.rs]
tech_stack:
  added: []
  patterns: [mod-declarations-only, pub-use-reexport, pub-super-visibility]
key_files:
  created:
    - src/cli/watch/state.rs
    - src/cli/watch/debounce.rs
    - src/cli/watch/dirs.rs
    - src/cli/watch/append.rs
    - src/cli/watch/watcher.rs
    - src/cli/watch/event.rs
    - src/cli/watch/trigger_full.rs
    - src/cli/watch/trigger_incremental.rs
    - src/cli/watch/status.rs
    - src/cli/watch/handler.rs
    - src/cli/watch/tests.rs
  modified:
    - src/cli/watch/mod.rs
decisions:
  - "WatchLoopState 字段升级为 pub(super) 而非保持私有，以允许兄弟模块直接访问"
  - "仅供集成测试使用的 pub use 加 #[allow(unused_imports)] 消除 bin target 下的 unused 警告"
  - "update_status_bar 放在 trigger_full.rs 中并暴露为 pub(super)，供 trigger_incremental 复用"
metrics:
  duration: "约 20 分钟"
  completed: "2026-06-07"
  tasks_completed: 3
  tasks_total: 3
  files_created: 11
  files_modified: 1
---

# Phase 71 Plan 10: watch/mod.rs 拆分为 11 个职责文件 Summary

将 `src/cli/watch/mod.rs`（998 行）按 watch 流程阶段拆分为 10 个职责文件 + `tests.rs`，`mod.rs` 缩减为 32 行极简骨架（仅含 mod 声明与 pub use）。

## 执行结果

### Task 1: 基础设施层（state + debounce + dirs + append）

创建 4 个基础设施文件，承接 watch 模块的核心数据结构与共用工具：

- `state.rs`（64 行）：`WatchLoopState` struct + `new/trigger_count/total_stats/file_offsets` 方法 + `DEBOUNCE_WINDOW/STATUS_REFRESH_INTERVAL` 两个常量，所有字段升级为 `pub(super)`
- `debounce.rs`（27 行）：`should_trigger` 防抖函数
- `dirs.rs`（54 行）：`collect_watch_dirs`（pub）+ `format_paths_display`（pub(super)）
- `append.rs`（19 行）：`force_append_for_watch_trigger`（WATCH-07/08 核心）

**Commit:** `ff7084b`

### Task 2: 流程层（watcher + event + trigger_full + trigger_incremental + status）

创建 5 个流程文件，承接 watch 流程的各阶段处理：

- `watcher.rs`（28 行）：`create_watcher` 函数（pub(super)）
- `event.rs`（49 行）：`handle_event` 按 `EventKind` 路由
- `trigger_full.rs`（96 行）：`trigger_full_file`（pub）+ `record_offset_after_trigger` + `update_status_bar`（pub(super)）
- `trigger_incremental.rs`（161 行）：`trigger_incremental`（pub）+ `resolve_incremental_offset` + `read_bytes_to_tempfile`（pub(super)）+ `run_incremental_handle_run` + `build_incremental_cfg`
- `status.rs`（81 行）：`build_progress_bar` + `render_active_status` + `refresh_active_status` + `format_elapsed_hms`（pub）+ `print_final_summary`

**Commit:** `66860cf`

### Task 3: handler.rs + tests.rs + mod.rs 骨架改写

- `handler.rs`（116 行）：`handle_watch`（pub 主入口）+ `run_watch_loop` + `maybe_refresh_status`
- `tests.rs`（390 行）：迁移全部 14 个原单元测试 + 3 个 WATCH-07/08/09 集成测试
- `mod.rs`（32 行）：改写为极简骨架，仅含 mod 声明与 pub use

**偏差修复（Rule 1）：**
1. `handler.rs` 中 `notify::RecvTimeoutError` 导入路径错误 → 修正为 `std::sync::mpsc::RecvTimeoutError`
2. 多个文件 doc 注释缺少反引号（clippy `doc_markdown` lint）→ 补全反引号
3. `mod.rs` 中仅供集成测试使用的 pub use 在 bin target 被报 unused → 添加 `#[allow(unused_imports)]`

**Commit:** `4050827`

## 质量门禁结果

- `cargo clippy --all-targets -- -D warnings`：通过
- `cargo test`：395 个单元测试 + 7 个 watch_incremental 集成测试，全部通过
- `src/cli/watch/mod.rs`：grep 无 fn/struct/impl 实现，仅 mod 声明与 pub use
- 端到端冒烟：`cargo run -- watch -c <config>` 可正常启动（无 panic）

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 修正 handler.rs 中错误的 notify 导入路径**
- **Found during:** Task 3（cargo build 失败）
- **Issue:** `use notify::RecvTimeoutError;` 应为 `use std::sync::mpsc::RecvTimeoutError;`
- **Fix:** 修正 use 路径
- **Files modified:** `src/cli/watch/handler.rs`
- **Commit:** `4050827`

**2. [Rule 1 - Bug] 修复 doc 注释 doc_markdown lint（6 处）**
- **Found during:** Task 3（cargo clippy 失败）
- **Issue:** `event.rs`/`trigger_full.rs`/`trigger_incremental.rs`/`mod.rs` 中函数名未加反引号
- **Fix:** 在相关函数名周围添加反引号
- **Files modified:** `src/cli/watch/event.rs`, `src/cli/watch/trigger_full.rs`, `src/cli/watch/trigger_incremental.rs`, `src/cli/watch/mod.rs`
- **Commit:** `4050827`

**3. [Rule 1 - Bug] 修复 mod.rs pub use 的 unused_imports 警告**
- **Found during:** Task 3（cargo clippy 失败）
- **Issue:** `trigger_full_file`/`trigger_incremental`/`collect_watch_dirs`/`WatchLoopState`/`format_elapsed_hms` 等 pub use 只在集成测试中被引用，bin target clippy 报 unused
- **Fix:** 为这些 pub use 添加 `#[allow(unused_imports)]` 注释说明用途
- **Files modified:** `src/cli/watch/mod.rs`
- **Commit:** `4050827`

## Known Stubs

无。所有函数均完整实现，无占位符或 TODO。

## Threat Flags

无新增安全相关接口。

## Self-Check: PASSED

- `src/cli/watch/state.rs` — FOUND
- `src/cli/watch/debounce.rs` — FOUND
- `src/cli/watch/dirs.rs` — FOUND
- `src/cli/watch/append.rs` — FOUND
- `src/cli/watch/watcher.rs` — FOUND
- `src/cli/watch/event.rs` — FOUND
- `src/cli/watch/trigger_full.rs` — FOUND
- `src/cli/watch/trigger_incremental.rs` — FOUND
- `src/cli/watch/status.rs` — FOUND
- `src/cli/watch/handler.rs` — FOUND
- `src/cli/watch/tests.rs` — FOUND
- Commit `ff7084b` — FOUND
- Commit `66860cf` — FOUND
- Commit `4050827` — FOUND
