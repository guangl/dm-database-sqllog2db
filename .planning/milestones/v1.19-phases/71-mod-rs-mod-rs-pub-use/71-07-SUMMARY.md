---
phase: 71-mod-rs-mod-rs-pub-use
plan: "07"
subsystem: exporter/csv
tags: [refactor, module-structure, visibility]
dependency_graph:
  requires: []
  provides: [exporter.rs, impls.rs]
  affects: [src/exporter/csv/mod.rs, src/exporter/csv/exporter.rs, src/exporter/csv/impls.rs, src/exporter/csv/writer.rs]
tech_stack:
  added: []
  patterns: [module-split, pub-in-path-visibility]
key_files:
  created:
    - src/exporter/csv/exporter.rs
    - src/exporter/csv/impls.rs
  modified:
    - src/exporter/csv/mod.rs
    - src/exporter/csv/writer.rs
    - Cargo.toml
    - Cargo.lock
decisions:
  - "writer.rs 中 write_record/write_record_preparsed 可见性由 pub(super) 改为 pub(in crate::exporter::csv)，使兄弟模块 impls.rs 可访问"
  - "Drop for CsvExporter 放在 impls.rs 而非 exporter.rs，因为 Drop 需要调用 Exporter::finalize"
  - "提取 open_for_write 辅助函数到 exporter.rs，将 initialize 的文件打开逻辑与 impls.rs 的写入逻辑分离"
  - "Cargo.toml rusqlite 降回 0.39.0，同步主仓库版本，修复 libsqlite3-sys 0.38.x 编译失败"
metrics:
  duration: "~25 minutes"
  completed: 2026-06-07
  tasks_completed: 1
  files_changed: 6
---

# Phase 71 Plan 07: 拆分 csv/mod.rs → exporter.rs + impls.rs 总结

将 src/exporter/csv/mod.rs（243 行）按职责拆分为：CsvExporter struct/构造/Drop 在 exporter.rs，Exporter trait 实现在 impls.rs，mod.rs 简化为仅含 mod 声明与 pub use（11 行）。

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | 拆分 csv/mod.rs 到 exporter.rs + impls.rs | f431dbc | mod.rs, exporter.rs(new), impls.rs(new), writer.rs |

## What Was Built

### 文件结构

**src/exporter/csv/mod.rs（11 行）**：
- 仅含模块声明（`pub(crate) mod writer`、`mod exporter`、`mod impls`、`#[cfg(test)] mod tests`）
- 仅含重导出（`pub use exporter::CsvExporter`）
- 无任何 fn/struct/impl/enum/trait 定义

**src/exporter/csv/exporter.rs（新建）**：
- `pub(super) enum WriteMode { Truncate, Append }`
- `pub struct CsvExporter { ... }`（所有字段保持原可见性）
- `impl std::fmt::Debug for CsvExporter`
- `impl CsvExporter { new, from_config, build_header }`
- `pub(super) fn writer_ref(...)` — 返回 `&mut BufWriter<File>` 或错误
- `pub(super) fn open_for_write(...)` — 封装文件打开逻辑（含 ensure_parent_dir）

**src/exporter/csv/impls.rs（新建）**：
- `impl Exporter for CsvExporter`（initialize/export/export_one_normalized/export_one_preparsed/finalize/stats_snapshot）
- `impl Drop for CsvExporter`（调用 self.finalize()）

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] writer.rs 可见性调整**
- **Found during:** Task 1（编译阶段）
- **Issue:** 计划假设 `impls.rs` 可通过 `use super::writer::{write_record, write_record_preparsed}` 访问，但这两个函数在 writer.rs 中是 `pub(super)`（对 `csv` 父模块可见），兄弟子模块 `impls.rs` 无法访问
- **Fix:** 将 writer.rs 中 write_record/write_record_preparsed 可见性从 `pub(super)` 改为 `pub(in crate::exporter::csv)`，使整个 csv 模块（含子模块）均可访问
- **Files modified:** src/exporter/csv/writer.rs
- **Commit:** f431dbc

**2. [Rule 3 - Blocking] Cargo.toml rusqlite 版本与主仓库不一致**
- **Found during:** Task 1（构建环境检查阶段）
- **Issue:** 工作树分支基于 `c24f56f`（rusqlite 0.40.0），但 libsqlite3-sys 0.38.x 在 rustc 1.94 上编译失败（cfg_select 不稳定特性）；主仓库 HEAD 使用 rusqlite 0.39.0（libsqlite3-sys 0.37.0）
- **Fix:** 降回 Cargo.toml 中 rusqlite 到 0.39.0，同步主仓库 Cargo.lock
- **Files modified:** Cargo.toml, Cargo.lock
- **Commit:** f431dbc

### 计划调整

- `Drop for CsvExporter` 放在 `impls.rs` 而非 `exporter.rs`：`Drop` 调用 `self.finalize()`，而 `finalize` 是 `Exporter` trait 的方法；将 `Drop` 与 `impl Exporter` 放在同一文件避免循环依赖问题

## Verification

- `cargo clippy --all-targets -- -D warnings`: 通过（无警告）
- `cargo test`: 通过（335 + 366 + 3 + 69 + 1 = 774 测试，0 失败）
- `src/exporter/csv/mod.rs` grep 检查：仅含声明，无 fn/struct/impl/enum
- `src/exporter/csv/writer.rs` 功能不变（仅可见性调整）
- `src/exporter/csv/tests.rs` 内容不变（use super::CsvExporter 路径无需修改）
- CsvExporter 在 `crate::exporter::csv::CsvExporter` 和 `crate::exporter::CsvExporter` 两路径均可达

## Known Stubs

无

## Threat Flags

无新增安全相关 surface

## Self-Check: PASSED

- src/exporter/csv/mod.rs: FOUND
- src/exporter/csv/exporter.rs: FOUND
- src/exporter/csv/impls.rs: FOUND
- Commit f431dbc: FOUND
