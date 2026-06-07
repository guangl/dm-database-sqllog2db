---
phase: 71-mod-rs-mod-rs-pub-use
plan: "06"
subsystem: exporter
tags: [refactor, module-split, visibility]
dependency_graph:
  requires: []
  provides: [exporter/api.rs, exporter/kind.rs, exporter/manager.rs, exporter/stats.rs, exporter/util.rs]
  affects: [src/exporter/csv/mod.rs, src/exporter/sqlite/mod.rs, src/exporter/tests.rs]
tech_stack:
  added: []
  patterns: [mod-declarations-only, pub-use-re-exports]
key_files:
  created:
    - src/exporter/api.rs
    - src/exporter/kind.rs
    - src/exporter/manager.rs
    - src/exporter/stats.rs
    - src/exporter/util.rs
  modified:
    - src/exporter/mod.rs
    - Cargo.toml
    - Cargo.lock
decisions:
  - strip_ip_prefix 可见性从 pub(super) 提升为 pub(crate)，以支持 mod.rs 的合法 re-export（子模块通过 super::super 引用）
  - rusqlite 降级 0.40.0→0.39.0，因 0.40.x 依赖的 libsqlite3-sys 0.38.x 使用 cfg_select! nightly 特性，在 stable Rust 1.94.0 构建失败
  - ExporterKind 不从 mod.rs 重导出（外部模块不直接使用），通过 manager.rs 内 super::kind::ExporterKind 访问
metrics:
  duration: "约 15 分钟"
  completed: "2026-06-07T12:21:19Z"
  tasks_completed: 1
  tasks_total: 1
  files_changed: 8
---

# Phase 71 Plan 06: exporter/mod.rs 拆分为 5 个子文件 Summary

**One-liner:** 将 310 行的 exporter/mod.rs 拆分为 api/kind/manager/stats/util 五个职责文件，mod.rs 缩减至 21 行仅含声明与 re-export。

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | 拆分 exporter/mod.rs 到 5 个新文件 | 323038f | src/exporter/{api,kind,manager,stats,util}.rs + mod.rs |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] strip_ip_prefix 可见性需提升**
- **Found during:** Task 1
- **Issue:** 原计划在 util.rs 中保持 `pub(super)` 并在 mod.rs 用 `pub(super) use util::strip_ip_prefix` 重导出，但 Rust 不允许 re-export 可见性受限的私有项（E0364）
- **Fix:** 将 util.rs 中 `strip_ip_prefix` 改为 `pub(crate)`，mod.rs 用 `pub(crate) use util::strip_ip_prefix`
- **Files modified:** src/exporter/util.rs, src/exporter/mod.rs
- **Commit:** 323038f

**2. [Rule 3 - Blocking] rusqlite 0.40.0 → 0.39.0 降级**
- **Found during:** Task 1 提交阶段（pre-commit hook 触发 clippy）
- **Issue:** 上游 commit c24f56f 引入 rusqlite 0.40.0，其依赖 libsqlite3-sys 0.38.0 使用了 `cfg_select!` nightly 特性（issue #115585），在 stable rustc 1.94.0 构建失败，导致所有 cargo 命令报错
- **Fix:** Cargo.toml 改为 rusqlite 0.39.0，`cargo update rusqlite` 同步降级 libsqlite3-sys 到 0.37.0
- **Files modified:** Cargo.toml, Cargo.lock
- **Commit:** 323038f

**3. [Rule 2 - Missing] ExporterKind 不在 mod.rs 重导出**
- **Found during:** Task 1 build 阶段
- **Issue:** 原计划 `pub(crate) use kind::ExporterKind` 产生 unused import warning（-D warnings 升级为 error），因为外部模块不直接引用 ExporterKind
- **Fix:** 移除 `pub(crate) use kind::ExporterKind`，manager.rs 通过 `super::kind::ExporterKind` 内部访问
- **Files modified:** src/exporter/mod.rs
- **Commit:** 323038f

## Verification Results

- cargo clippy --all-targets -- -D warnings: PASS
- cargo test: PASS（335 单元测试 + 69 集成测试 + 1 jemalloc 测试，全部通过）
- cargo fmt --check: PASS
- mod.rs grep 仅含声明: OK
- csv/mod.rs 与 sqlite/mod.rs 的 `use super::*` 路径: OK（未修改，原路径仍解析）

## Self-Check: PASSED

- src/exporter/api.rs: FOUND
- src/exporter/kind.rs: FOUND
- src/exporter/manager.rs: FOUND
- src/exporter/stats.rs: FOUND
- src/exporter/util.rs: FOUND
- src/exporter/mod.rs: FOUND（21 行，仅 mod 声明与 pub use）
- Commit 323038f: FOUND
