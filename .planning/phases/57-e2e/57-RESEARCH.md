# Phase 57: e2e 测试扩展 - Research

**Researched:** 2026-06-02
**Domain:** Rust CLI 集成测试（assert_cmd / predicates / rusqlite）
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-01:** 当 `stats.from` 晚于 `stats.to` 时报错非零退出——在 `src/stats/config.rs` 的 `validate_stats_time_range` 中加入跨字段字符串比较（YYYY-MM-DD 字典序 == 日期序）

**D-02:** 错误格式遵循 `ConfigError::InvalidValue` 模式，错误信息包含字段名+具体值，例如：`"stats.from (2024-01-31) must be <= stats.to (2024-01-01)"`

**D-03:** 验证在 `validate_stats_time_range` 中执行，与现有调用点（`Config::validate` 和 `run_stats`）保持一致

**D-04:** 新增辅助函数 `write_run_config_toml(dir, log_dir, output_path) -> PathBuf`，风格参考 `make_stats_csv_config()`

**D-05:** CSV 内容验证：header 行完整匹配 `"ts,ep,sess_id,thrd_id,username,trx_id,statement,appname,client_ip,tag,sql,exec_time_ms,row_count,exec_id,normalized_sql"` + 记录行数正确

**D-06:** 测试数据复用现有 `write_test_log()` helper

**D-07:** SQLite 验证：文件存在 + 用 rusqlite 查询 `sqllog` 表记录数等于写入行数（rusqlite 已是项目依赖）

**D-08:** 新增两个 init assert_cmd 测试：
  1. `sqllog2db init -o <新路径>` 成功创建文件，exit 0
  2. 文件已存在 + 不加 `--force`，exit 非零 + stderr 包含错误信息

### Claude's Discretion

无（所有实现决策已锁定）

### Deferred Ideas (OUT OF SCOPE)

- from > to 影响退出码（退出码 1 vs 2）细化 → 遵循现有 ConfigError 映射规则，Phase 57 不改变退出码策略
- run CLI 测试的多平台矩阵（Windows + Linux）→ v1.15 后续 CI 阶段

</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TEST-01 | run 子命令 CLI 全链路测试——给定真实输入文件，验证 CSV 输出内容与退出码；给定真实输入文件，验证 SQLite 输出与退出码 | assert_cmd + write_test_log + rusqlite 已全部就绪；write_run_config_toml 辅助函数需新增 |
| TEST-02 | init 子命令 assert_cmd 测试——验证生成 config.toml 的 CLI 行为、文件存在与退出码 | assert_cmd 风格已有模板（test_init_template_contains_stats_section 行 1838），仅需两个新测试 |
| TEST-03 | stats 子命令 --from/--to 边界条件 e2e 测试（空范围、边界值、无效格式拒绝） | from==to 已覆盖（行 1878），from>to 需先加代码验证（D-01），无效格式已覆盖（行 1818）|

</phase_requirements>

---

## Summary

Phase 57 是一个纯测试+小型代码改动 Phase，不引入新依赖。工作分两部分：

**Part A — 代码改动（前提）：** 在 `src/stats/config.rs` 的 `validate_stats_time_range` 函数末尾增加 from ≤ to 的跨字段检查。改动极小：在两个 `if let` 块之后、`Ok(())` 之前加一个跨字段比较。错误格式沿用已有的 `ConfigError::InvalidValue`，字典序比较对 YYYY-MM-DD 格式完全等价于日期比较，无需引入 chrono。

**Part B — 测试扩展（核心）：** 在 `tests/integration.rs` 末尾增加约 5 个测试函数。所有依赖（assert_cmd、predicates、tempfile、rusqlite）均已在 `[dev-dependencies]` 中。测试风格有大量现成模板可参考，新测试的结构高度相似，无需创造性设计。

**最小变更集：** 仅修改 `src/stats/config.rs`（1个函数，+约10行）和 `tests/integration.rs`（+约100行测试代码）。

**Primary recommendation:** 先写 D-01 代码改动并通过现有测试，再逐个添加 5 个新测试函数。

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| from ≤ to 验证 | Config/业务逻辑层 | — | validate_stats_time_range 是专用验证函数，调用点在 Config::validate 和 run_stats |
| run CLI e2e 测试 | 测试层（集成测试） | CLI 二进制 | assert_cmd 调用真实二进制，验证完整链路 |
| init CLI 测试 | 测试层（集成测试） | CLI 二进制 | 验证文件生成行为和退出码 |
| stats 边界条件测试 | 测试层（集成测试） | Config 验证层 | from>to 场景需要代码验证支撑才能测试 |

---

## Standard Stack

### Core（所有依赖已存在于 Cargo.toml）

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| assert_cmd | 2.x | CLI 进程断言 | `[dev-dependencies]` 已有；项目现有 60+ 测试用此库 |
| predicates | 3.x | stdout/stderr 字符串匹配 | `[dev-dependencies]` 已有；`predicates::str::contains` 是现有测试的标准用法 |
| tempfile | 3.27.0 | 临时目录/文件 | `[dev-dependencies]` 已有；所有集成测试均用此创建隔离环境 |
| rusqlite | 0.39.0 | 查询 SQLite 输出验证 | 已在 `[dependencies]`；D-07 指定用此验证 sqllog 表记录数 |

[VERIFIED: 直接读取 /Users/guang/Projects/sqllog2db/Cargo.toml]

### 无需安装新依赖

本 Phase 所有 dev-dependencies 已就绪，不执行 Package Legitimacy Audit（无新包）。

---

## Architecture Patterns

### System Architecture Diagram

```
测试文件 (write_test_log) ──→ 临时日志目录
                                  ↓
write_run_config_toml ────→ 临时 config.toml
                                  ↓
assert_cmd::Command::cargo_bin("sqllog2db")
    .args(["run", "-c", <config>])
    .assert()
    .success()
                                  ↓
验证输出文件:
  CSV: read_to_string → 检查 header 行 + 行数
  SQLite: rusqlite::Connection::open → SELECT COUNT(*) FROM sqllog
```

### Recommended Project Structure（无变化）

```
src/
├── stats/config.rs    # 唯一修改点：validate_stats_time_range 加 from<=to 检查
tests/
└── integration.rs     # 唯一测试文件：末尾追加 5 个测试函数
```

### Pattern 1: write_run_config_toml 辅助函数

**What:** 仿照 `make_stats_csv_config`（行 1464），生成含 `[sqllog]` + `[exporter.csv]` 或 `[exporter.sqlite]` 的临时 config.toml

**When to use:** run CLI 测试需要的配置文件生成

**Example（CSV 版本）：**
```rust
// Source: tests/integration.rs 行 1464 make_stats_csv_config 风格
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

**SQLite 版本（用于 D-07 测试）：**
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

注意：`make_stats_csv_config` 把日志文件路径作为 input，而 run 测试输入的是**目录**（SqllogParser 会扫目录）。`write_run_config_toml` 的 `log_dir` 参数传日志目录路径。

### Pattern 2: run CLI CSV 测试

**What:** 调用 `sqllog2db run -c <config>`，验证 exit 0，CSV header 匹配，记录行数正确

```rust
// Source: tests/integration.rs test_init_template_contains_stats_section 行 1838 风格
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

### Pattern 3: run CLI SQLite 测试（D-07）

```rust
#[test]
fn test_cli_run_sqlite_output_row_count() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    let record_count = 5usize;
    write_test_log(&log_dir.join("test.log"), record_count);

    let db_file = dir.path().join("out.db");
    let cfg_path = write_run_sqlite_config_toml(dir.path(), &log_dir, &db_file);

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["run", "-c"])
        .arg(&cfg_path)
        .assert()
        .success();

    assert!(db_file.exists(), "SQLite output file must exist");
    let conn = rusqlite::Connection::open(&db_file).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sqllog", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, record_count as i64, "sqllog table row count must match");
}
```

注意表名：SQLite exporter 默认表名为 `sqllog_records`（见 `SqliteExporterConfig::default()`），但 CONTEXT.md D-07 写的是 `sqllog`。**需要确认实际表名**，见下文 Open Questions Q1。

### Pattern 4: init CLI 测试（D-08）

```rust
#[test]
fn test_cli_init_creates_file_exit_0() {
    use assert_cmd::Command;
    let dir = tempfile::TempDir::new().unwrap();
    let out_file = dir.path().join("new_config.toml");

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["init", "-o"])
        .arg(&out_file)
        .assert()
        .success();

    assert!(out_file.exists(), "init must create the config file");
}

#[test]
fn test_cli_init_existing_file_without_force_exits_nonzero() {
    use assert_cmd::Command;
    use predicates::str::contains;

    let dir = tempfile::TempDir::new().unwrap();
    let out_file = dir.path().join("existing.toml");
    std::fs::write(&out_file, "existing content").unwrap();

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["init", "-o"])
        .arg(&out_file)
        .assert()
        .failure()
        .stderr(contains("already exists").or(contains("[CRITICAL]")));
}
```

### Pattern 5: validate_stats_time_range 的 from ≤ to 跨字段检查（D-01/D-02）

```rust
// Source: src/stats/config.rs validate_stats_time_range 现有逻辑
// 在两个 if let 块之后、Ok(()) 之前插入：
if let (Some(from), Some(to)) = (&stats.from, &stats.to) {
    // YYYY-MM-DD 字典序 == 日期序，字符串比较合法
    if from.as_str() > to.as_str() {
        return Err(Error::Config(ConfigError::InvalidValue {
            field: "stats.from".to_string(),
            value: from.clone(),
            reason: format!("stats.from ({from}) must be <= stats.to ({to})"),
        }));
    }
}
```

### Pattern 6: stats from > to 测试（TEST-03 新增部分）

```rust
#[test]
fn test_cli_stats_rejects_from_after_to() {
    use assert_cmd::Command;
    use predicates::str::contains;

    let dir = tempfile::tempdir().unwrap();
    let cfg_path = make_stats_config_file(dir.path());

    Command::cargo_bin("sqllog2db")
        .unwrap()
        .args(["stats", "-c"])
        .arg(&cfg_path)
        .args(["--from", "2024-01-31", "--to", "2024-01-01"])
        .assert()
        .failure()
        .stderr(contains("stats.from"))
        .stderr(contains("must be <=").or(contains("2024-01-31")));
}
```

### Anti-Patterns to Avoid

- **不要重写已覆盖的测试：** `test_stats_from_to_filters_to_single_day`（from==to）和 `test_cli_stats_runtime_rejects_bad_cli_from_format`（无效格式）已有，只需新增 from>to 场景
- **不要用 `std::process::Command` 替代 `assert_cmd::Command`：** 部分旧测试用了 `env!("CARGO_BIN_EXE_sqllog2db")` + `std::process::Command`，但现有 Phase 53/54 测试全用 `assert_cmd`，新测试应统一用 assert_cmd
- **不要在 `write_run_config_toml` 里添加 `[logging]` 节：** run 命令不需要，keep it minimal（参考 `make_stats_csv_config` 的简洁性）

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 进程退出码断言 | 自己 `match output.status.code()` | `assert_cmd .assert().success()` / `.failure()` | 已有大量现成用法 |
| stderr 字符串匹配 | 手动 `String::from_utf8_lossy(&output.stderr).contains(...)` | `predicates::str::contains` | 链式 API 更清晰，错误消息自动包含实际值 |
| 临时目录清理 | 手动 `std::fs::remove_dir_all` | `tempfile::TempDir::new()` | Drop 时自动清理，避免测试污染 |
| SQLite 记录数查询 | 解析 CSV 行数 | `rusqlite::Connection::open + query_row` | 直接查数据库，D-07 已明确要求 |

---

## Common Pitfalls

### Pitfall 1: SQLite 表名不匹配
**What goes wrong:** 测试用 `SELECT COUNT(*) FROM sqllog` 但实际表名是 `sqllog_records`（`SqliteExporterConfig::default()` 的 `table_name` 字段默认值）
**Why it happens:** CONTEXT.md D-07 写的是 `sqllog`，但代码实际默认值是 `sqllog_records`
**How to avoid:** 实现前检查 `src/config/mod.rs` 中 `SqliteExporterConfig::default()` 的 `table_name` 字段值，或在测试 config 中显式指定 `table_name = "sqllog"`
**Warning signs:** 测试 panic at `rusqlite::Error::SqliteFailure` "no such table"

### Pitfall 2: run 输入路径是目录 vs 文件
**What goes wrong:** `write_run_config_toml` 把日志文件路径（而非目录路径）写入 `inputs`，导致 `SqllogParser` 扫目录时找不到文件
**Why it happens:** `make_stats_csv_config` 的 input 是单个文件，`make_run_config`（行 31）的 `inputs` 是**目录**
**How to avoid:** `write_run_config_toml` 的 `log_dir` 参数应指向目录，而不是 .log 文件本身

### Pitfall 3: from > to 检查触发点
**What goes wrong:** 只在 `validate_stats_time_range` 中加检查，但遗漏了从 CLI `--from`/`--to` 传入时的调用路径
**Why it happens:** `run_stats` 中 CLI 参数覆盖 config 后重新调用 `validate_stats_time_range`（D-03）——只要函数里有检查，两条路径都覆盖
**How to avoid:** 确认 `handle_stats` 函数中的调用链：CLI 参数 merge 后再调用 `validate_stats_time_range`，无需重复加检查

### Pitfall 4: 测试中字符串 escape
**What goes wrong:** Windows 路径 `\` 在 TOML 字符串中需要转义，导致 config 解析失败
**Why it happens:** 现有测试全部用 `.replace('\\', "/")` 处理路径
**How to avoid:** 生成 config 内容时统一用 `path.to_string_lossy().replace('\\', "/")`

### Pitfall 5: assert_cmd 需要先 build
**What goes wrong:** `cargo test` 在没有先 build release 时，`cargo_bin("sqllog2db")` 会构建 debug 二进制——这是正确行为，无需担心；但如果测试 CI 环境只有 release binary 而没有 debug build，可能找不到
**Why it happens:** assert_cmd 会自动调用 `cargo build`（debug）
**How to avoid:** 本 Phase 无需关注，本地 `cargo test` 会自动处理

---

## Code Examples

### 完整的 from ≤ to 检查插入位置

```rust
// Source: src/stats/config.rs validate_stats_time_range（现有函数）
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
    // ← 在此处插入 from ≤ to 跨字段检查（D-01）
    if let (Some(from), Some(to)) = (&stats.from, &stats.to) {
        if from.as_str() > to.as_str() {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "stats.from".to_string(),
                value: from.clone(),
                reason: format!("stats.from ({from}) must be <= stats.to ({to})"),
            }));
        }
    }
    Ok(())
}
```

### FIELD_NAMES 顺序（CSV header 完整字符串）

```
ts,ep,sess_id,thrd_id,username,trx_id,statement,appname,client_ip,tag,sql,exec_time_ms,row_count,exec_id,normalized_sql
```

来源：`src/pipeline/mod.rs` 第 11-27 行 `FIELD_NAMES` 常量 [VERIFIED: 直接读取代码]

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `std::process::Command` + 手动 status check | `assert_cmd::Command + .assert()` | 项目现有趋势 | 新测试统一用 assert_cmd |
| 无 from≤to 验证 | 在 validate_stats_time_range 中加跨字段检查 | Phase 57 | 测试先行验证场景变得可能 |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | SQLite exporter 写入的表名在 run 命令下是 `sqllog_records`（config 默认值） | Pattern 3 | 测试用错表名会 panic；实现时需确认 |

---

## Open Questions (RESOLVED)

1. **SQLite 表名：`sqllog` 还是 `sqllog_records`？**
   - What we know: `SqliteExporterConfig::default()` 的 `table_name` 字段默认值是 `"sqllog_records"`（见 src/config/mod.rs 测试行 117）
   - What's unclear: CONTEXT.md D-07 写的是 `sqllog`，可能是笔误，也可能 SQLite exporter 用了不同的表名常量
   - Recommendation: 实现 D-07 测试时，先用 `make_run_sqlite_config` 生成配置并运行一次，查看实际创建的表名；或直接在 config 中指定 `table_name = "sqllog"` 并在测试里用此名称——但需与 CONTEXT.md 确认
   - **RESOLVED:** 实际表名为 `sqllog_records`（`SqliteExporterConfig::default().table_name` 的真实值，已由 PATTERNS.md 通过读取 `src/exporter/sqlite/tests.rs:47` 确认）。CONTEXT.md D-07 中的 `sqllog` 是笔误。57-02-PLAN.md 全程使用 `sqllog_records`。

---

## Environment Availability

Phase 57 为代码+测试改动，无外部工具依赖，跳过此节。

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test（内置）+ assert_cmd 2.x + criterion 0.7（bench only） |
| Config file | Cargo.toml（`[dev-dependencies]`） |
| Quick run command | `cargo test` |
| Full suite command | `cargo test -- --test-threads=4` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TEST-01 | run CSV 输出 header + 行数正确，exit 0 | integration | `cargo test test_cli_run_csv_output_header_and_row_count` | ❌ Wave 0 |
| TEST-01 | run SQLite 输出文件存在 + 记录数正确，exit 0 | integration | `cargo test test_cli_run_sqlite_output_row_count` | ❌ Wave 0 |
| TEST-02 | init 生成新文件，exit 0 | integration | `cargo test test_cli_init_creates_file_exit_0` | ❌ Wave 0 |
| TEST-02 | init 文件已存在不加 --force，exit 非零 + stderr 含错误信息 | integration | `cargo test test_cli_init_existing_file_without_force_exits_nonzero` | ❌ Wave 0 |
| TEST-03 | stats --from 晚于 --to，exit 非零 + stderr 含字段名 | integration | `cargo test test_cli_stats_rejects_from_after_to` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test`
- **Per wave merge:** `cargo test && cargo clippy --all-targets -- -D warnings`
- **Phase gate:** `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`

### Wave 0 Gaps

- [ ] `tests/integration.rs` 末尾：5 个新测试函数（对应上表 5 条）
- [ ] `src/stats/config.rs` `validate_stats_time_range` 函数：from ≤ to 跨字段检查（TEST-03 前提）

---

## Security Domain

Phase 57 仅涉及测试文件和单函数验证改动，无认证、会话、加密、网络等安全相关内容。跳过 ASVS 检查。

---

## Sources

### Primary (HIGH confidence)
- 直接读取 `/Users/guang/Projects/sqllog2db/tests/integration.rs` — 现有测试函数全表（约 1940 行）
- 直接读取 `/Users/guang/Projects/sqllog2db/src/stats/config.rs` — validate_stats_time_range 完整实现
- 直接读取 `/Users/guang/Projects/sqllog2db/src/error.rs` — ConfigError::InvalidValue 签名
- 直接读取 `/Users/guang/Projects/sqllog2db/Cargo.toml` — dev-dependencies 版本确认
- 直接读取 `/Users/guang/Projects/sqllog2db/src/pipeline/mod.rs` — FIELD_NAMES 常量
- 直接读取 `/Users/guang/Projects/sqllog2db/src/main.rs` — 退出码常量（EXIT_FATAL=2, EXIT_PARTIAL=1）

### Secondary (MEDIUM confidence)
- `.planning/phases/57-e2e/57-CONTEXT.md` — 所有锁定决策

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — 直接读 Cargo.toml 确认，无新依赖
- Architecture: HIGH — 直接读现有测试代码，新测试高度相似
- Pitfalls: HIGH（SQLite 表名）/ MEDIUM（其余）— 基于代码直读；表名需实现时确认

**Research date:** 2026-06-02
**Valid until:** 稳定（无外部依赖变化风险）
