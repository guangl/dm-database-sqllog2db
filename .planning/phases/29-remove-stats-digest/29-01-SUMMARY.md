---
phase: 29-remove-stats-digest
plan: 01
type: execute
subsystem: cli
tags:
  - remove-stats
  - cli-cleanup
  - rm-03
requires: []
provides:
  - "sqllog2db --help no longer shows stats subcommand"
affects:
  - src/cli/mod.rs
  - src/cli/opts.rs
  - src/main.rs
  - src/lang.rs
  - tests/integration.rs
tech-stack:
  added: []
  patterns:
    - "移除 CLI 子命令的标准流程：删除文件 → 清理 enum → 清理 match arm → 清理 i18n → 清理测试 → 编译验证"
key-files:
  created: []
  modified:
    - src/cli/mod.rs
    - src/cli/opts.rs
    - src/main.rs
    - src/lang.rs
    - tests/integration.rs
  deleted:
    - src/cli/stats.rs (~1043 lines)
decisions: []
metrics:
  duration: "~10 min"
  completed_date: "2026-05-20"
---

# Phase 29 Plan 01: 移除 stats CLI 子命令

移除 stats 统计子命令（RM-03），减少代码量约 1043 行（含 ~230 行测试）。serde_json 依赖暂保留（digest.rs 仍使用，由 Plan 02 统一移除）。

## 执行摘要

**文件变更：**
- **删除：** `src/cli/stats.rs`（1043 行，含 stats 命令实现、handle_stats 函数和 ~230 行测试）
- **修改：** `src/cli/mod.rs` — 删除 `pub mod stats;`
- **修改：** `src/cli/opts.rs` — 删除 `Commands::Stats` variant（含 10 个 arg 字段）
- **修改：** `src/main.rs` — 删除 Stats match arm（~36 行）、更新 `needs_simple_logging` 的 matches! 宏、更新注释
- **修改：** `src/lang.rs` — 删除 `.mut_subcommand("stats", zh_stats)` 引用和 `fn zh_stats()` 函数（~14 行）
- **修改：** `tests/integration.rs` — 删除 `handle_stats` 导入、删除 14 个 stats 测试函数（~235 行），保留 `make_stats_cfg` 辅助函数（digest 测试仍使用）

**验证结果：**
- `cargo build` — 编译通过
- `cargo clippy --all-targets -- -D warnings` — 通过，无警告
- `cargo test` — 全部 764 测试通过（349 lib + 367 bin + 48 integration）
- `cargo fmt` — 格式通过

**引用扫描（确认无残留）：**
- `grep -r "pub mod stats" src/cli/mod.rs` — 无输出
- `grep -r "Commands::Stats" src/main.rs` — 无输出
- `grep -r "zh_stats" src/lang.rs` — 无输出
- `grep -r "Stats" src/cli/opts.rs` — 无输出
- `test -f src/cli/stats.rs` — 文件已删除

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] 集成测试文件引用已删除的 stats 模块**

- **Found during:** Task 1 (clippy 阶段)
- **Issue:** `tests/integration.rs` 在第 7 行导入 `dm_database_sqllog2db::cli::stats::handle_stats`，并在 14 个测试函数中调用 `handle_stats()`，同时在 stats 测试区域定义了辅助函数 `write_test_log_multi_ts`。stats 文件删除后，该文件无法编译。
- **Fix:** 删除导入行、删除 14 个 stats 测试函数（test_handle_stats_empty_dir 到 test_handle_stats_group_and_bucket_non_quiet）、删除仅 stats 使用的辅助函数 `write_test_log_multi_ts`。保留 `make_stats_cfg`（digest 测试仍使用）。
- **Files modified:** `tests/integration.rs`
- **Commit:** `c4d4288`

## Threat Surface Scan

无新增威胁 — 本次仅删除已存在的 CLI 子命令，未引入任何新网络端点、认证路径或文件访问模式。

## Self-Check: PASSED

验证项全部通过：
- [x] `src/cli/stats.rs` 已删除
- [x] `src/cli/mod.rs` 无 `pub mod stats` 声明
- [x] `src/cli/opts.rs` 无 `Stats` variant
- [x] `src/main.rs` 无 Stats match arm
- [x] `src/lang.rs` 无 `zh_stats`
- [x] `cargo build` 编译成功
- [x] `cargo clippy --all-targets -- -D warnings` 通过
- [x] `cargo test` 全部通过（764 passed）
