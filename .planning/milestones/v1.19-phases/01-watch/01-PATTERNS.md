# Phase 1: watch 功能完善 - Pattern Map

**Mapped:** 2026-06-06
**Files analyzed:** 3 (需要修改的文件)
**Analogs found:** 3 / 3

## File Classification

| 需要修改的文件 | Role | Data Flow | Closest Analog | Match Quality |
|----------------|------|-----------|----------------|---------------|
| `src/cli/watch/mod.rs` | service | event-driven | `src/cli/watch/mod.rs` 内部现有模式 | exact（内部扩展） |
| `src/cli/run/mod.rs` | service | file-I/O | `src/exporter/csv/mod.rs` `initialize()` | role-match（同为文件打开模式切换） |
| `src/config/mod.rs` | config | — | `src/config/exporter.rs` `CsvExporterConfig` | role-match（同为 serde 配置结构体） |

---

## Pattern Assignments

### `src/cli/watch/mod.rs` — trigger_full_file + build_incremental_cfg + handle_watch

#### 修改点 1：`trigger_full_file`（line 295–326）

**当前代码（line 311–313，需要在此之后插入 CSV/error_log 注入）：**
```rust
// src/cli/watch/mod.rs line 311-313
let mut tmp_cfg = cfg.clone();
tmp_cfg.sqllog.inputs = vec![path.to_string_lossy().into_owned()];
match crate::cli::run::handle_run(&tmp_cfg, quiet, verbose, interrupted, None) {
```

**类比模式（build_incremental_cfg SQLite 注入，line 514–523）：**
```rust
// src/cli/watch/mod.rs line 514-523
fn build_incremental_cfg(cfg: &Config, tmp_file: &tempfile::NamedTempFile) -> Config {
    let mut tmp_cfg = cfg.clone();
    tmp_cfg.sqllog.inputs = vec![tmp_file.path().to_string_lossy().into_owned()];
    // D-09: 增量路径强制 append=true、overwrite=false，避免清空表
    if let Some(ref mut sqlite_cfg) = tmp_cfg.exporter.sqlite {
        sqlite_cfg.append = true;
        sqlite_cfg.overwrite = false;
    }
    tmp_cfg
}
```

**WATCH-07 + WATCH-08 需要在 `trigger_full_file` 的 `tmp_cfg.sqllog.inputs = ...` 之后、`handle_run` 之前添加（复制自 build_incremental_cfg 的 SQLite 模式，对称扩展）：**
```rust
// 新增注入（WATCH-07）
if let Some(ref mut csv_cfg) = tmp_cfg.exporter.csv {
    csv_cfg.append = true;
    csv_cfg.overwrite = false;
}
// 新增注入（WATCH-08）
tmp_cfg.append_error_log = true;
```

#### 修改点 2：`build_incremental_cfg`（line 514–523）

**当前代码（line 514–523）：**
```rust
fn build_incremental_cfg(cfg: &Config, tmp_file: &tempfile::NamedTempFile) -> Config {
    let mut tmp_cfg = cfg.clone();
    tmp_cfg.sqllog.inputs = vec![tmp_file.path().to_string_lossy().into_owned()];
    // D-09: 增量路径强制 append=true、overwrite=false，避免清空表
    if let Some(ref mut sqlite_cfg) = tmp_cfg.exporter.sqlite {
        sqlite_cfg.append = true;
        sqlite_cfg.overwrite = false;
    }
    tmp_cfg
}
```

**扩展模式（在 SQLite 块之后、`tmp_cfg` 返回之前追加，与 SQLite 完全对称）：**
```rust
// 新增（WATCH-07）
if let Some(ref mut csv_cfg) = tmp_cfg.exporter.csv {
    csv_cfg.append = true;
    csv_cfg.overwrite = false;
}
// 新增（WATCH-08）
tmp_cfg.append_error_log = true;
```

> **Claude's Discretion:** 若提取 `force_append_exporters` 辅助函数，函数签名为：
> ```rust
> fn force_append_exporters(cfg: &mut Config) {
>     if let Some(ref mut sqlite_cfg) = cfg.exporter.sqlite {
>         sqlite_cfg.append = true;
>         sqlite_cfg.overwrite = false;
>     }
>     if let Some(ref mut csv_cfg) = cfg.exporter.csv {
>         csv_cfg.append = true;
>         csv_cfg.overwrite = false;
>     }
>     cfg.append_error_log = true;
> }
> ```
> 调用方：`force_append_exporters(&mut tmp_cfg);`

#### 修改点 3：`handle_watch` 尾部（line 66–73）

**当前代码（line 66–73）：**
```rust
// src/cli/watch/mod.rs line 66-73
pb.finish_and_clear();
print_final_summary(
    &start,
    state.trigger_count(),
    state.total_stats().records_exported,
    quiet,
);
Ok(())
```

**类比模式（`src/cli/run/mod.rs` line 148–150，interrupted 检查后返回错误）：**
```rust
// src/cli/run/mod.rs line 148-150
if interrupted.load(Ordering::Acquire) {
    return Err(Error::Interrupted);
}
```

**WATCH-09 需要在 `print_final_summary` 之后、`Ok(())` 之前插入（D-08：先打印摘要再检查）：**
```rust
// 新增（WATCH-09）
if interrupted.load(Ordering::Acquire) {
    return Err(crate::error::Error::Interrupted);
}
Ok(())
```

---

### `src/cli/run/mod.rs` — write_error_log（line 423–461）

**当前代码（line 425–438）：**
```rust
// src/cli/run/mod.rs line 425-438
fn write_error_log(cfg: &crate::config::Config, stats: &ErrorStats) {
    let Some(error_cfg) = cfg.error.as_ref() else {
        return;
    };
    if stats.parse_error_records.is_empty() {
        return;
    }
    use std::io::Write;
    let file = match std::fs::File::create(&error_cfg.file) {
        Ok(f) => f,
        Err(e) => {
            log::warn!("Failed to create error log {}: {e}", error_cfg.file);
            return;
        }
    };
```

**类比模式（`src/exporter/csv/mod.rs` line 102–120，OpenOptions 两分支切换）：**
```rust
// src/exporter/csv/mod.rs line 102-120
let append_mode = self.write_mode == WriteMode::Append;
let file = if append_mode {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&self.path)
} else {
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(self.write_mode == WriteMode::Truncate)
        .open(&self.path)
}
.map_err(|e| { ... })?;
```

**WATCH-08 修改后的 `write_error_log` 文件打开段（替换 `std::fs::File::create` 这一行）：**
```rust
use std::io::Write;
let file = if cfg.append_error_log {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&error_cfg.file)
} else {
    std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&error_cfg.file)
};
let file = match file {
    Ok(f) => f,
    Err(e) => {
        log::warn!("Failed to create error log {}: {e}", error_cfg.file);
        return;
    }
};
```

**保持不变的部分（line 440–461）：**
```rust
// src/cli/run/mod.rs line 440-461（flush 逻辑原样保留）
let mut writer = std::io::BufWriter::new(file);
let truncated = stats.parse_errors > stats.parse_error_records.len();
for rec in &stats.parse_error_records {
    let _ = writeln!(
        writer,
        "[ERROR] line {}: {}  reason: {}",
        rec.line_number,
        rec.raw_truncated,
        rec.kind.kind_display()
    );
}
if truncated {
    let _ = writeln!(
        writer,
        "[truncated; showing first 10000 of {} total parse errors]",
        stats.parse_errors
    );
}
if let Err(e) = writer.flush() {
    log::warn!("Failed to flush error log: {e}");
}
```

---

### `src/config/mod.rs` — Config 结构体（line 23–41）

**当前代码（line 23–41）：**
```rust
// src/config/mod.rs line 23-41
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub sqllog: SqllogConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub exporter: ExporterConfig,
    #[serde(default)]
    pub replace_parameters: Option<NormalizeConfig>,
    #[serde(default)]
    pub filter: Option<FiltersFeature>,
    #[serde(default)]
    pub output: Option<OutputConfig>,
    #[serde(default)]
    pub stats: StatsConfig,
    #[serde(default)]
    pub error: Option<ErrorLogConfig>,
}
```

**类比模式（`src/config/exporter.rs` line 42–47，`#[serde(default)]` 字段声明）：**
```rust
// src/config/exporter.rs line 42-47
#[derive(Debug, Deserialize, Clone)]
pub struct CsvExporterConfig {
    pub file: String,
    #[serde(default = "default_true")]
    pub overwrite: bool,
    #[serde(default)]
    pub append: bool,
```

**WATCH-08 需要在 `Config` 最后一个字段 `error` 之后追加（D-04）：**
```rust
/// watch 触发时设为 true，使 write_error_log 以追加模式打开文件。
/// run 路径不设置此字段，默认 false（覆盖写）。
#[serde(skip)]
pub(crate) append_error_log: bool,
```

> `bool` 的 `Default::default()` 为 `false`，`derive(Default)` 自动保证 run 路径行为不变。

---

## Shared Patterns

### 模式 1：克隆 cfg 后注入覆盖标志
**来源：** `src/cli/watch/mod.rs` `build_incremental_cfg`（line 514–523）
**应用到：** `trigger_full_file` 内联注入、`build_incremental_cfg` 扩展
```rust
let mut tmp_cfg = cfg.clone();
// ... 修改 tmp_cfg 的各字段 ...
if let Some(ref mut sqlite_cfg) = tmp_cfg.exporter.sqlite {
    sqlite_cfg.append = true;
    sqlite_cfg.overwrite = false;
}
```

### 模式 2：OpenOptions 双分支文件打开
**来源：** `src/exporter/csv/mod.rs` `initialize()`（line 102–120）
**应用到：** `write_error_log` 的文件打开段
```rust
let file = if append_mode {
    OpenOptions::new().create(true).append(true).open(&path)
} else {
    OpenOptions::new().create(true).write(true).truncate(true).open(&path)
};
```

### 模式 3：interrupted 检查后返回 Err(Error::Interrupted)
**来源：** `src/cli/run/mod.rs`（line 148–150）和 `run_watch_loop`（line 172–174）
**应用到：** `handle_watch` 尾部
```rust
if interrupted.load(Ordering::Acquire) {
    return Err(Error::Interrupted);
}
```

### 模式 4：#[serde(skip)] 内部字段
**来源：** `src/config/exporter.rs`（`#[serde(default)]` 风格）；`#[serde(skip)]` 确保字段不参与 TOML 解析
**应用到：** `Config.append_error_log`
```rust
#[serde(skip)]
pub(crate) append_error_log: bool,
```

---

## No Analog Found

本阶段所有修改点均有明确类比，无"无类比"条目。

---

## Key Implementation Notes

| 条目 | 内容 |
|------|------|
| 函数 `write_error_log` 当前用 `std::fs::File::create` | 等价于 `OpenOptions::new().create(true).write(true).truncate(true)` — D-06 明确替换为显式 `OpenOptions` 双分支 |
| `trigger_full_file` 与 `build_incremental_cfg` 两处对称 | Pitfall 1：只改一处会导致 Create 事件（全量）仍覆盖 CSV |
| `append_error_log` 默认 false | Rust `bool` 的 `Default::default()` 为 false，derive(Default) 自动安全 |
| 退出码检查位置 | 必须在 `print_final_summary` 之后（D-08），否则 Ctrl+C 后看不到摘要 |
| `Error::Interrupted` 变体 | 已在 `src/error.rs` line 163 定义，`main.rs` line 114–115 已有 exit(130) 分支 |

---

## Metadata

**Analog search scope:** `src/cli/watch/`, `src/cli/run/`, `src/config/`, `src/exporter/csv/`, `src/error.rs`
**Files scanned:** 5
**Pattern extraction date:** 2026-06-06
