---
phase: 43-parser-api-filter
plan: "01"
subsystem: pipeline/filters
tags: [refactor, type-alignment, comment-cleanup]
dependency_graph:
  requires: []
  provides: [IndicatorFilters::matches(u32)]
  affects: [src/pipeline/filters/mod.rs, src/cli/run/prescan.rs]
tech_stack:
  added: []
  patterns: [type-alignment, version-agnostic-comments]
key_files:
  created: []
  modified:
    - src/pipeline/filters/mod.rs
    - src/cli/run/prescan.rs
    - src/pipeline/mod.rs
    - src/cli/run/filter_processor.rs
    - src/cli/run/processor.rs
    - src/exporter/mod.rs
    - src/exporter/sqlite/write.rs
    - src/exporter/csv/writer.rs
decisions:
  - "将 IndicatorFilters::matches 第三参数从 i64 改为 u32，与 parser 库 Sqllog.rowcount: u32 类型对齐，消除调用方冗余的 i64::from() 转换"
  - "将所有 v1.1.0 版本号注释替换为 'parser 库' 表述，避免注释与版本绑定导致的过时问题"
metrics:
  duration: "4m"
  completed: "2026-05-24T08:31:07Z"
  tasks_completed: 2
  tasks_total: 2
---

# Phase 43 Plan 01: Parser API Filter 类型对齐 Summary

**One-liner:** 将 `IndicatorFilters::matches` 的 `row_count` 参数从 `i64` 改为 `u32`，消除调用方冗余的 `i64::from(result.rowcount)` 转换，并清理 7 处 v1.1.0 版本绑定注释。

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | 调整 IndicatorFilters::matches 签名为 row_count: u32 | d27b1ef | src/pipeline/filters/mod.rs |
| 2 | 更新 prescan.rs 调用点 + 修订过时注释 + 清理 v1.1.0 残留 | e8c4426 | src/cli/run/prescan.rs, src/pipeline/mod.rs, src/cli/run/filter_processor.rs, src/cli/run/processor.rs, src/exporter/mod.rs, src/exporter/sqlite/write.rs, src/exporter/csv/writer.rs |

## What Was Built

- `IndicatorFilters::matches` 签名第三参数由 `i64` 改为 `u32`，函数体内 `i64::from(min_r)` 比较消除，直接 `row_count >= min_r`（两侧均为 u32）
- `prescan.rs` 调用点从 `i64::from(result.rowcount)` 改为直接 `result.rowcount`（u32 直传）
- `prescan.rs` collect 注释由 "v1.1.0 的 LogParser" 改为 "LogIterator 未实现 rayon::IntoParallelIterator trait"，版本无关
- 7 个文件中的 v1.1.0 版本绑定注释全部替换为 "parser 库" 表述

## Verification Results

- `cargo build --release`: 无 warning，无 error
- `cargo test --lib -- pipeline::filters::tests::test_indicator`: 5 个测试全部通过
- `cargo test` (全套): 215 个 lib 测试 + integration tests 全部通过
- `cargo clippy --all-targets -- -D warnings`: 通过
- `cargo fmt --check`: 通过
- `grep -c "i64::from(result.rowcount)" src/cli/run/prescan.rs`: 0
- `grep -rn "v1\.1\.0" src/`: 空（无残留）

## Deviations from Plan

None - plan executed exactly as written.

## Known Stubs

None.

## Threat Flags

None - changes are purely type alignment and comment cleanup, no new security surface introduced.

## Self-Check: PASSED

- src/pipeline/filters/mod.rs: FOUND
- src/cli/run/prescan.rs: FOUND
- Commit d27b1ef: FOUND
- Commit e8c4426: FOUND
