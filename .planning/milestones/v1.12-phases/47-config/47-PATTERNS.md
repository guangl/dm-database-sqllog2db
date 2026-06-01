# Phase 47: 配置文件体验 - Pattern Map

**Mapped:** 2026-05-31
**Files analyzed:** 2 (modified files only)
**Analogs found:** 2 / 2

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/cli/validate.rs` | cli-handler | request-response | `src/preflight.rs` (print_and_check) + `src/main.rs` (format_error_output) | role-match |
| `src/cli/init.rs` | cli-handler / config-template | transform | self (existing `CONFIG_TEMPLATE_EN`) | exact |

## Pattern Assignments

### `src/cli/validate.rs` (cli-handler, request-response)

**目标变更：** 将当前所有 `log::info!()` 调用替换为直接 `println!` 输出（D-01 简化方案）。全部通过输出单行 `Configuration valid.`；有失败项输出 `[FAIL] field: reason\n  hint: ...` 格式。

**当前代码（完整，需重写）** (`src/cli/validate.rs` lines 1-79)：
```rust
use crate::config::Config;
use log::info;

pub fn handle_validate(cfg: &Config) {
    info!("SQL log input path: {}", cfg.sqllog.path);
    info!("Log level: {}", cfg.logging.level);
    // ... 所有输出均走 log::info!
}
```

**Analog 1 — 直接 stdout/stderr 输出模式** (`src/preflight.rs` lines 96-106)：
```rust
pub(crate) fn print_and_check(&self) -> bool {
    for warn in &self.warnings {
        eprintln!("Warning: {warn}");
    }
    for err in &self.errors {
        eprintln!("Error: {err}");
    }
    self.has_errors()
}
```
提取要点：用 `eprintln!` 逐条输出，不走日志系统。本 phase 的 validate 输出用 `println!` 走 stdout。

**Analog 2 — `[SEVERITY] message\n  hint: text` 格式** (`src/main.rs` lines 62-70)：
```rust
fn format_error_output(error: &Error) -> String {
    let severity = error.severity();
    let hint = error.suggestion();
    if hint.is_empty() {
        format!("[{severity}] {error}")
    } else {
        format!("[{severity}] {error}\n  hint: {hint}")
    }
}
```
提取要点：`[TAG] message` 首行 + `  hint: ...` 第二行（两空格缩进），这是 Phase 46 确立的项目输出规范。`[FAIL]` 对应此处的 `[{severity}]` 位置。

**新 handle_validate 输出规范（据 D-02 / D-03）：**

```rust
// 全部通过：
println!("Configuration valid.");

// 有失败项（每项）：
println!("[FAIL] {field}: {reason}");
println!("  hint: {hint_text}");
```

**函数签名保持不变：**
```rust
pub fn handle_validate(cfg: &Config) { ... }
```

**Imports 模式（替换现有 imports）：**
```rust
use crate::config::Config;
// 移除: use log::info;
// 无需新增 use，println! 是 std macro
```

**注意：** `main.rs` 中 validate 子命令路径（lines 139-149）在 `handle_validate` 前仍调用 `logging::init_logging(&cfg.logging, true)`——此调用保留不动，但 `handle_validate` 本身不再依赖日志系统输出用户可见结果。

---

### `src/cli/init.rs` — `CONFIG_TEMPLATE_EN` 常量 (config-template, transform)

**目标变更：** 检查并补全 `exporter.csv.append`、`exporter.sqlite.*` 各字段的行内注释（D-04）。

**当前模板尾部** (`src/cli/init.rs` lines 118-133，需对比每个字段)：

```toml
[exporter.csv]
file = "outputs/sqllog.csv"
overwrite = true
append = false                     # ← 无注释，需补充

# [exporter.sqlite]
# database_url = "export/sqllog2db.db"   # ← 无注释，需补充
# table_name = "sqllog_records"           # ← 无注释，需补充
# overwrite = true                        # ← 无注释，需补充
# append = false                          # ← 无注释，需补充
```

**已有注释的参考风格**（`src/cli/init.rs` lines 67-73，行内注释格式）：
```toml
# Application log file path
file = "logs/sqllog2db.log"
# Log level: trace | debug | info | warn | error
level = "info"
# Log retention in days (1-365)
retention_days = 7
```
提取要点：注释在字段**上方独立行**，格式 `# <Field description>: <valid values or range>`。保持与已有注释风格一致。

**补充内容目标字段：**

| 字段 | 应添加注释内容 |
|------|---------------|
| `exporter.csv.append` | Append to existing CSV file instead of overwriting (true/false) |
| `exporter.sqlite.database_url` | SQLite database file path |
| `exporter.sqlite.table_name` | Table name to write records into (ASCII identifiers only) |
| `exporter.sqlite.overwrite` | Drop and recreate the table before writing (true/false) |
| `exporter.sqlite.append` | Append rows to existing table instead of overwriting (true/false) |

---

## Shared Patterns

### 直接用户输出（不走日志路由）
**Source:** `src/preflight.rs` lines 96-106 (`print_and_check`)
**Apply to:** `handle_validate` 的所有用户可见输出行
```rust
// 用 println! 输出到 stdout（validate 是正常输出，非错误）
println!("Configuration valid.");
// 或失败项
println!("[FAIL] {field}: {reason}");
println!("  hint: {hint_text}");
```

### `[TAG] message\n  hint: ...` 输出格式
**Source:** `src/main.rs` lines 62-70 (`format_error_output`)
**Apply to:** `handle_validate` 中每个失败项的格式化
- `[FAIL]` 对齐 `[ERROR]` / `[WARNING]` 的括号标签风格
- hint 行用两空格缩进，前缀 `hint: `（无 `[]`）

### 常量字符串模板风格
**Source:** `src/cli/init.rs` lines 61-133 (`CONFIG_TEMPLATE_EN`)
**Apply to:** `CONFIG_TEMPLATE_EN` 中新增注释行
- 注释在字段上方独立行
- 格式：`# <Description>: <valid values>`（与已有注释风格一致）
- 注释内容保持英文（`CONFIG_TEMPLATE_EN` 是英文模板）

---

## No Analog Found

无——本 phase 所有变更均有明确现存模式可参照。

---

## Metadata

**Analog search scope:** `src/cli/`, `src/preflight.rs`, `src/main.rs`, `tests/integration.rs`
**Files scanned:** 8
**Pattern extraction date:** 2026-05-31
