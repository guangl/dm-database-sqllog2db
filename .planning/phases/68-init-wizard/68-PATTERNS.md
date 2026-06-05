# Phase 68: 交互式配置向导 - Pattern Map

**Mapped:** 2026-06-06
**Files analyzed:** 4 (新增/修改)
**Analogs found:** 4 / 4

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/cli/opts.rs` | config (CLI flag) | request-response | `src/cli/opts.rs` 现有 `force: bool` | exact (同文件内扩展) |
| `src/main.rs` | controller (dispatch) | request-response | `src/main.rs` 现有 `Commands::Init` match arm | exact (同文件内扩展) |
| `src/cli/init.rs` | service (wizard + write) | request-response | `src/cli/init.rs` 现有 `handle_init` | exact (同文件内扩展) |
| `tests/integration.rs` | test | request-response | `tests/integration.rs` 现有 `test_handle_init_*` 系列 | exact |

---

## Pattern Assignments

### `src/cli/opts.rs` — 新增 `interactive: bool` flag

**Analog:** `src/cli/opts.rs` lines 97-99（现有 `force` bool flag 模式）

**现有 bool flag 模式**（lines 97-99）:
```rust
/// Force overwrite if file exists
#[arg(short = 'f', long = "force")]
force: bool,
```

**新增 interactive flag 应复制此模式**（紧跟 `force` 之后）:
```rust
/// Start interactive configuration wizard
#[arg(short = 'i', long = "interactive")]
interactive: bool,
```

**注意事项：**
- `Commands::Run` 的 `-i` short flag 是 `--input`（ArgAction::Append），在不同 subcommand variant 内，clap 自动隔离，无冲突（opts.rs lines 72-78 可确认）
- `Init` variant 完整定义在 opts.rs lines 88-100，新 flag 加在 `force` 之后

---

### `src/main.rs` — dispatch 分支修改

**Analog:** `src/main.rs` lines 141-143（现有 `Commands::Init` match arm）

**现有 dispatch 模式**（lines 141-143）:
```rust
Some(cli::opts::Commands::Init { output, force }) => {
    cli::init::handle_init(output, *force)?;
    Ok(None)
}
```

**修改后应扩展为**（复制同文件 run 分支的 if/else 风格）:
```rust
Some(cli::opts::Commands::Init { output, force, interactive }) => {
    if *interactive {
        cli::init::handle_init_interactive(output, *force)?;
    } else {
        cli::init::handle_init(output, *force)?;
    }
    Ok(None)
}
```

**参考：** `init_simple_logging` 在 `needs_simple_logging` 块内已对 `Init` 生效（lines 132-138），无需修改日志初始化路径。

---

### `src/cli/init.rs` — 新增向导逻辑

**Analog:** `src/cli/init.rs` lines 1-59（现有 `handle_init` 完整实现）

#### 导入模式（lines 1-4）:
```rust
use crate::error::{Error, FileError, Result};
use log::{debug, error, info, warn};
use std::fs;
use std::path::Path;
```

**新增向导需额外引入**（在文件顶部追加）:
```rust
use crate::error::ConfigError;
use std::io::{BufRead, Write};
```

#### 文件写入提取：`write_config_file` 私有函数

从 `handle_init` lines 8-58 提取核心写入逻辑，两个 public 函数共用：

```rust
// Extracted from handle_init (lines 8-58) — 保持两个 public 函数各在 40 行内
fn write_config_file(path: &Path, content: &str, force: bool) -> Result<()> {
    let output_path = path.to_string_lossy();
    info!("Preparing to generate configuration file: {output_path}");
    let file_existed = path.exists();

    if file_existed && !force {
        error!("Configuration file already exists: {output_path}");
        info!("Tip: use --force to overwrite");
        return Err(Error::File(FileError::AlreadyExists {
            path: path.to_path_buf(),
        }));
    }
    if file_existed && force {
        warn!("Will overwrite existing configuration file");
    }
    if let Some(parent) = path.parent().filter(|p| !p.exists()) {
        info!("Creating directory: {}", parent.display());
        fs::create_dir_all(parent).map_err(|e| {
            Error::File(FileError::CreateDirectoryFailed {
                path: parent.to_path_buf(),
                reason: e.to_string(),
            })
        })?;
    }
    fs::write(path, content).map_err(|e| {
        Error::File(FileError::WriteFailed {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })
    })?;
    if file_existed {
        info!("Configuration file overwritten: {output_path}");
    } else {
        info!("Configuration file generated: {output_path}");
    }
    Ok(())
}
```

#### "Next steps" 输出模式（lines 53-57）:
```rust
info!("Next steps:");
info!("  1. Edit configuration file: {output_path}");
info!("  2. Validate configuration: sqllog2db validate -c {output_path}");
info!("  3. Run export: sqllog2db run -c {output_path}");
```

`handle_init_interactive` 在写入完成后调用相同的 `info!` 输出（或提取为共享私有函数）。

#### `WizardAnswers` 结构体（新增）:
```rust
#[derive(Debug)]
pub enum ExporterChoice {
    Csv,
    Sqlite,
}

#[derive(Debug)]
pub struct WizardAnswers {
    pub inputs: String,
    pub exporter: ExporterChoice,
    pub csv_file: Option<String>,
    pub sqlite_db: Option<String>,
    pub sqlite_table: Option<String>,
}
```

#### `run_wizard` 函数签名与 IO 模式（来自 RESEARCH.md Pattern 3）:

```rust
pub fn run_wizard(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> Result<WizardAnswers> {
    // Step 1: inputs
    write!(writer, "SQL log 输入目录（可以是目录、文件或 glob 模式）[default: sqllogs]: ")?;
    writer.flush()?;
    let mut buf = String::new();
    reader.read_line(&mut buf)?;
    let inputs = if buf.trim().is_empty() {
        "sqllogs".to_owned()
    } else {
        buf.trim().to_owned()
    };
    buf.clear();

    // Step 2: 格式验证（最多 3 次）
    // ...
    // 返回 Err(Error::Config(ConfigError::InvalidValue { ... })) 超出次数时

    Ok(WizardAnswers { ... })
}
```

**关键：** `write!` 宏需要 `use std::io::Write` 在 scope 内；与 `tests/integration.rs` line 17 中 `use std::fmt::Write as _` 不冲突（作用域不同）。

#### 导出格式验证错误类型（来自 error.rs lines 253-258）:
```rust
// 超出 3 次时返回此错误
Error::Config(ConfigError::InvalidValue {
    field: "exporter".to_owned(),
    value: last_input.to_owned(),
    reason: "must be 'csv' or 'sqlite'".to_owned(),
})
```

#### `apply_wizard_answers_to_template` 字符串替换模式

模板中精确的替换目标（来自 `src/cli/init.rs` lines 63-152 实际内容）：

| 替换场景 | 精确搜索字符串 | 替换为 |
|---------|--------------|-------|
| inputs 路径 | `inputs = ["sqllogs"]` | `inputs = ["{user_inputs}"]` |
| CSV file 路径 | `file = "outputs/sqllog.csv"` | `file = "{user_csv_file}"` |
| SQLite 激活 — database_url | `# database_url = "export/sqllog2db.db"` | `database_url = "{user_sqlite_db}"` |
| SQLite 激活 — table_name | `# table_name = "sqllog_records"` | `table_name = "{user_sqlite_table}"` |
| SQLite 激活 — 段头 | `# [exporter.sqlite]` | `[exporter.sqlite]` |

**CSV → SQLite 模式时需额外注释掉 `[exporter.csv]` 段**（逐行加 `# `）。

**风险警告：** `[logging]` 段有 `file = "logs/sqllog2db.log"`，与 csv 的 `file =` 相似。替换时必须使用完整字符串 `file = "outputs/sqllog.csv"` 作为搜索键（在模板中唯一，lines 136）。

#### `handle_init_interactive` 公开函数:
```rust
pub fn handle_init_interactive(output: &str, force: bool) -> Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    let answers = run_wizard(&mut reader, &mut writer)?;
    let content = apply_wizard_answers_to_template(&answers);
    let path = Path::new(output);
    write_config_file(path, &content, force)?;
    info!("Next steps:");
    info!("  1. Edit configuration file: {output}");
    info!("  2. Validate configuration: sqllog2db validate -c {output}");
    info!("  3. Run export: sqllog2db run -c {output}");
    Ok(())
}
```

---

### `tests/integration.rs` — 新增 interactive CLI 测试

**Analog:** `tests/integration.rs` lines 158-212（现有 `test_handle_init_*` 系列）

#### 测试 imports 模式（lines 1-12）:
```rust
use dm_database_sqllog2db::cli::init::handle_init;
// 新增：
use dm_database_sqllog2db::cli::init::{run_wizard, ExporterChoice};
```

#### 单元测试注入 Cursor 模式（来自 RESEARCH.md Code Examples）:
```rust
#[test]
fn test_wizard_all_defaults() {
    let input = b"\n\n\n";  // Enter × 3（inputs / format / csv_path 全默认）
    let mut reader = std::io::Cursor::new(input.as_ref());
    let mut writer = Vec::<u8>::new();
    let answers = run_wizard(&mut reader, &mut writer).unwrap();
    assert_eq!(answers.inputs, "sqllogs");
    assert!(matches!(answers.exporter, ExporterChoice::Csv));
    assert_eq!(answers.csv_file.as_deref(), Some("outputs/sqllog.csv"));
}
```

#### tempfile + 文件内容断言模式（lines 159-168）:
```rust
#[test]
fn test_handle_init_creates_config_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    handle_init(config_path.to_str().unwrap(), false).unwrap();
    assert!(config_path.exists());
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[sqllog]"), "...");
}
```

向导集成测试复用此 `TempDir` 模式，额外验证替换后内容是否包含用户输入值。

---

## Shared Patterns

### 错误传播模式
**Source:** `src/cli/init.rs` lines 17-19, 31-36, 40-45
**Apply to:** `handle_init_interactive`, `run_wizard`, `write_config_file`
```rust
// ? 传播 + map_err 包装带上下文的错误
fs::write(path, content).map_err(|e| {
    Error::File(FileError::WriteFailed {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })
})?;
```

### `log::info!` / `log::warn!` 输出约定
**Source:** `src/cli/init.rs` lines 10-56
**Apply to:** `handle_init_interactive`, `write_config_file`
- 系统状态消息（配置生成、目录创建、Next steps）走 `log::info!`
- 向导提示文本（用户交互 prompt）走 `write!(writer, ...)` + `writer.flush()?`，不走 log crate

### `Result<()>` + `crate::error::Result` 类型约定
**Source:** `src/cli/init.rs` line 7 / `src/error.rs` line 7
**Apply to:** 所有新函数签名
```rust
pub type Result<T> = std::result::Result<T, Error>;
// 新函数均返回 crate::error::Result<()>
```

### IO From\<io::Error\> 自动转换
**Source:** `src/error.rs` lines 157-158
```rust
#[error("IO error: {0}")]
Io(#[from] io::Error),
```
`read_line` / `write!` / `flush` 失败时 `?` 自动转换为 `Error::Io`，无需手动 `map_err`。

---

## CONFIG_TEMPLATE_EN 精确内容参考

以下是模板中向导替换涉及的精确行（`src/cli/init.rs` lines 68, 136, 143-148）：

```toml
inputs = ["sqllogs"]           # line 68 — inputs 替换目标

file = "outputs/sqllog.csv"    # line 136 — csv 路径替换目标（logging 的 file 行不同，不会误替换）

# [exporter.sqlite]             # line 143 — sqlite 段激活（去掉 "# "）
# database_url = "export/sqllog2db.db"  # line 145
# table_name = "sqllog_records"         # line 147
# overwrite = true                      # line 149
# append = false                        # line 151
```

CSV → SQLite 模式时，需将 lines 134-140（`[exporter.csv]` 整段）每行加 `# ` 前缀注释掉。

---

## No Analog Found

无——所有新增文件均在现有文件内扩展，模式完全来自项目内部。

---

## Metadata

**Analog search scope:** `src/cli/`, `src/error.rs`, `src/main.rs`, `tests/`
**Files scanned:** 5（init.rs, opts.rs, main.rs, error.rs, tests/integration.rs）
**Pattern extraction date:** 2026-06-06
