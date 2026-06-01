---
phase: 52-exporter
verified: 2026-06-01T16:00:00Z
status: passed
score: 7/7 must-haves verified
overrides_applied: 0
---

# Phase 52: 统计输出与 Exporter 集成 验证报告

**Phase Goal:** 用户运行 `stats` 后可在 config.toml 指定的 CSV 或 SQLite 文件中看到两张独立的统计表：慢 SQL TOP-N（按 elapsed 降序）和高频 SQL TOP-N（按调用次数降序）
**Verified:** 2026-06-01T16:00:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #  | Truth                                                                                  | Status     | Evidence                                                                                              |
|----|----------------------------------------------------------------------------------------|------------|-------------------------------------------------------------------------------------------------------|
| 1  | 慢 SQL 表字段 sql_text/elapsed_ms/timestamp，按 elapsed 降序，行数 ≤ top_n            | VERIFIED   | `output.rs` 表头 `sql_text,elapsed_ms,timestamp`；`aggregate.rs::build_slow_rows` 降序排序；集成测试 1/2 验证 |
| 2  | 高频 SQL 表字段 normalized_sql/call_count/avg_elapsed_ms/max_elapsed_ms，按 call_count 降序，行数 ≤ top_n | VERIFIED | `output.rs` 表头正确；`build_freq_rows` 先排序后 `truncate(top_n)`；集成测试 2 限制行数               |
| 3  | CSV 配置时生成两个独立文件；SQLite 配置时两张表；同时配置时 CSV 优先                  | VERIFIED   | `write_stats_output` 明确 `if csv_cfg ... return Ok(())` 优先分支；集成测试 3/4 覆盖                  |
| 4  | `--top 5` 输出行数严格不超过 5（不足时按实际输出）                                    | VERIFIED   | `StatsAccumulator` 最小堆大小限制 + `freq_rows.truncate(top_n)`；集成测试 2 断言 `slow_data <= 5`    |
| 5  | `cargo clippy -- -D warnings` + `cargo test` 全部通过，不引入新依赖                   | VERIFIED   | clippy: 0 warnings；cargo test: 35 lib + 11 integration tests all pass；Cargo.toml [dependencies] 无新增 |
| 6  | elapsed 为 0/负数的记录纳入双侧统计（不过滤）                                         | VERIFIED   | `max_elapsed` 初值 `f32::NEG_INFINITY` 保证负数被捕获；单元测试 `test_slow_sql_includes_zero_and_negative_elapsed`；集成测试 5 |
| 7  | 解析错误不终止流程（log::warn + continue）                                             | VERIFIED   | `scan_files_into_accumulator` 中 `Err(err) => log::warn!(...)` 分支；单元测试 `test_run_stats_skips_parse_errors` |

**Score:** 7/7 truths verified

### Required Artifacts

| Artifact                    | Expected                                                  | Status     | Details                                                             |
|-----------------------------|-----------------------------------------------------------|------------|---------------------------------------------------------------------|
| `src/stats/aggregate.rs`    | SlowSqlRow/FrequentSqlRow/StatsAccumulator；BinaryHeap+HashMap | VERIFIED | 文件存在，281 行；`BinaryHeap<Reverse<SlowSqlEntry>>` + `HashMap<String, AggState>`；6 个单元测试 |
| `src/stats/output.rs`       | write_csv_stats/write_sqlite_stats 独立输出函数           | VERIFIED   | 文件存在，345 行；两个 `pub fn` 均实现，不复用 Exporter trait；9 个单元测试 |
| `src/stats/mod.rs`          | pub fn run_stats(cfg, top_n) 编排入口；pub mod aggregate/output | VERIFIED | `pub fn run_stats` 在第 17 行；`pub mod aggregate/normalize/output` 均声明 |
| `src/cli/stats/mod.rs`      | handle_stats 调用 crate::stats::run_stats                 | VERIFIED   | 第 18 行：`crate::stats::run_stats(cfg, top)` 直接调用              |
| `src/exporter/mod.rs`       | ensure_parent_dir/f32_ms_to_i64 改为 pub(crate)           | VERIFIED   | grep 命中：第 276 行 `pub(crate) fn f32_ms_to_i64`，第 302 行 `pub(crate) fn ensure_parent_dir` |

### Key Link Verification

| From                               | To                                    | Via                    | Status  | Details                                                         |
|------------------------------------|---------------------------------------|------------------------|---------|-----------------------------------------------------------------|
| `src/cli/stats/mod.rs::handle_stats` | `crate::stats::run_stats`           | 直接函数调用           | WIRED   | 第 18 行 `crate::stats::run_stats(cfg, top)` 确认               |
| `src/stats/mod.rs::run_stats`      | `SqllogParser::new(...).log_files()` | 复用 Phase 49 多输入   | WIRED   | 第 19 行 `crate::parser::SqllogParser::new(cfg.sqllog.inputs.clone()).log_files()?` |
| `src/stats/aggregate.rs::update`   | `crate::stats::normalize::normalize_sql` | 高频 SQL key 计算   | WIRED   | 第 88 行 `crate::stats::normalize::normalize_sql(&record.sql)` |
| `src/stats/output.rs`              | `crate::exporter::ensure_parent_dir` | pub(crate) 工具函数    | WIRED   | 第 5 行 `use crate::exporter::ensure_parent_dir;`；第 20/80 行调用 |
| `src/stats/output.rs::write_csv_stats` | `crate::exporter::csv::writer::write_csv_escaped` | CSV 转义复用 | WIRED | 第 4 行 import；第 34/58 行调用                                  |

### Data-Flow Trace (Level 4)

| Artifact              | Data Variable   | Source                      | Produces Real Data | Status   |
|-----------------------|-----------------|-----------------------------|--------------------|----------|
| `write_csv_stats`     | slow/frequent   | StatsAccumulator::into_results | 是，来自真实日志解析 | FLOWING  |
| `write_sqlite_stats`  | slow/frequent   | StatsAccumulator::into_results | 是，同上           | FLOWING  |
| `run_stats`           | log_files       | SqllogParser::log_files()   | 是，从文件系统扫描  | FLOWING  |

### Behavioral Spot-Checks

| Behavior                            | Command                               | Result        | Status |
|-------------------------------------|---------------------------------------|---------------|--------|
| 所有 stats 单元测试通过             | `cargo test stats`                    | 35 passed     | PASS   |
| 所有 stats 集成测试通过             | `cargo test --test integration stats` | 11 passed     | PASS   |
| clippy 零警告                       | `cargo clippy --all-targets -- -D warnings` | 0 warnings | PASS   |
| fmt 通过                            | `cargo fmt --check`                   | 无输出（通过） | PASS   |

### Requirements Coverage

| Requirement | Source Plan | Description                                        | Status    | Evidence                                                        |
|-------------|-------------|----------------------------------------------------|-----------|-----------------------------------------------------------------|
| STATS-03    | 52-01-PLAN  | 慢 SQL TOP-N，按 elapsed 降序，含 SQL 文本/elapsed/时间戳 | SATISFIED | `SlowSqlRow` 字段；降序排序；集成测试 1 验证表头                |
| STATS-04    | 52-01-PLAN  | 高频 SQL TOP-N，按调用次数降序，含标准化SQL/调用次数/avg elapsed/max elapsed | SATISFIED | `FrequentSqlRow` 字段；`sort_by call_count desc`；集成测试 1 验证表头 |
| STATS-05    | 52-01-PLAN  | 统计结果输出格式遵循 config.toml 中的 exporter 配置 | SATISFIED | `write_stats_output` CSV 优先路由；集成测试 3（SQLite）、4（CSV 优先）覆盖 |

### Anti-Patterns Found

| File                      | Line | Pattern       | Severity | Impact |
|---------------------------|------|---------------|----------|--------|
| 无 debt markers           | —    | —             | —        | 无     |

扫描了所有 Phase 52 修改的文件（`src/stats/aggregate.rs`、`src/stats/output.rs`、`src/stats/mod.rs`、`src/cli/stats/mod.rs`、`src/exporter/mod.rs`），无 TBD/FIXME/XXX/placeholder 等 debt markers。

### Human Verification Required

无需人工验证。所有 must-have 均可通过代码审查和测试结果确认：
- 输出文件格式正确性通过集成测试验证
- 排序行为通过单元测试验证
- CSV/SQLite 优先策略通过集成测试 4 覆盖

---

## Gaps Summary

无 gaps。Phase 52 所有 5 条 ROADMAP Success Criteria 全部满足，STATS-03/STATS-04/STATS-05 需求全部覆盖，23 个测试（6 + 9 + 5 + 3）全部通过，cargo clippy 零警告，无新增依赖。

---

_Verified: 2026-06-01T16:00:00Z_
_Verifier: Claude (gsd-verifier)_
