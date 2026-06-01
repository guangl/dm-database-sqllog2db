# Phase 49: Glob 输入支持 - Pattern Map

**Mapped:** 2026-05-31
**Files analyzed:** 5
**Analogs found:** 5 / 5

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/config/sqllog.rs` | config/model | CRUD | `src/config/sqllog.rs` (self) + `src/config/mod.rs` (`pipeline_deprecated` pattern) | exact |
| `src/parser.rs` | service | file-I/O | `src/parser.rs` (self, refactor) | exact |
| `src/cli/opts.rs` | config/CLI | request-response | `src/cli/opts.rs` (self) | exact |
| `src/main.rs` | controller | request-response | `src/main.rs` (self, injection point) | exact |
| `src/cli/run/mod.rs` | controller | file-I/O | `src/cli/run/mod.rs` (self) + `src/cli/run/prescan.rs` | exact |

---

## Pattern Assignments

### `src/config/sqllog.rs` (config/model, CRUD)

**主要变更：** `path: String` → `inputs: Vec<String>`，添加 `path_deprecated` 旧键检测字段，`validate()` 检测旧键并返回迁移错误。

**Analog:** `src/config/mod.rs`（`pipeline_deprecated` 和 `template_deprecated` 旧键检测模式）

**当前完整文件**（`src/config/sqllog.rs` 全文，32 行）：
```rust
use crate::error::{ConfigError, Error, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct SqllogConfig {
    #[serde(alias = "directory")]
    pub path: String,
}

impl Default for SqllogConfig {
    fn default() -> Self {
        Self {
            path: "sqllogs".to_string(),
        }
    }
}

impl SqllogConfig {
    pub fn validate(&self) -> Result<()> {
        if self.path.trim().is_empty() {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "sqllog.path".to_string(),
                value: self.path.clone(),
                reason: "Input path cannot be empty".to_string(),
            }));
        }
        Ok(())
    }
}
```

**旧键检测模式**（从 `src/config/mod.rs` 第 33-42 行提取）：
```rust
/// 旧路径检测：捕获 `[pipeline]` 表（若用户仍用旧格式）。
/// 非 None 时 validate() 会返回迁移错误，用户不应直接使用此字段。
#[doc(hidden)]
#[serde(rename = "pipeline", default)]
pub pipeline_deprecated: Option<toml::Value>,

/// 旧路径检测：捕获 `[template]` 表（若用户仍用旧格式）。
/// 非 None 时 validate() 会返回废弃错误，用户不应直接使用此字段。
#[doc(hidden)]
#[serde(rename = "template", default)]
pub template_deprecated: Option<toml::Value>,
```

**旧键 validate 触发模式**（从 `src/config/validate.rs` 第 27-38 行提取）：
```rust
if self.pipeline_deprecated.is_some() {
    return Err(Error::Config(ConfigError::InvalidValue {
        field: "[pipeline]".to_string(),
        value: String::new(),
        reason: PIPELINE_MIGRATION_HINT.to_string(),
    }));
}
if self.template_deprecated.is_some() {
    return Err(Error::Config(ConfigError::InvalidValue {
        field: "[template]".to_string(),
        value: String::new(),
        reason: "配置段 [template] 已废弃，请移除此配置段".to_string(),
    }));
}
```

**复制要点：**
- `inputs` 字段：`#[serde(default)] pub inputs: Vec<String>`，`Default` 实现返回 `vec!["sqllogs".to_string()]`
- 旧键字段：`#[doc(hidden)] #[serde(rename = "path", default)] pub path_deprecated: Option<toml::Value>`
- 注意：不能再用 `#[serde(alias = "directory")]`，因为 `path` 现在变成了废弃键检测字段，`directory` 别名也需要一并移除
- `validate()` 中先检测 `path_deprecated.is_some()`，取出实际值用 `toml::Value::to_string()` 填入 `value` 字段，`reason` 包含迁移示例
- `inputs` 非空验证：检查 `self.inputs.is_empty()` 或任意条目 `.trim().is_empty()`

---

### `src/parser.rs` (service, file-I/O)

**主要变更：** `SqllogParser` 从持有单个 `path: PathBuf` 改为持有 `inputs: Vec<String>`；`log_files()` 遍历所有 inputs，每条独立展开（文件/目录/glob），结果合并去重排序；无匹配时返回错误而非 warn。

**Analog:** `src/parser.rs`（self，内部 `scan_glob` 已实现单路径展开，复用为多路径内部函数）

**当前 `new()` + `log_files()` 接口**（第 14-25 行）：
```rust
impl SqllogParser {
    pub(crate) fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub(crate) fn log_files(&self) -> Result<Vec<PathBuf>> {
        self.scan_log_files()
    }
```

**当前 `scan_glob` 实现**（第 95-127 行，无匹配时仅 warn，Phase 49 需改为 error）：
```rust
fn scan_glob(&self, pattern: &str) -> Result<Vec<PathBuf>> {
    #[cfg(windows)]
    let pattern_normalized = pattern.replace('\\', "/");
    #[cfg(not(windows))]
    let pattern_normalized = pattern.to_owned();
    let pattern = pattern_normalized.as_str();

    let mut log_files: Vec<PathBuf> = glob::glob(pattern)
        .map_err(|e| {
            Error::Parser(ParserError::InvalidPath {
                path: self.path.clone(),
                reason: format!("invalid glob pattern: {e}"),
                line_number: None,
            })
        })?
        .filter_map(std::result::Result::ok)
        .filter(|p| p.is_file() && p.extension().is_some_and(|ext| ext == "log"))
        .collect();

    log_files.sort();

    if log_files.is_empty() {
        warn!("No .log files matched glob pattern: {pattern}");
    } else {
        info!(
            "Glob matched {} log files for pattern: {pattern}",
            log_files.len()
        );
    }

    Ok(log_files)
}
```

**复制要点：**
- `SqllogParser` 字段改为 `inputs: Vec<String>`；`new()` 签名改为接受 `Vec<String>`（或 `impl IntoIterator<Item = impl Into<String>>`）
- `scan_log_files()` 改为遍历 `self.inputs`，每条调用单路径展开（重命名现有 `scan_log_files` 逻辑为 `expand_single`），结果追加到共享 `Vec`
- 最终 `dedup`（先 `sort()` 再 `dedup()`）确保跨 inputs 无重复
- **无匹配时 error**：所有 inputs 展开后 `log_files.is_empty()` 时，返回 `Err(Error::Parser(ParserError::NoFilesFound { inputs: self.inputs.clone() }))` 或使用现有 `InvalidPath` 变体 + 特定 reason；需在 `error.rs::ParserError` 添加新变体或复用现有变体
- 单个 input 展开失败（如 invalid glob pattern、路径不存在）仍返回 `Err`（现有行为不变）
- 目录内无 `.log` 文件时改为静默（不报错，由最终空列表触发上层错误）

---

### `src/cli/opts.rs` (config/CLI, request-response)

**主要变更：** `Run` 变体添加 `input: Option<Vec<String>>` 字段，使用 `clap::ArgAction::Append` 支持重复。

**Analog:** `src/cli/opts.rs`（self，现有 `-c/--config` 字段的 `#[arg(...)]` 声明模式）

**现有 `Run` 变体字段模式**（第 58-68 行）：
```rust
Run {
    /// TOML configuration file path
    #[arg(
        short = 'c',
        long = "config",
        default_value = "config.toml",
        env = "SQLLOG2DB_CONFIG",
        help = "TOML configuration file path. See [csv], [sqlite], [pipeline] sections."
    )]
    config: String,
},
```

**D-04 新字段声明**（直接从 CONTEXT.md 决策）：
```rust
#[arg(long = "input", short = 'i', action = clap::ArgAction::Append)]
pub input: Option<Vec<String>>,
```

**复制要点：**
- `Run` 变体变为具名字段结构，添加 `input` 字段
- `main.rs` 中 `Commands::Run { config }` 的模式匹配需同步扩展为 `Commands::Run { config, input }`
- `after_help` 中更新 examples，添加 `--input` 用法示例

---

### `src/main.rs` (controller, request-response)

**主要变更：** `Commands::Run` 匹配解构添加 `input`，在 `handle_run` 之前将 CLI `--input` 值注入 `cfg.sqllog.inputs`。

**Analog:** `src/main.rs`（self，`apply_verbosity_to_config` 函数展示了 run 前修改 `cfg` 字段的模式）

**现有 CLI 值注入 config 的模式**（第 115-120 行）：
```rust
Some(cli::opts::Commands::Run { config }) => {
    let mut cfg = load_config(config)?;
    let compiled_filters = cfg.validate_and_compile()?;

    apply_verbosity_to_config(&mut cfg, cli.verbose, cli.quiet);
```

**`apply_verbosity_to_config` 的字段覆盖模式**（第 51-57 行）：
```rust
fn apply_verbosity_to_config(cfg: &mut Config, verbose: u8, quiet: bool) {
    if verbose >= 1 {
        cfg.logging.level = "debug".to_string();
    } else if quiet {
        cfg.logging.level = "error".to_string();
    }
}
```

**复制要点：**
- 模式匹配改为 `Commands::Run { config, input }`
- `load_config` 之后、`validate_and_compile` 之前，插入：
  ```rust
  if let Some(cli_inputs) = input {
      cfg.sqllog.inputs = cli_inputs.clone();
  }
  ```
- 或者提取为 `apply_cli_inputs_to_config(cfg: &mut Config, input: &Option<Vec<String>>)` 函数，与 `apply_verbosity_to_config` 风格一致

---

### `src/cli/run/mod.rs` (controller, file-I/O)

**主要变更：** `SqllogParser::new(&cfg.sqllog.path)` 改为 `SqllogParser::new(cfg.sqllog.inputs.clone())`（或传引用，视新接口签名而定）；`log_files.is_empty()` 时的 stdin fallback 逻辑不变（`parser.rs` 层返回错误后，`run/mod.rs` 不再需要手动检查空列表）。

**Analog:** `src/cli/run/mod.rs`（self）和 `src/cli/run/prescan.rs`（调用方的 `path` 字段传递模式）

**现有 `SqllogParser::new` 调用点**（第 40 行）：
```rust
let log_files = SqllogParser::new(&cfg.sqllog.path).log_files()?;
```

**现有空列表 + stdin fallback**（第 43-55 行）：
```rust
let is_stdin_pipe =
    log_files.is_empty() && !std::io::stdin().is_terminal() && !cfg!(target_os = "windows");
let log_files = if is_stdin_pipe {
    info!("No log files found, reading from stdin (pipe mode)");
    vec![std::path::PathBuf::from("/dev/stdin")]
} else if log_files.is_empty() {
    warn!("No log files found");
    return Ok(ErrorStats::default());
} else {
    log_files
};
```

**复制要点：**
- 第 40 行改为 `SqllogParser::new(cfg.sqllog.inputs.clone()).log_files()`（或传 `&cfg.sqllog.inputs`，视新接口）
- Phase 49 要求无匹配时 parser 层直接返回 `Err`，但 stdin pipe 模式的空列表 fallback 必须保留：需在 `log_files()` 层面或调用后区分"真正无文件"与"stdin pipe"；最简方案：parser 返回空 `Vec` 时仍不报错，由 `run/mod.rs` 的现有空列表检查决定是否进 stdin 或报错；或者 parser 仅在非 stdin 上下文下报错（通过 flag 参数控制，但不推荐耦合）。
  - **推荐方案**：`log_files()` 返回 `Ok(empty_vec)` 而非 Err，`run/mod.rs` 中 `is_stdin_pipe` 判断不变，`else if log_files.is_empty()` 分支改为返回 `Err(Error::Parser(...))` 并带 hint，而不是静默 `return Ok(ErrorStats::default())`。这样 stdin pipe 逻辑无需改动。

---

## Shared Patterns

### 错误类型：ConfigError::InvalidValue（旧键迁移 hint）
**Source:** `src/error.rs` 第 177-182 行，`src/config/validate.rs` 第 27-38 行
**Apply to:** `src/config/sqllog.rs` 的 `validate()` 旧键检测

```rust
// error.rs 中 InvalidValue 变体
#[error("Invalid configuration value {field} = '{value}': {reason}")]
InvalidValue {
    field: String,
    value: String,
    reason: String,
},
```

`suggestion()` 返回固定字符串 `"Check the field value in the configuration file."`——Phase 49 的迁移 hint 通过 `reason` 字段传递（而非 suggestion），与 `pipeline_deprecated` 模式一致。

### 错误类型：ParserError（无文件匹配的新变体或复用）
**Source:** `src/error.rs` 第 199-237 行
**Apply to:** `src/parser.rs` 空列表错误路径

```rust
pub enum ParserError {
    PathNotFound { path: PathBuf },
    InvalidPath {
        path: PathBuf,
        reason: String,
        line_number: Option<u64>,
    },
    ReadDirFailed { path: PathBuf, reason: String },
}
```

Phase 49 需要一个能携带 `inputs: Vec<String>` 的新变体，或者在 `run/mod.rs` 中用 `InvalidPath` + reason 字符串模拟。推荐添加新变体以避免语义混淆：
```rust
NoFilesFound { inputs: Vec<String> },
```
对应 `Display` 格式：`"No log files found matching inputs: {inputs:?}"`

### `format_error_output` + hint 前缀
**Source:** `src/main.rs` 第 59-70 行
**Apply to:** 任何新增的 `Error::suggestion()` 分支

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

新增 `ParserError::NoFilesFound` 需在 `Error::suggestion()` 的 `Error::Parser(e)` 匹配分支添加对应 hint 文本。

### serde 旧键检测模式
**Source:** `src/config/mod.rs` 第 33-42 行 + `src/config/validate.rs` 第 27-38 行
**Apply to:** `src/config/sqllog.rs`

三要素：
1. 结构体中声明 `#[doc(hidden)] #[serde(rename = "旧键名", default)] pub xxx_deprecated: Option<toml::Value>`
2. `validate()`/`validate_and_compile()` 开头检测 `if self.xxx_deprecated.is_some()`
3. 提取实际值字符串：`self.path_deprecated.as_ref().map(|v| v.to_string()).unwrap_or_default()` 填入 `ConfigError::InvalidValue.value`

---

## 调用方引用更新

以下两处调用 `cfg.sqllog.path` 需同步改为 `cfg.sqllog.inputs`（planner 需单独列出 action）：

| 文件 | 行号 | 当前代码 | 改为 |
|------|------|----------|------|
| `src/cli/run/mod.rs` | 40 | `SqllogParser::new(&cfg.sqllog.path)` | `SqllogParser::new(cfg.sqllog.inputs.clone())` 或传引用 |
| `src/cli/run/prescan.rs` | （通过 `cfg` 间接访问，不直接引用 `cfg.sqllog.path`） | — | 无需改动 |
| `src/config/validate.rs` | 163（测试）| `cfg.sqllog.path = "  ".into()` | `cfg.sqllog.inputs = vec![]` 或测试空字符串条目 |
| `src/config/mod.rs` 测试 | 98 | `cfg.sqllog.path` | `cfg.sqllog.inputs` |

---

## No Analog Found

所有文件均有明确 analog，无需依赖 RESEARCH.md 外部模式。

---

## Metadata

**Analog search scope:** `src/config/`, `src/parser.rs`, `src/cli/`, `src/main.rs`, `src/error.rs`
**Files scanned:** 8
**Pattern extraction date:** 2026-05-31
