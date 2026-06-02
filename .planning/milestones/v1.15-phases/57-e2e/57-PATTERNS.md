# Phase 57: e2e 测试扩展 - Pattern Map

**Mapped:** 2026-06-02
**Files analyzed:** 2 (1 修改 src 文件 + 1 修改测试文件)
**Analogs found:** 2 / 2

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/stats/config.rs` | utility / validation | request-response | `src/stats/config.rs` 自身（`validate_time_str` + `ConfigError::InvalidValue` 用法） | exact |
| `tests/integration.rs` | test | request-response | `tests/integration.rs` 行 1818–1833（`test_cli_stats_runtime_rejects_bad_cli_from_format`）、行 1838–1873（`test_init_template_contains_stats_section`）、行 1464–1474（`make_stats_csv_config`） | exact |

## Pattern Assignments

### `src/stats/config.rs` — `validate_stats_time_range` 跨字段检查（D-01 / D-02）

**Analog:** `src/stats/config.rs` 自身（行 17–37）

**现有函数完整结构**（行 17–37）：
```rust
pub fn validate_stats_time_range(stats: &StatsConfig) -> crate::error::Result<()> {
    if let Some(from) = &stats.from {
        validate_time_str(from).map_err(|reason| {
            Error::Config(ConfigError::InvalidValue {
                field: "stats.from".to_string(),
                value: from.clone(),
                reason,
            })
        })?;
    }
    if let Some(to) = &stats.to {
        validate_time_str(to).map_err(|reason| {
            Error::Config(ConfigError::InvalidValue {
                field: "stats.to".to_string(),
                value: to.clone(),
                reason,
            })
        })?;
    }
    Ok(())
}
```

**插入点：** 在最后一个 `if let Some(to)` 块与 `Ok(())` 之间，新增以下代码块：
```rust
if let (Some(from), Some(to)) = (&stats.from, &stats.to) {
    // YYYY-MM-DD 字典序 == 日期序，字符串比较合法（D-01）
    if from.as_str() > to.as_str() {
        return Err(Error::Config(ConfigError::InvalidValue {
            field: "stats.from".to_string(),
            value: from.clone(),
            reason: format!("stats.from ({from}) must be <= stats.to ({to})"),
        }));
    }
}
```

**`ConfigError::InvalidValue` 变体签名**（`src/error.rs` 行 184–189）：
```rust
#[error("Invalid configuration value {field} = '{value}': {reason}")]
InvalidValue {
    field: String,
    value: String,
    reason: String,
},
```

---

### `tests/integration.rs` — 新增辅助函数 + 5 个测试（TEST-01 / TEST-02 / TEST-03）

**Analog:** `tests/integration.rs` 现有测试基础设施

#### 辅助函数模式：`write_run_config_toml`（仿照 `make_stats_csv_config`）

**参考原型**（行 1464–1474）：
```rust
fn make_stats_csv_config(dir: &std::path::Path, log_path: &std::path::Path) -> std::path::PathBuf {
    let cfg_path = dir.join("stats_csv.toml");
    let csv_path = dir.join("out").join("data.csv");
    let content = format!(
        "[sqllog]\ninputs = [\"{}\"]\n[exporter.csv]\nfile = \"{}\"\noverwrite = true\n",
        log_path.to_string_lossy().replace('\\', "/"),
        csv_path.to_string_lossy().replace('\\', "/"),
    );
    std::fs::write(&cfg_path, content).unwrap();
    cfg_path
}
```

**关键差异 — `write_run_config_toml` 用目录作为 `inputs`（Pitfall 2）：**
- `make_stats_csv_config` 传的是单个文件路径
- `write_run_config_toml` 的 `log_dir` 参数传目录路径（SqllogParser 会扫目录）

**新函数模式（CSV 版本）：**
```rust
fn write_run_config_toml(
    dir: &std::path::Path,
    log_dir: &std::path::Path,
    csv_output: &std::path::Path,
) -> std::path::PathBuf {
    let cfg_path = dir.join("run_config.toml");
    let content = format!(
        "[sqllog]\ninputs = [\"{}\"]\n[exporter.csv]\nfile = \"{}\"\noverwrite = true\n",
        log_dir.to_string_lossy().replace('\\', "/"),
        csv_output.to_string_lossy().replace('\\', "/"),
    );
    std::fs::write(&cfg_path, content).unwrap();
    cfg_path
}
```

**新函数模式（SQLite 版本，仿照 `make_stats_sqlite_config` 行 1477–1490）：**
```rust
fn write_run_sqlite_config_toml(
    dir: &std::path::Path,
    log_dir: &std::path::Path,
    db_output: &std::path::Path,
) -> std::path::PathBuf {
    let cfg_path = dir.join("run_sqlite_config.toml");
    let content = format!(
        "[sqllog]\ninputs = [\"{}\"]\n[exporter.sqlite]\ndatabase_url = \"{}\"\n",
        log_dir.to_string_lossy().replace('\\', "/"),
        db_output.to_string_lossy().replace('\\', "/"),
    );
    std::fs::write(&cfg_path, content).unwrap();
    cfg_path
}
```

**SQLite 表名确认（Pitfall 1 解决方案）：**
- `src/config/mod.rs` 行 117 的单元测试断言：`assert_eq!(cfg.table_name, "sqllog_records")`
- `src/exporter/sqlite/tests.rs` 行 47：`SELECT COUNT(*) FROM sqllog_records`
- **结论：** 实际表名是 `sqllog_records`，而非 CONTEXT.md D-07 所写的 `sqllog`。
- **处置方式（两选一）：** (a) 在 `write_run_sqlite_config_toml` 生成的 TOML 中加 `table_name = "sqllog_records"` 并在测试中用此名；(b) 默认不指定 `table_name`，测试中用 `sqllog_records` 查询。推荐方式 (b)，保持 config 极简。

#### TEST-01 — run CLI CSV 全链路测试

**参考模板**（行 1838–1873，`test_init_template_contains_stats_section` 的 assert_cmd 风格）：
```rust
#[test]
fn test_cli_run_csv_output_header_and_row_count() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    let record_count = 10usize;
    write_test_log(&log_dir.join("test.log"), record_count);

    let csv_file = dir.path().join("out.csv");
    let cfg_path = write_run_config_toml(dir.path(), &log_dir, &csv_file);

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["run", "-c"])
        .arg(&cfg_path)
        .assert()
        .success();

    let content = std::fs::read_to_string(&csv_file).unwrap();
    let mut lines = content.lines();
    assert_eq!(
        lines.next().unwrap(),
        "ts,ep,sess_id,thrd_id,username,trx_id,statement,appname,client_ip,tag,sql,exec_time_ms,row_count,exec_id,normalized_sql",
        "CSV header must match FIELD_NAMES order"
    );
    let data_count = lines.filter(|l| !l.is_empty()).count();
    assert_eq!(data_count, record_count, "row count must match written records");
}
```

**CSV 验证模式参考**（行 1519–1528，`test_stats_csv_outputs_two_files`）：
```rust
let content = std::fs::read_to_string(out_dir.join("slow_sql.csv")).unwrap();
assert_eq!(
    content.lines().next().unwrap(),
    "sql_text,elapsed_ms,timestamp"
);
// 行数计算方式（行 1903–1904）：
let data_lines: Vec<&str> = content.lines().skip(1).filter(|l| !l.is_empty()).collect();
assert_eq!(data_lines.len(), expected_count, "...");
```

#### TEST-01 — run CLI SQLite 全链路测试

**rusqlite 查询模式**（`src/exporter/sqlite/tests.rs` 行 44–49）：
```rust
let conn = rusqlite::Connection::open(&dbfile).unwrap();
let count: i64 = conn
    .query_row("SELECT COUNT(*) FROM sqllog_records", [], |r| r.get(0))
    .unwrap();
assert_eq!(count, 5);
```

**注意：** 实际表名为 `sqllog_records`，不是 `sqllog`（见上文表名确认）。

#### TEST-02 — init CLI 测试

**参考模板**（行 1838–1873，`test_init_template_contains_stats_section`）：
```rust
#[test]
fn test_init_template_contains_stats_section() {
    use assert_cmd::Command;
    let dir = tempfile::tempdir().unwrap();
    let out_file = dir.path().join("cfg.toml");

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["init", "-o"])
        .arg(&out_file)
        .args(["--force"])
        .assert()
        .success();

    let content = std::fs::read_to_string(&out_file).unwrap();
    assert!(content.contains("[stats]"), "...");
}
```

**`--force` 标志注意事项（D-08）：**
- 测试 1（新文件）：不加 `--force`，期望 `.success()`
- 测试 2（已存在文件）：不加 `--force`，期望 `.failure()`

**failure + stderr 验证模式**（行 810–840，`test_cli_error_uses_hint_prefix`，但用了旧式 `std::process::Command`）：

更新式写法应参考（行 1818–1833）的 assert_cmd 风格：
```rust
Command::cargo_bin("sqllog2db")
    .unwrap()
    .args(["stats", "-c"])
    .arg(&cfg_path)
    .args(["--from", "not-a-date"])
    .assert()
    .failure()
    .stderr(contains("stats.from"))
    .stderr(contains("YYYY-MM-DD"));
```

**D-08 已存在文件测试 stderr 匹配：**
- `FileError::AlreadyExists` 的 Display 消息（`src/error.rs` 行 197）：`"File already exists: {path} (set overwrite=true to replace)"`
- 建议匹配 `contains("already exists")` 或 `contains("[CRITICAL]")`（stderr 格式在 `test_cli_error_uses_hint_prefix` 中已确认）

#### TEST-03 — stats from > to 边界条件

**参考模板**（行 1818–1833，`test_cli_stats_runtime_rejects_bad_cli_from_format`）：
```rust
fn test_cli_stats_runtime_rejects_bad_cli_from_format() {
    use assert_cmd::Command;
    use predicates::str::contains;

    let dir = tempfile::tempdir().unwrap();
    let cfg_path = make_stats_config_file(dir.path());

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["stats", "-c"])
        .arg(&cfg_path)
        .args(["--from", "not-a-date"])
        .assert()
        .failure()
        .stderr(contains("stats.from"))
        .stderr(contains("YYYY-MM-DD"));
}
```

**新测试同结构，替换参数：**
```rust
.args(["--from", "2024-01-31", "--to", "2024-01-01"])
.assert()
.failure()
.stderr(contains("stats.from"))
.stderr(contains("must be <=").or(contains("2024-01-31")));
```

---

## Shared Patterns

### assert_cmd 调用约定
**Source:** `tests/integration.rs` 行 1501–1507（`test_stats_csv_outputs_two_files`）
**Apply to:** 所有 CLI 测试函数
```rust
use assert_cmd::Command;
Command::cargo_bin("sqllog2db")
    .unwrap()
    .args([/* subcommand, flags */])
    .arg(&path_arg)  // PathBuf 参数单独传，避免类型转换
    .assert()
    .success();      // 或 .failure()
```

### tempfile 目录隔离
**Source:** `tests/integration.rs` 行 1496（`test_stats_csv_outputs_two_files`）
**Apply to:** 所有集成测试函数
```rust
let dir = tempfile::TempDir::new().unwrap();
// dir 在函数末尾 Drop 时自动清理
```

### predicates::str::contains + 链式 stderr 断言
**Source:** `tests/integration.rs` 行 1820（`test_cli_stats_runtime_rejects_bad_cli_from_format`）
**Apply to:** 所有期望非零退出 + stderr 验证的测试
```rust
use predicates::str::contains;
// 单个条件：
.stderr(contains("stats.from"))
// OR 条件（predicates trait）：
.stderr(contains("must be <=").or(contains("2024-01-31")))
```

### write_test_log helper 复用
**Source:** `tests/integration.rs` 行 16–29
**Apply to:** TEST-01 两个 run CLI 测试
```rust
// 已存在，直接调用：
write_test_log(&log_dir.join("test.log"), record_count);
// 生成真实格式的达梦 SQL 日志行，record_count 条
```

### 路径 Windows 兼容转义
**Source:** `tests/integration.rs` 行 1469（`make_stats_csv_config`）
**Apply to:** 所有 config 生成辅助函数
```rust
log_path.to_string_lossy().replace('\\', "/")
```

### ConfigError::InvalidValue 构造
**Source:** `src/stats/config.rs` 行 19–25
**Apply to:** `validate_stats_time_range` 的新检查分支
```rust
Error::Config(ConfigError::InvalidValue {
    field: "stats.from".to_string(),
    value: from.clone(),
    reason: format!("..."),
})
```

---

## No Analog Found

无——所有文件均有精确或近似匹配的现有代码。

---

## Critical Notes for Planner

1. **SQLite 表名：** `sqllog_records`（不是 `sqllog`）。测试中的 SQL 查询应为 `SELECT COUNT(*) FROM sqllog_records`。
2. **write_run_config_toml 的 inputs 传目录：** `SqllogParser` 扫目录，`inputs` 填目录路径，不是 `.log` 文件路径。
3. **实现顺序：** 先改 `src/stats/config.rs`（TEST-03 前提），确认现有测试通过后再写 5 个新测试。
4. **测试文件在行 1940 结尾：** 所有新测试追加到末尾，在最后一个测试函数后面。

## Metadata

**Analog search scope:** `src/stats/config.rs`, `src/error.rs`, `src/exporter/sqlite/tests.rs`, `tests/integration.rs`
**Files scanned:** 5
**Pattern extraction date:** 2026-06-02
