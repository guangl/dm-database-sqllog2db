---
phase: 71-mod-rs-mod-rs-pub-use
plan: "08"
subsystem: exporter/sqlite
tags: [refactor, module-split, sqlite]
dependency_graph:
  requires: []
  provides:
    - src/exporter/sqlite/exporter.rs
    - src/exporter/sqlite/impls.rs
    - src/exporter/sqlite/pragma.rs
  affects:
    - src/exporter/sqlite/mod.rs
tech_stack:
  added: []
  patterns:
    - mod-rs-pub-use：mod.rs 仅声明 + pub use，实现拆到独立文件
key_files:
  created:
    - src/exporter/sqlite/exporter.rs
    - src/exporter/sqlite/impls.rs
    - src/exporter/sqlite/pragma.rs
  modified:
    - src/exporter/sqlite/mod.rs
    - Cargo.toml
    - Cargo.lock
decisions:
  - "normalize/field_mask/ordered_indices 三个字段从 pub(super) 升级为 pub(crate)，因为 src/exporter/mod.rs 的 ExporterManager::from_config 需要直接访问"
  - "mod.rs 加 #[cfg(test)] pub(super) use super::Exporter 让 tests.rs 的 use super::* 能访问 Exporter trait 方法"
  - "Cargo.toml 中 rusqlite 0.40→0.39，Cargo.lock 同步，解决 libsqlite3-sys 0.38 与 Rust 1.94 的 cfg_select 不兼容问题"
metrics:
  duration: "约 15 分钟"
  completed_date: "2026-06-07"
  tasks_completed: 1
  files_changed: 6
---

# Phase 71 Plan 08: exporter/sqlite/mod.rs 拆分 Summary

**一句话概要：** 将 249 行的 sqlite/mod.rs 按"PRAGMA 配置 / struct+工具方法 / trait 行为"三个维度拆分为 pragma.rs + exporter.rs + impls.rs，mod.rs 缩减到 15 行仅含 mod 声明和 pub use 重导出。

## 任务执行

### Task 1: 拆分 sqlite/mod.rs 到 exporter.rs + impls.rs + pragma.rs

**状态：** 完成  
**提交：** be1b7ca

**执行内容：**
- 新建 `src/exporter/sqlite/pragma.rs`：包含 `pub(super) fn initialize_pragmas`
- 新建 `src/exporter/sqlite/exporter.rs`：包含 `pub(crate) struct SqliteExporter` + 所有构造方法（`new`, `from_config`）+ 内部工具方法（`db_err`, `conn_ref`, `set_wal_mode`, `batch_commit_if_needed`, `handle_delete_clear_result`, `prepare_target_table`）+ `Debug impl`
- 新建 `src/exporter/sqlite/impls.rs`：包含 `impl Exporter for SqliteExporter` 完整块
- 改写 `src/exporter/sqlite/mod.rs`：精简为 15 行（注释 + mod 声明 + pub use）
- `sql_builder.rs` / `write.rs` / `tests.rs` 内容完全不变

**验证结果：**
- `cargo clippy --all-targets -- -D warnings`：通过（exit 0）
- `cargo test`：335 tests 全部通过（0 failed）
- `src/exporter/sqlite/mod.rs` 过滤注释/空行后无 fn/struct/impl/trait/enum
- `sql_builder.rs` / `write.rs` / `tests.rs` 三个文件 diff 为 0（内容不变）

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 字段可见性升级：normalize/field_mask/ordered_indices 改为 pub(crate)**
- **Found during:** Task 1 编译阶段
- **Issue:** 计划指定这三个字段为 `pub(super)` 但 `src/exporter/mod.rs` 的 `ExporterManager::from_config` 直接访问它们；`pub(super)` 在 exporter.rs（sqlite 子模块内）的 super 是 sqlite 模块，不是 exporter 模块
- **Fix:** 将这三个字段改为 `pub(crate)` 允许 crate 内任意模块访问
- **Files modified:** src/exporter/sqlite/exporter.rs
- **Commit:** be1b7ca

**2. [Rule 1 - Bug] 添加 #[cfg(test)] pub(super) use super::Exporter 到 mod.rs**
- **Found during:** Task 1 clippy 阶段
- **Issue:** tests.rs 使用 `use super::*;` 需要 Exporter trait 在 scope 中才能调用 trait 方法；拆分后 trait impl 在独立的 impls.rs 中，`super::*` 不再自动引入 Exporter
- **Fix:** 在 mod.rs 加入 `#[cfg(test)] pub(super) use super::Exporter;`，仅在测试编译时暴露，避免非测试时 unused import 警告
- **Files modified:** src/exporter/sqlite/mod.rs
- **Commit:** be1b7ca

**3. [Rule 3 - Blocking] rusqlite 版本降级 0.40→0.39**
- **Found during:** Task 1 cargo build 阶段
- **Issue:** worktree 基于 `c24f56f`（rusqlite 0.40.0 bump），使用 libsqlite3-sys 0.38.0，该版本使用 `cfg_select!` 宏（Rust unstable 特性 #115585），与 Rust 1.94 stable 不兼容
- **Fix:** 将 Cargo.toml 中 rusqlite 改回 0.39.0，复制主仓库 Cargo.lock 到 worktree 使版本锁定一致
- **Files modified:** Cargo.toml, Cargo.lock
- **Commit:** be1b7ca

## Known Stubs

无。所有实现均为真实逻辑，无占位符。

## Threat Flags

无新增安全敏感 surface。

## Self-Check: PASSED

- src/exporter/sqlite/pragma.rs: FOUND
- src/exporter/sqlite/exporter.rs: FOUND
- src/exporter/sqlite/impls.rs: FOUND
- src/exporter/sqlite/mod.rs: FOUND (≤15 行，仅声明)
- Commit be1b7ca: FOUND
- cargo test 335 passed: VERIFIED
- cargo clippy -D warnings: VERIFIED
- sql_builder.rs/write.rs/tests.rs 内容不变: VERIFIED (0 diff lines)
