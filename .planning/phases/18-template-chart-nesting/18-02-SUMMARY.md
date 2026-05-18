---
phase: 18-template-chart-nesting
plan: "02"
subsystem: exporter/cli
tags: [refactor, breaking-change, exporter, cli, signature-upgrade]
dependency_graph:
  requires:
    - 18-01 (TemplateConfig/OutputConfig struct 定义，Config 顶层字段)
  provides:
    - write_template_stats 新签名 (csv_output_path: Option<&str>, sqlite_table_name: Option<&str>)
    - CsvExporter 显式路径写入（不再推导 companion 文件名）
    - SqliteExporter 动态表名 + ascii_alphanumeric 注入防护
    - run.rs 并行/顺序两条路径均从 cfg.template 派生显式输出路径
  affects:
    - src/exporter/mod.rs
    - src/exporter/csv.rs
    - src/exporter/sqlite.rs
    - src/cli/run.rs
tech_stack:
  added: []
  patterns:
    - Option<&str> 参数替代 Option<&Path> 实现跨 exporter 路径传递
    - ascii_alphanumeric_or_underscore 轻量校验防 SQLite DDL 注入
    - pub(crate) #[allow(dead_code)] 保留但不强制使用的辅助函数
key_files:
  created: []
  modified:
    - src/exporter/mod.rs
    - src/exporter/csv.rs
    - src/exporter/sqlite.rs
    - src/cli/run.rs
decisions:
  - "CsvExporter.write_template_stats 改为直接写指定路径，build_companion_path 保留为 pub(crate) 供未来使用"
  - "SqliteExporter 在拼接 DDL 前做轻量 ascii_alphanumeric_or_underscore 校验，Config.validate() 层的强校验由 Plan 03 补强"
  - "并行路径直接调用 write_companion_rows + SqliteExporter（绕过 ExporterManager），避免在并行任务外重建完整实例"
metrics:
  duration_minutes: 60
  completed_date: "2026-05-17"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 4
---

# Phase 18 Plan 02: Exporter Trait 签名升级与调用方迁移 Summary

将 `write_template_stats` 签名从基于路径推导的 `final_path: Option<&Path>` 升级为显式双参数 `(csv_output_path: Option<&str>, sqlite_table_name: Option<&str>)`，消除 D-04 路径推导耦合；并在 run.rs 并行/顺序两条路径中从 `cfg.template` 派生显式路径传入。

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | 升级 Exporter trait + CsvExporter / SqliteExporter write_template_stats 签名 | f48afc6 | src/exporter/mod.rs, csv.rs, sqlite.rs |
| 2 | 迁移 run.rs 调用方 — template 显式输出路径 | 12db626 | src/cli/run.rs |

## What Was Built

**Task 1 — Exporter 签名升级（f48afc6）：**
- `Exporter` trait `write_template_stats` 签名：`final_path: Option<&Path>` → `(csv_output_path: Option<&str>, sqlite_table_name: Option<&str>)`
- `ExporterKind::write_template_stats` + `ExporterManager::write_template_stats` 透传新签名
- `DryRunExporter::write_template_stats` 日志格式更新为同时打印两个参数
- `CsvExporter::write_template_stats`：csv_output_path 为 None/空串时跳过，非空时直接写指定路径（不再调用 build_companion_path）
- `SqliteExporter::write_template_stats`：sqlite_table_name 为 None/空串时跳过；非 None 时先做 ascii_alphanumeric_or_underscore 校验防 SQL 注入，通过后用动态表名替代硬编码 "sql_templates"
- `build_companion_path` 保留为 `pub(crate) #[allow(dead_code)]`
- 新增 csv.rs 测试：`test_csv_write_template_stats_none_skips`、`test_csv_write_template_stats_empty_path_skips`
- 新增 sqlite.rs 测试：`test_sqlite_write_template_stats_none_skips`、`test_sqlite_write_template_stats_empty_table_name_skips`、`test_sqlite_write_template_stats_custom_table`、`test_sqlite_write_template_stats_invalid_name_rejected`
- 更新 mod.rs/csv.rs/sqlite.rs 所有旧签名测试调用（两参数 → 三参数）

**Task 2 — run.rs 调用方迁移（12db626）：**
- 并行路径：移除旧的 `build_companion_path + write_companion_rows` 内联逻辑，改为从 `final_cfg.template.output_csv_path / output_sqlite_table` 派生显式路径；CSV 直接调用 `write_companion_rows`，SQLite 构造新 `SqliteExporter` 实例处理
- 顺序路径：`write_template_stats(stats, None)` → `write_template_stats(stats, csv_out_path, sqlite_table)`，参数从 `final_cfg.template` 派生
- 导入 `Exporter` trait（并行路径调用 `SqliteExporter` 方法时需要）
- 更新 `test_template_stats_enabled_end_to_end_sequential`：TOML 中添加显式 `output_csv_path`，断言该路径文件存在且 header 完整匹配

## Deviations from Plan

**1. [Rule 3 - Blocking] Task 2 在 Task 1 提交中一并完成**
- **Found during:** Task 1 执行后
- **Issue:** run.rs 的两处 `write_template_stats` 调用在修改 exporter 签名时必须同步更新，否则编译失败
- **Fix:** 将 run.rs 的并行路径和顺序路径调用方更新纳入 Task 1 提交，Task 2 专注于 run.rs 额外细节（Exporter trait 导入、测试更新）
- **Files modified:** src/cli/run.rs
- **Commit:** f48afc6, 12db626

## Decisions Made

1. `build_companion_path` 保留为 `pub(crate) #[allow(dead_code)]` — 生产代码已不使用，但保留接口稳定性
2. SqliteExporter 注入防护仅做轻量 ascii_alphanumeric_or_underscore 校验 — 完整的 Config.validate() 层校验由 Plan 03 补强
3. 并行路径绕过 ExporterManager 直接调用底层函数 — 避免重建完整 ExporterManager 实例的开销

## Known Stubs

无 — 所有路径均有真实输出逻辑，无占位数据。

## Threat Flags

**T-18-05 已完全缓解：** `SqliteExporter::write_template_stats` 在拼接 sqlite_table_name 到 DDL 前做 ascii_alphanumeric_or_underscore 校验，非法字符返回 `ConfigError::InvalidValue`，阻止 SQL 注入。测试 `test_sqlite_write_template_stats_invalid_name_rejected` 验证了 `"bad name;DROP"` 被拒绝。

## Self-Check: PASSED

- FOUND: src/exporter/mod.rs (write_template_stats 新签名)
- FOUND: src/exporter/csv.rs (CsvExporter 显式路径逻辑)
- FOUND: src/exporter/sqlite.rs (SqliteExporter 动态表名 + 注入防护)
- FOUND: src/cli/run.rs (两条路径显式路径传入)
- FOUND commit f48afc6 (Task 1)
- FOUND commit 12db626 (Task 2)
- cargo build --release 退出码 0
- cargo clippy --all-targets -- -D warnings 零警告
- cargo test --lib 437 passed, 0 failed
- grep -n "final_path" exporter/mod.rs csv.rs sqlite.rs: NOT FOUND (旧参数名已移除)
- grep -n "csv_output_path|sqlite_table_name" exporter/mod.rs: 命中 8 处 (新签名存在)
