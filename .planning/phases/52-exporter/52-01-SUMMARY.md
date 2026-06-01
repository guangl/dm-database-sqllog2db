---
phase: 52-exporter
plan: "01"
subsystem: stats
tags: [stats, csv, sqlite, aggregation, top-n]
dependency_graph:
  requires: [50-sql, 51-stats-cli]
  provides: [run_stats, write_csv_stats, write_sqlite_stats, StatsAccumulator]
  affects: [src/stats/, src/cli/stats/, src/exporter/mod.rs, src/main.rs, tests/integration.rs]
tech_stack:
  added: []
  patterns: [BinaryHeap-min-heap, HashMap-aggregation, single-pass-streaming, CSV-BufWriter, SQLite-transaction]
key_files:
  created:
    - src/stats/aggregate.rs
    - src/stats/output.rs
  modified:
    - src/stats/mod.rs
    - src/cli/stats/mod.rs
    - src/exporter/mod.rs
    - src/main.rs
    - tests/integration.rs
decisions:
  - "StatsAccumulator 使用 BinaryHeap<Reverse<SlowSqlEntry>> 维护慢 SQL TOP-N，O(M log N) 时间 O(N) 内存"
  - "高频 SQL 聚合使用 HashMap<String, AggState>，key 为 normalize_sql 结果"
  - "write_csv_stats / write_sqlite_stats 独立实现，不复用 Exporter trait（D-06）"
  - "CSV 输出目录优先级高于 SQLite，与 ExporterManager 一致（CSV priority policy）"
  - "SQLite 写入使用 DROP + CREATE（不累积历史数据，D-10）"
  - "ensure_parent_dir 和 f32_ms_to_i64 从 pub(super) 改为 pub(crate)（最小侵入式改动）"
  - "在 main.rs 添加 mod stats 以使 binary crate 能通过 crate::stats 访问统计模块"
metrics:
  duration_seconds: 1049
  completed_date: "2026-06-01"
  task_count: 3
  file_count: 7
---

# Phase 52 Plan 01: 统计输出与 Exporter 集成 Summary

**一行概述：** 实现 `sqllog2db stats` 命令端到端功能——单次流式扫描日志文件，通过最小堆+HashMap 双侧聚合 TOP-N 慢 SQL 与高频 SQL，通过独立输出函数写入 CSV（两文件）或 SQLite（两表，DROP+CREATE）。

## 产出文件

| 文件 | 状态 | 说明 |
|------|------|------|
| `src/stats/aggregate.rs` | 新建 | `StatsAccumulator`、`SlowSqlRow`、`FrequentSqlRow`、6 个单元测试 |
| `src/stats/output.rs` | 新建 | `write_csv_stats`、`write_sqlite_stats`、9 个单元测试 |
| `src/stats/mod.rs` | 修改 | 注册 aggregate/output 子模块，实现 `pub fn run_stats`，3 个单元测试 |
| `src/cli/stats/mod.rs` | 修改 | 接入 `crate::stats::run_stats`，修复旧有单元测试 |
| `src/exporter/mod.rs` | 修改 | `ensure_parent_dir` 和 `f32_ms_to_i64` 改为 `pub(crate)` |
| `src/main.rs` | 修改 | 添加 `mod stats;` 供 binary crate 访问 |
| `tests/integration.rs` | 修改 | 5 个新集成测试；修复旧 stats 测试（Phase 51 遗留存根问题） |

## 测试结果（23 项目标 + 4 偏差修复）

### Task 1 单元测试（6/6 通过）
- `test_slow_sql_top_n_limit` ✓
- `test_slow_sql_includes_zero_and_negative_elapsed` ✓
- `test_frequent_sql_aggregation` ✓
- `test_frequent_sql_top_n_limit_and_sort` ✓
- `test_slow_entry_total_cmp_handles_equal_elapsed` ✓
- `test_into_results_when_records_fewer_than_top_n` ✓

### Task 2 单元测试（9/9 通过）
- `test_write_csv_stats_creates_two_files` ✓
- `test_write_csv_stats_headers_and_rows` ✓
- `test_write_csv_stats_escapes_double_quotes` ✓
- `test_write_csv_stats_creates_parent_dir` ✓
- `test_write_csv_stats_empty_rows` ✓
- `test_write_sqlite_stats_creates_two_tables` ✓
- `test_write_sqlite_stats_schema` ✓
- `test_write_sqlite_stats_drop_recreates` ✓
- `test_write_sqlite_stats_creates_parent_dir` ✓

### Task 3 单元测试（3/3 通过）
- `test_run_stats_csv_mode_selected_when_only_csv_configured` ✓
- `test_run_stats_propagates_no_files_found` ✓
- `test_run_stats_skips_parse_errors` ✓

### Task 3 集成测试（5/5 通过）
- `test_stats_csv_outputs_two_files` ✓
- `test_stats_csv_top_5_limits_rows` ✓
- `test_stats_sqlite_outputs_two_tables` ✓
- `test_stats_csv_preferred_over_sqlite_when_both_configured` ✓
- `test_stats_zero_elapsed_records_included` ✓

### 全套质量门禁
- `cargo fmt --check`: PASS
- `cargo clippy --all-targets -- -D warnings`: PASS (0 warnings)
- `cargo test`: PASS (255 lib + 57 integration + 1 benchmark = 313 tests)

## Cargo.toml 依赖变更

无新增依赖（STATS-05 Success Criteria 5 满足）。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 修复 test_into_results_when_records_fewer_than_top_n 测试**
- **Found during:** Task 1 TDD 红灯阶段
- **Issue:** 测试使用 `"SELECT 1"` 和 `"SELECT 2"` — normalize_sql 将两者规范化为相同 key `"SELECT ?"` → frequent.len() = 1 ≠ 2
- **Fix:** 改用结构不同的 SQL（`SELECT id FROM users` 和 `INSERT INTO orders VALUES (1)`），normalize 后 key 不同
- **Files modified:** `src/stats/aggregate.rs`
- **Commit:** 5e4a5df

**2. [Rule 1 - Bug] 修复 Phase 51 遗留测试（使用 `__placeholder__` 路径）**
- **Found during:** Task 3 实现后运行全套测试
- **Issue:** `make_stats_config_file` 使用 `inputs = ["__placeholder__"]`（不存在路径）；Phase 51 时 handle_stats 是存根可以通过，Phase 52 接入 run_stats 后会返回 `PathNotFound` 错误
- **Fix:** 修改 `make_stats_config_file` 创建真实的 `input.log` 文件（含 1 条有效 DML 记录）
- **Files modified:** `tests/integration.rs`
- **Commit:** 2a0e8f1

**3. [Rule 1 - Bug] 修复 cli::stats 单元测试（使用 Config::default() 导致 PathNotFound）**
- **Found during:** Task 3 运行 cargo test
- **Issue:** `test_handle_stats_top_default_passes` 和 `test_handle_stats_top_nonzero_passes` 使用 `Config::default()`，其 `sqllog.inputs = ["sqllogs"]`，该目录不存在
- **Fix:** 添加 `make_test_config_with_log()` helper，提供真实的临时日志文件和 CSV 配置
- **Files modified:** `src/cli/stats/mod.rs`
- **Commit:** 2a0e8f1

**4. [Rule 2 - Missing] 在 main.rs 添加 mod stats 声明**
- **Found during:** Task 3 编译阶段
- **Issue:** `src/cli/stats/mod.rs` 中 `crate::stats::run_stats` 在 binary crate 无法解析，因为 `main.rs` 没有声明 `mod stats;`
- **Fix:** 在 `src/main.rs` 添加 `mod stats;`（与 lib crate 的 `pub mod stats` 平行）
- **Files modified:** `src/main.rs`
- **Commit:** 2a0e8f1

## Commits

| Hash | Task | Message |
|------|------|---------|
| 5e4a5df | Task 1 | feat(stats): add StatsAccumulator with slow heap and frequent map |
| 0aa343f | Task 2 | feat(stats): add write_csv_stats and write_sqlite_stats for stats output |
| 2a0e8f1 | Task 3 | feat(stats): wire run_stats orchestrator and CLI integration for stats subcommand |

## Known Stubs

无 — 所有输出函数已完整实现，无 TODO/FIXME/placeholder。

## Threat Flags

无新增安全威胁面（所有输入来自本地配置文件和本地日志文件，STRIDE T-52-01/02/03/04 均在 PLAN 威胁模型中覆盖）。

## v1.13 Milestone 闭环

- Phase 50 (normalize_sql): DONE
- Phase 51 (CLI 脚手架): DONE
- Phase 52 (统计输出): DONE（本阶段）

`sqllog2db stats -c config.toml --top N` 端到端功能全部实现，v1.13 milestone 闭环完成。

## Self-Check: PASSED

Files exist:
- src/stats/aggregate.rs: FOUND
- src/stats/output.rs: FOUND
- src/stats/mod.rs: FOUND (modified)
- src/cli/stats/mod.rs: FOUND (modified)
- src/exporter/mod.rs: FOUND (modified)
- src/main.rs: FOUND (modified)
- tests/integration.rs: FOUND (modified)
- .planning/phases/52-exporter/52-01-SUMMARY.md: FOUND

Commits exist:
- 5e4a5df: FOUND
- 0aa343f: FOUND
- 2a0e8f1: FOUND
