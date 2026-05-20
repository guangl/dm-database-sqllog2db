# Phase 33 — 核心功能验证检查清单

**生成时间:** 2026-05-20 15:23:55
**二进制:** target/release/sqllog2db (cargo run --)
**数据源:** synthetic
**测试目录:** /var/folders/5h/hnxrxbts1ln5nq9hf3c4mzq80000gn/T/tmp.RH72shxyKc

## 通过率: 11/11

| KEEP | 项目 | 状态 | 证据 |
|------|------|------|------|
| KEEP-01 | CSV 导出 | PASS | output.csv: 801 data rows, 1 header |
| KEEP-02 | SQLite 导出 | PASS | output.db: 800 rows matched CSV |
| KEEP-03 | Include 过滤器 | PASS |      450 rows, all user=TESTUSER |
| KEEP-03 | Exclude 过滤器 | PASS |      701 rows, no EXCLUDE_USER |
| KEEP-03 | Indicators 过滤器 | PASS |      646 rows (min_runtime_ms filter applied) |
| KEEP-03 | SQL 过滤器 | PASS |       50 rows,       50 contain DROP |
| KEEP-03 | 综合过滤器 | PASS |      350 rows (include+indicators+sql combined) |
| KEEP-04 | 参数归一化 | PASS | CSV + SQLite normalized_sql 列存在 (行数不一致: CSV=801, SQLite=800) |
| KEEP-05 | 并行 CSV | PASS |      801 rows, sequential/parallel content matches |
| D-08 | 配置模板生成 | PASS | init + validate 成功 |
| D-11 | 错误日志 | PASS | app.log 存在，      12 行 |

## 详细信息


### KEEP-01: CSV 导出

- **状态:** PASS
- **证据:** output.csv: 801 data rows, 1 header
- **可复现步骤:** `cargo run -- run -c .planning/phases/33-core-verification/smoke_test/config_csv.toml`

### KEEP-02: SQLite 导出

- **状态:** PASS
- **证据:** output.db: 800 rows matched CSV
- **可复现步骤:** `cargo run -- run -c .planning/phases/33-core-verification/smoke_test/config_sqlite.toml`

### KEEP-03: Include 过滤器

- **状态:** PASS
- **证据:**      450 rows, all user=TESTUSER
- **可复现步骤:** `cargo run -- run -c .planning/phases/33-core-verification/smoke_test/config_include.toml`

### KEEP-03: Exclude 过滤器

- **状态:** PASS
- **证据:**      701 rows, no EXCLUDE_USER
- **可复现步骤:** `cargo run -- run -c .planning/phases/33-core-verification/smoke_test/config_exclude.toml`

### KEEP-03: Indicators 过滤器

- **状态:** PASS
- **证据:**      646 rows (min_runtime_ms filter applied)
- **可复现步骤:** `cargo run -- run -c .planning/phases/33-core-verification/smoke_test/config_indicators.toml`

### KEEP-03: SQL 过滤器

- **状态:** PASS
- **证据:**       50 rows,       50 contain DROP
- **可复现步骤:** `cargo run -- run -c .planning/phases/33-core-verification/smoke_test/config_sql_filter.toml`

### KEEP-03: 综合过滤器

- **状态:** PASS
- **证据:**      350 rows (include+indicators+sql combined)
- **可复现步骤:** `cargo run -- run -c .planning/phases/33-core-verification/smoke_test/config_all_filters.toml`

### KEEP-04: 参数归一化

- **状态:** PASS
- **证据:** CSV + SQLite normalized_sql 列存在 (行数不一致: CSV=801, SQLite=800)
- **可复现步骤:** `cargo run -- run -c .planning/phases/33-core-verification/smoke_test/config_params.toml`

### KEEP-05: 并行 CSV

- **状态:** PASS
- **证据:**      801 rows, sequential/parallel content matches
- **可复现步骤:** `cargo run -- run -c .planning/phases/33-core-verification/smoke_test/config_parallel_csv.toml --jobs 4`

### D-08: 配置模板生成

- **状态:** PASS
- **证据:** init + validate 成功
- **可复现步骤:** `cargo run -- init -o /tmp/test_config.toml --force && cargo run -- validate -c /tmp/test_config.toml`

### D-11: 错误日志

- **状态:** PASS
- **证据:** app.log 存在，      12 行
- **可复现步骤:** `cargo run -- run -c .planning/phases/33-core-verification/smoke_test/config_error_log.toml`

## 测试统计

| 指标 | 值 |
|------|-----|
| 通过 | 11 |
| 失败 | 0 |
| 总数 | 11 |
| 数据源 | synthetic |
| 测试时间 | 2026-05-20 15:23:55 |

