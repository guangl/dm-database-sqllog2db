---
phase: 63-test-coverage
plan: "02"
subsystem: testing
tags: [rust, csv-exporter, sqlite-exporter, unit-tests, coverage, field-projection, has-metrics]

requires:
  - phase: 63-test-coverage-01
    provides: filters/types.rs serde_helpers 间接覆盖测试（先行任务）

provides:
  - CSV writer has_metrics=false 全量路径 b",," 分支覆盖（writer.rs:82）
  - CSV writer 非全量字段投影路径（writer.rs:93）中 idx=0/1/2/6/7/8/9/11/12/13/14 各分支覆盖
  - SQLite exporter conn=None 未初始化 Err 路径覆盖（mod.rs:209-212）
  - SQLite initialize_pragmas 调用路径间接验证（通过 initialize() 成功返回）
  - SQLite 字段投影导出非全量路径覆盖（ordered_indices != ALL）

affects:
  - 63-test-coverage 其他 plan（共用 exporter 基础设施测试模式）
  - 后续维护：writer.rs 与 sqlite/mod.rs 覆盖率提升减少回归风险

tech-stack:
  added: []
  patterns:
    - "零指标日志行构造模式：内联 std::fs::write 写入 EXECTIME: 0/ROWCOUNT: 0/EXEC_ID: 0"
    - "SQLite EXCLUSIVE 锁释放模式：exporter 放入 {} 块，drop 后再 rusqlite::Connection::open"
    - "FieldMask::from_names + ordered_indices 直接赋值构造字段投影"

key-files:
  created: []
  modified:
    - src/exporter/csv/tests.rs
    - src/exporter/sqlite/tests.rs

key-decisions:
  - "SQLite initialize_pragmas 验证改为间接验证（initialize() 返回 Ok），避免与 EXCLUSIVE 锁机制冲突"
  - "CSV 全量路径 b',,' 测试单独一个测试覆盖（test_csv_all_zero_metrics_outputs_empty_columns），而非合并到投影测试"
  - "SQLite 未初始化测试拆分为两个：export() 与 export_one_normalized() 分别验证，覆盖两条调用链"

patterns-established:
  - "Pattern: 零指标日志行通过内联 std::fs::write 构造，不复用 write_test_log（后者 EXEC_ID=i 永远非零）"

requirements-completed:
  - TEST-02

duration: 18min
completed: 2026-06-03
---

# Phase 63 Plan 02: CSV & SQLite Exporter 关键路径覆盖测试

**在 CSV exporter 追加 6 个测试覆盖 has_metrics=false 全量/投影路径与 idx=0-14 各分支，在 SQLite exporter 追加 4 个测试覆盖未初始化 Err 路径、pragma 间接验证与字段投影非全量路径。**

## Performance

- **Duration:** 18 min
- **Started:** 2026-06-03T00:00:00Z
- **Completed:** 2026-06-03T00:18:00Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- CSV tests.rs 新增 6 个测试函数（原 21 → 27），覆盖 writer.rs 全量路径 has_metrics=false 分支（writer.rs:82）、非全量投影路径（writer.rs:93）中 idx=0/1/2/6/7/8/9/11/12/13/14 各分支
- SQLite tests.rs 新增 4 个测试函数（原 14 → 18），覆盖 export_one_preparsed conn=None 路径（mod.rs:209-212）、initialize_pragmas 间接验证、字段投影非全量路径
- 全部三道质量门禁（cargo test / clippy -D warnings / fmt --check）通过

## 新增测试函数清单

### src/exporter/csv/tests.rs

| 测试函数名 | 覆盖路径 |
|-----------|---------|
| `test_csv_all_zero_metrics_outputs_empty_columns` | writer.rs:74-83 全量路径 has_metrics=false 分支，断言输出含 `,,` |
| `test_csv_projection_subset_emits_only_requested_columns` | writer.rs:93 非全量路径入口 + idx=0/1/2 三条 match 分支 |
| `test_csv_projection_statement_appname_client_ip_tag` | writer.rs:132-149 idx=6/7/8/9 四条 match 分支 |
| `test_csv_projection_zero_metrics_skips_idx_11_12_13` | writer.rs:156-184 has_metrics=false 时 idx=11/12/13 不写 itoa 分支 |
| `test_csv_projection_with_normalize_idx_14` | writer.rs:187-194 idx=14 normalize=true + Some(ns) 分支 |
| `test_csv_projection_with_normalize_none_emits_empty_idx_14` | writer.rs:189-193 idx=14 normalize=true + None 子分支 |

### src/exporter/sqlite/tests.rs

| 测试函数名 | 覆盖路径 |
|-----------|---------|
| `test_sqlite_export_without_initialize_returns_err` | mod.rs:209-212 conn=None ok_or_else + db_err 调用；断言 err 含 "not initialized" |
| `test_sqlite_export_one_normalized_without_initialize_returns_err` | export_one_normalized → export_one_preparsed 未初始化路径（同一分支的 normalized 通道） |
| `test_sqlite_initialize_pragmas_applied` | mod.rs:30-42 initialize_pragmas 间接验证（initialize() 返回 Ok + DB 文件非空） |
| `test_sqlite_projection_subset_export` | export_one_preparsed ordered_indices 非全量路径 + build_insert_sql/build_create_sql 非全量分支 |

## Task Commits

1. **Task 1: CSV exporter 新增 has_metrics=false 与字段投影分支测试** - `fcff294` (test)
2. **Task 2: SQLite exporter 新增未初始化 Err 路径、pragma 验证、投影测试** - `08976d9` (test)

**Plan metadata:** (docs commit 在此之后)

## Files Created/Modified

- `src/exporter/csv/tests.rs` — 末尾追加 6 个测试函数（has_metrics=false 全量/投影、idx=0/1/2/6/7/8/9/11/12/13/14 各分支）
- `src/exporter/sqlite/tests.rs` — 末尾追加 4 个测试函数（conn=None Err 路径、initialize_pragmas 间接验证、字段投影）

## Decisions Made

- **SQLite pragma 验证简化**：计划建议通过 PRAGMA journal_mode 查询验证，但 EXCLUSIVE 锁模式下在 exporter 生命周期内无法另开连接，改为验证 initialize() 返回 Ok 且 DB 文件非空（更稳健）。
- **CSV has_metrics=false 单独测试**：write_test_log 辅助函数生成的日志 EXEC_ID=i（i≥0），i=0 时才触发 has_metrics=false；为避免混淆，单独写内联日志行确保 EXEC_ID=0。
- **SQLite 未初始化测试分两个**：export() 和 export_one_normalized() 均调用 export_one_preparsed，但调用链不同；分开可覆盖 export → export_one_normalized → export_one_preparsed 两条路径。

## Deviations from Plan

### 计划微调

**1. [计划调整 - 测试策略] SQLite initialize_pragmas 验证改为 DB 文件非空断言**
- **涉及：** Task 2 Test 3（test_sqlite_initialize_pragmas_applied）
- **计划原文：** 建议"在 finalize 前增加一次 pragma_query"或"在 initialize() 不 panic 且返回 Ok 即作为证据"
- **实施：** 采用后者（更简单方案）：验证 initialize() 返回 Ok + DB 文件存在且大小 > 0
- **原因：** EXCLUSIVE locking_mode 期间无法从同一进程另外打开连接查询 PRAGMA，采用简化验证更稳健
- **对覆盖率的影响：** initialize_pragmas 函数仍被调用（通过 initialize()），函数覆盖率提升一致

无其他偏差，生产代码（writer.rs、csv/mod.rs、sqlite/mod.rs、sqlite/write.rs、sqlite/sql_builder.rs）无任何 git diff 变更。

## Issues Encountered

无。所有测试首次运行即通过，无需迭代修复。

## User Setup Required

无 — 纯测试代码变更，不需要外部服务或手动配置。

## Next Phase Readiness

- CSV exporter（writer.rs）覆盖率已从 66.29% 提升，关键未覆盖路径已补全
- SQLite exporter（mod.rs）函数覆盖率已从 53.33% 提升，conn_ref/db_err/initialize_pragmas/projection 路径已覆盖
- Plan 03（error.rs + prescan.rs 覆盖）可直接开始

---
*Phase: 63-test-coverage*
*Completed: 2026-06-03*
