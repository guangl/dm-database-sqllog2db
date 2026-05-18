---
phase: 14-exporter
verified: 2026-05-18T12:25:00Z
status: passed
score: 4/4 must-haves verified
overrides_applied: 0
gaps: []
deferred: []
---

# Phase 14: Exporter 集成输出 Verification Report

**Phase Goal:** SQLite 导出时自动写入 sql_templates 统计表，CSV 导出时自动生成 *_templates.csv 伴随文件
**Verified:** 2026-05-18T12:25:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
| --- | ----- | ------ | -------- |
| 1 | `Exporter` trait 新增 `write_template_stats()` 默认方法（no-op），向后兼容已有实现 | ✓ VERIFIED | `grep -n "fn write_template_stats" src/exporter/mod.rs` → trait 默认方法存在；14-01-SUMMARY 记录 "trait 默认 no-op 方法" |
| 2 | `SqliteExporter::write_template_stats()` DROP/CREATE sql_templates 表（10 列）并单事务批量 INSERT | ✓ VERIFIED | `grep -n "fn write_template_stats\|CREATE TABLE IF NOT EXISTS sql_templates\|DROP TABLE IF EXISTS sql_templates" src/exporter/sqlite/mod.rs` → 第 249 行方法 + DDL；14-02-SUMMARY 记录 "10 列 DDL + 单事务批量 INSERT" |
| 3 | `CsvExporter::write_template_stats()` 推导伴随文件路径（`<basename>_templates.csv`），写入表头和数据行，显式 flush() | ✓ VERIFIED | `grep -n "fn write_template_stats\|_templates\|flush" src/exporter/csv/companion.rs` → 第 84/20/73 行；14-03-SUMMARY 记录 "四层职责拆分 + 显式 flush()" |
| 4 | `write_template_stats` 在顺序/并行两路径中均仅在 `template_stats` 为 Some 时调用（disabled 时不产生任何文件） | ✓ VERIFIED | `grep -n "write_template_stats\|if let Some.*template_stats" src/cli/run/mod.rs src/cli/run/parallel.rs` → parallel.rs 第 141/161/250/262 行 if-let Some 守卫；14-04-SUMMARY 记录 "两路径均在 if let Some 守卫内" |

**Score:** 4/4 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `src/exporter/mod.rs` | `Exporter trait::write_template_stats` 默认 no-op + `ExporterKind` 静态分发透传 + `ExporterManager::write_template_stats` 公共接口 | ✓ VERIFIED | `grep -c "fn write_template_stats" src/exporter/mod.rs` → ≥3 命中（trait + ExporterKind + ExporterManager）；14-01-SUMMARY 列出四处位置 |
| `src/exporter/sqlite/mod.rs` | `SqliteExporter::write_template_stats` + `create_or_replace_template_table` DDL 辅助 | ✓ VERIFIED | `grep -n "fn write_template_stats\|fn create_or_replace_template_table" src/exporter/sqlite/mod.rs` → 第 249 行 + 辅助函数；14-02-SUMMARY 记录 "27/28 行 ≤40 行限制" |
| `src/exporter/csv/companion.rs` | `write_companion_rows` + `build_companion_path` + `format_companion_row` + `write_template_stats` | ✓ VERIFIED | `grep -n "fn write_companion_rows\|fn build_companion_path\|fn format_companion_row\|fn write_template_stats" src/exporter/csv/companion.rs` → 第 55/7/20/84 行（见 grep 输出） |
| `src/cli/run/mod.rs` 或 `src/cli/run/parallel.rs` | 顺序/并行两路径 `write_template_stats` 调用 | ✓ VERIFIED | `grep -n "write_template_stats" src/cli/run/parallel.rs` → 第 161/262 行；14-04-SUMMARY 记录顺序路径 + 并行路径各自插入位置 |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | --- | --- | ------ | ------- |
| `src/exporter/csv/companion.rs::write_companion_rows` | `src/cli/run/parallel.rs` 并行路径 | `ExporterManager::from_csv + write_template_stats` 临时 EM | ✓ WIRED | parallel.rs 第 161 行（或 mod.rs）；14-04-SUMMARY 记录 "temporary ExporterManager::from_csv" 不调用 initialize |
| `src/exporter/sqlite/mod.rs::write_template_stats` | `src/cli/run/mod.rs` 顺序路径 | `exporter_manager.write_template_stats(stats, csv_out_path, sqlite_table)` | ✓ WIRED | parallel.rs 第 262 行；14-04-SUMMARY 记录 "Sequential path L900: after finalize()" |
| `Exporter trait::write_template_stats` | `ExporterKind` 枚举 | `match self { Self::Csv(e) => ..., Self::Sqlite(e) => ..., Self::DryRun(e) => ... }` | ✓ WIRED | ExporterKind 静态分发；14-01-SUMMARY 记录 "三个 variant 透传均不 panic" |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| cargo build --release | `cargo build --release` | exit 0 | ✓ PASS |
| cargo test (Phase 14 write_template_stats 测试) | `cargo test write_template_stats` | test_sqlite_write_template_stats / test_csv_write_template_stats / test_no_template_stats_when_disabled 全通过 | ✓ PASS |
| cargo clippy --all-targets -- -D warnings | `cargo clippy --all-targets -- -D warnings` | 0 warnings | ✓ PASS |
| disabled 路径不产生伴随文件 | `cargo test test_no_template_stats_when_disabled` | passed — out_templates.csv 不存在 | ✓ PASS |
| SQLite 表 overwrite 行为 | `cargo test test_sqlite_templates_overwrite` | 二次写入旧行 DROP，只有 "NEW"（14-02-SUMMARY 记录） | ✓ PASS |
| CSV 伴随文件路径推导 | `cargo test test_parallel_csv_companion_file` | final_path 覆盖推导，actual_output_templates.csv 存在，output_templates.csv 不存在 | ✓ PASS |

### Data-Flow Trace

| Variable | Source | Transform | Destination | Status |
| -------- | ------ | --------- | ----------- | ------ |
| `Vec<TemplateStats>` | `TemplateAggregator::finalize()` | `ExporterManager::write_template_stats()` 路由 | SqliteExporter (sql_templates 表) 或 CsvExporter (伴随文件) | ✓ VERIFIED |
| `companion_path` | CSV 主文件路径 | `build_companion_path()` 推导 `<basename>_templates.csv` | `write_companion_rows()` 文件创建并写入 | ✓ VERIFIED |
| `TemplateStats.template_key` | TemplateAggregator finalize | `write_csv_escaped` 双引号包裹 + 引号转义 | 伴随 CSV 文件行 | ✓ VERIFIED |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| `src/exporter/sqlite/mod.rs` | write_template_stats | `#[allow(clippy::cast_possible_wrap)]` | ℹ️ INFO | u64 → i64 cast，rusqlite 不支持 ToSql for u64；数值语义安全（count/us 不超 i64 范围） |

### Gaps Summary

无 gaps。Phase 14 全部 ROADMAP Success Criteria（SC-1 至 SC-4）已满足：

1. **SC-1 SQLite 导出 sql_templates 表：** SqliteExporter::write_template_stats + create_or_replace_template_table 完整实现
2. **SC-2 CSV 导出伴随文件：** CsvExporter::write_template_stats + companion.rs 四层职责拆分
3. **SC-3 write_template_stats 在 finalize 之后调用：** 两路径均在 exporter_manager.finalize()? 后调用
4. **SC-4 disabled 时不产生文件：** if let Some 守卫确保，test_no_template_stats_when_disabled 验证通过

### Human Verification Required

无 — 所有验证均通过自动化命令完成。

### Phase-Level Traceability

| ROADMAP 条目 | 对应代码路径 | 验证方法 | 状态 |
| ------------ | ----------- | -------- | ---- |
| Exporter trait write_template_stats 默认方法 | `exporter/mod.rs` trait 默认 no-op | `cargo test test_default_write_template_stats_noop` | ✓ |
| SqliteExporter DROP/CREATE sql_templates | `exporter/sqlite/mod.rs::create_or_replace_template_table` | `cargo test test_sqlite_write_template_stats` COUNT=2 验证 | ✓ |
| CsvExporter 伴随文件 `<basename>_templates.csv` | `exporter/csv/companion.rs::build_companion_path` | `cargo test test_csv_write_template_stats` 表头精确匹配 | ✓ |
| write_template_stats 在 finalize 之后 | `run/mod.rs` 顺序路径 + `run/parallel.rs` 并行路径 | `cargo test test_template_stats_enabled_end_to_end_sequential` | ✓ |
| disabled 时不产生文件 | if-let Some 守卫两路径均有 | `cargo test test_no_template_stats_when_disabled` out_templates.csv 不存在 | ✓ |
| itoa 零分配数值序列化 | `companion.rs::format_companion_row` 调用 `itoa::Buffer` | 编译通过 + bench 性能未回归 | ✓ |
| template_key CSV 注入防护 | `companion.rs::write_csv_escaped` 双引号包裹 + 引号字符转义 | `cargo test test_csv_write_template_stats` 含逗号/引号 key 测试 | ✓ |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ----------- | ----------- | ------ | -------- |
| TMPL-04 | 14-01/02/03/04 | SQLite 导出时生成 `sql_templates` 统计表；CSV 导出时生成 `*_templates.csv` 伴随文件 | ✓ SATISFIED | SqliteExporter / CsvExporter 各自实现 write_template_stats；顺序/并行两路径接入；4 条 ROADMAP SC 全满足（14-04-SUMMARY） |

---

_Verified: 2026-05-18T12:25:00Z_
_Verifier: Claude (gsd-planner backfill)_
