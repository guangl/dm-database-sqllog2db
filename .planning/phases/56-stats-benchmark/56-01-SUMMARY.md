---
phase: 56-stats-benchmark
plan: "01"
subsystem: scanner / stats
tags: [refactor, scanner, stats, error-handling, clean-01]
dependency_graph:
  requires: []
  provides: [scanner-module, stats-scanner-refactor]
  affects: [src/scanner.rs, src/lib.rs, src/stats/mod.rs, src/main.rs]
tech_stack:
  added: []
  patterns: [pub(crate)-module, ErrorStats-callback, FnMut-callback]
key_files:
  created:
    - src/scanner.rs
  modified:
    - src/lib.rs
    - src/stats/mod.rs
    - src/main.rs
decisions:
  - "scanner 模块可见性使用 pub(crate)，与现有 pub(crate) mod parser 保持一致"
  - "scan_files 函数接受 &mut FnMut 回调，调用方保留完全控制权（stats 传 accumulator.update，run 可传导出逻辑）"
  - "parse error 汇总日志（文件级）改为 scanner 内部 warn!，调用方通过 ErrorStats 观察计数"
  - "main.rs 需要单独声明 mod scanner（bin target 独立模块树，不继承 lib.rs 声明）"
metrics:
  duration: "约 40 分钟"
  completed_date: "2026-06-02"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 4
---

# Phase 56 Plan 01: scanner 公共模块与 stats 重构 Summary

新建 `pub(crate) mod scanner` 模块，将 `scan_files_into_accumulator` 内部实现替换为调用 `scanner::scan_files`，统一 parse error 计数（`ErrorStats`）+ `log::warn!` 可观测性，通过 grep+awk 验证 CLEAN-01 静态条件。

## Tasks Completed

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 1 | 新建 src/scanner.rs 公共扫描模块并在 lib.rs 注册 | d77c50a | src/scanner.rs, src/lib.rs |
| 2 | 重构 stats/mod.rs 调用 scanner + 验证 CLEAN-01 静态条件 | e309a93 | src/stats/mod.rs, src/scanner.rs, src/main.rs |

## What Was Built

**src/scanner.rs (新建)**

公共文件扫描模块，提供 `pub(crate) fn scan_files<F>(log_files: &[PathBuf], on_record: &mut F, stats: &mut ErrorStats) -> Result<()>` 函数：
- 文件路径错误（non-UTF8 或打开失败）返回 `Err`，终止扫描
- 单条记录 parse error：`stats.add_parse_error()` 计数 + `log::warn!`，不终止迭代
- 文件级 parse error 汇总 `log::warn!`（若有错误）
- 2 个单元测试：`test_scan_files_counts_parse_errors`、`test_scan_files_returns_err_on_invalid_path`

**src/stats/mod.rs (重构)**

`scan_files_into_accumulator` 函数体从 30 行 LogParserBuilder 循环替换为 10 行 scanner 调用：
- 引入 `ErrorStats` 到 use 声明
- 调用 `crate::scanner::scan_files(log_files, &mut |record| accumulator.update(record), &mut scan_stats)?`
- parse error 通过 `log::info!` 汇总报告

**src/main.rs (修复)**

添加 `mod scanner;` 声明（bin target 的模块树独立于 lib.rs，需要单独声明）。

## Verification Results

- `cargo build --release`: 成功，无警告
- `cargo test`: 全套通过（261 lib + 292 bin + 64 integration + 1 jemalloc = 618 tests）
- `cargo clippy --all-targets -- -D warnings`: 零告警
- `cargo fmt --check`: 零差异

**CLEAN-01 静态检查：**
- `src/cli/stats/mod.rs` 无 `warn!` 调用：grep 返回 0（已确认）
- `src/stats/output.rs` 所有函数实际行数 ≤40 行（已确认，见偏差说明）

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] main.rs 需要单独声明 mod scanner**

- **Found during:** Task 2 clippy 验证
- **Issue:** `src/stats/mod.rs` 在 lib crate 中调用 `crate::scanner::scan_files`，但 bin target (`src/main.rs`) 有独立的模块树，编译时报 `E0433: could not find 'scanner' in the crate root`
- **Fix:** 在 `src/main.rs` 模块声明列表中添加 `mod scanner;`（与其他 `mod` 声明保持一致顺序）
- **Files modified:** src/main.rs
- **Commit:** e309a93

**2. [Rule 2 - Missing critical] scan_files 的临时 #[allow(dead_code)]**

- **Found during:** Task 1 clippy 验证
- **Issue:** `pub(crate) fn scan_files` 在 Task 1 时仅被 `#[cfg(test)]` 代码使用，clippy 报 dead_code 错误
- **Fix:** 临时添加 `#[allow(dead_code)]`；Task 2 时 `stats/mod.rs` 开始调用，dead_code 警告自动消除，同步删除 allow 属性
- **Files modified:** src/scanner.rs
- **Commit:** d77c50a (添加), e309a93 (删除)

### CLEAN-01 awk 验证 False Positive 说明

计划中的 awk 检查命令：
```bash
awk '/^[[:space:]]*(pub )?fn /{name=$0; start=NR} /^}$/{if(NR-start>40){...}' src/stats/output.rs
```

对 `write_sqlite_stats` 报告"45 lines"为 false positive。原因：`write_sqlite_stats` 的函数签名跨 5 行（第 76-80 行），awk 的 `(pub )?fn` 模式无法匹配 `pub(crate) fn`，导致函数开始行未被正确识别。实际函数体为第 80-95 行，共 15 行，远低于 40 行限制。

**真实行数（手动验证）：**

| 函数 | 实际行数 |
|------|---------|
| write_csv_stats | 10 |
| write_slow_csv | 21 |
| write_frequent_csv | 23 |
| write_sqlite_stats | 15 |
| run_sqlite_transaction | 9 |
| write_slow_table | 19 |
| write_frequent_table | 30 |
| db_err | 3 |

全部 ≤40 行，CLEAN-01 实际满足。

## Threat Surface Scan

本计划不引入新的网络端点、认证路径或文件访问模式。`scanner.rs` 的 trust boundary 与 `T-56-01` 至 `T-56-04` 完全匹配，无超出 threat model 的新暴露面。

## Known Stubs

无。所有函数均正确实现，无占位符或 TODO 项。

## Self-Check: PASSED

- [x] src/scanner.rs 存在
- [x] d77c50a commit 存在
- [x] e309a93 commit 存在
- [x] 261 lib tests pass
- [x] clippy 零告警
- [x] fmt 零差异
