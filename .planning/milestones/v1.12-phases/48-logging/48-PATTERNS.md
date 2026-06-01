# Phase 48: 日志级别与运行提示 - Pattern Map

**Mapped:** 2026-05-31
**Files analyzed:** 3 (修改文件，无新建文件)
**Analogs found:** 3 / 3

## File Classification

| 修改文件 | Role | Data Flow | Closest Analog | Match Quality |
|----------|------|-----------|----------------|---------------|
| `src/cli/opts.rs` | config | request-response | `src/cli/opts.rs` 自身（修改现有定义） | exact |
| `src/main.rs` | utility/orchestrator | request-response | `src/main.rs` 自身（修改现有函数） | exact |
| `src/cli/run/mod.rs` | controller | request-response | `src/cli/run/mod.rs` 自身（修改现有逻辑） | exact |

---

## Pattern Assignments

### `src/cli/opts.rs` — 将 `-v` 从 Count 改为 bool

**当前代码（lines 26-36）：**
```rust
/// Verbose output (-v for debug, -vv for trace)
#[arg(
    short = 'v',
    action = clap::ArgAction::Count,
    global = true,
    help = "-v for debug logging, -vv for trace logging."
)]
pub(crate) verbose: u8,

/// Suppress non-error output
#[arg(short = 'q', global = true, conflicts_with = "verbose")]
pub(crate) quiet: bool,
```

**目标模式（参照 `quiet: bool` 的既有风格）：**

`quiet` 字段本身就是 bool 标志的正确参照：
- `#[arg(short = 'q', global = true, conflicts_with = "verbose")]`
- 类型 `bool`，无需 `action = ...`（clap 默认 bool 对应 `ArgAction::SetTrue`）

改写 `verbose` 时完全对称照搬该模式，添加 `long = "verbose"` 显式设置长标志名。

**改写后应得到（D-01）：**
```rust
/// Show per-file processing details
#[arg(
    short = 'v',
    long = "verbose",
    global = true,
    conflicts_with = "quiet",
    help = "Show per-file processing details on stderr."
)]
pub(crate) verbose: bool,
```

注意：现有 `quiet` 字段已有 `conflicts_with = "verbose"`，改写后两者互斥由两侧 `conflicts_with` 共同保证，与 D-01 一致。

---

### `src/main.rs` — 移除 debug 映射，传递 verbose，抑制摘要

#### 1. `apply_verbosity_to_config` 清理（D-02）

**当前代码（lines 51-57）：**
```rust
fn apply_verbosity_to_config(cfg: &mut Config, verbose: u8, quiet: bool) {
    if verbose >= 1 {
        cfg.logging.level = "debug".to_string();
    } else if quiet {
        cfg.logging.level = "error".to_string();
    }
}
```

`verbose` 改为 `bool` 后，`verbose >= 1` 分支失去意义（run 命令不再映射日志级别）。
函数签名改为 `(cfg: &mut Config, verbose: bool, quiet: bool)` 并移除 debug 分支，只保留 quiet 分支。
如果函数体只剩 quiet 一行，planner 可评估是否内联删除该函数。

现有测试 `test_apply_verbosity_verbose`（line 238）和 `test_apply_verbosity_trace`（line 258）需同步更新：这两个测试断言 `debug` 级别映射，改为 bool 后相关行为变化，测试应改为验证 `verbose=true` 不改变 logging level。

#### 2. 传递 verbose 到 handle_run（D-03 & Integration）

**当前调用（line 136）：**
```rust
let stats = cli::run::handle_run(&cfg, cli.quiet, &interrupted, compiled_filters)?;
```

改写为：
```rust
let stats = cli::run::handle_run(&cfg, cli.quiet, cli.verbose, &interrupted, compiled_filters)?;
```

类型从 `u8` 变为 `bool`：`cli.verbose` 直接传入（无需 `cli.verbose >= 1` 转换）。

#### 3. quiet 抑制摘要（D-06）

**当前 `Ok(Some(stats))` 分支（lines 74-86）：**
```rust
Ok(Some(stats)) => {
    if stats.has_fatal() {
        std::process::exit(EXIT_FATAL);
    }
    if stats.has_errors() {
        eprintln!(
            "Completed with {} error(s) ({} parse, {} export).",
            stats.total_errors, stats.parse_errors, stats.export_errors
        );
        std::process::exit(EXIT_PARTIAL);
    }
    // EXIT_CLEAN (0) is default
}
```

`quiet` 变量需要在此分支可见（当前 `run()` 函数中 `cli.quiet` 在 `Run` 匹配臂内可用），用 `if !cli.quiet` 包裹 `eprintln!` 摘要行。

---

### `src/cli/run/mod.rs` — ProgressBar 条件实例化 + verbose 文件输出

#### 1. 函数签名添加 `verbose` 参数（lines 28-33）

**当前签名：**
```rust
pub fn handle_run(
    cfg: &Config,
    quiet: bool,
    interrupted: &Arc<AtomicBool>,
    compiled_filters: Option<(CompiledMetaFilters, CompiledSqlFilters)>,
) -> Result<ErrorStats> {
```

改写后：
```rust
pub fn handle_run(
    cfg: &Config,
    quiet: bool,
    verbose: bool,
    interrupted: &Arc<AtomicBool>,
    compiled_filters: Option<(CompiledMetaFilters, CompiledSqlFilters)>,
) -> Result<ErrorStats> {
```

#### 2. ProgressBar 实例化条件（D-04 & D-05）

**当前代码（lines 114-126）：**
```rust
let show_progress = !quiet;
let pb = if show_progress {
    let bar = ProgressBar::new_spinner();
    bar.set_style(
        ProgressStyle::with_template("{msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner())
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
    );
    bar.enable_steady_tick(std::time::Duration::from_millis(80));
    Some(bar)
} else {
    None
};
```

目标：verbose 和 quiet 都不创建 ProgressBar，只有默认模式（`!quiet && !verbose`）才创建：

```rust
let show_progress = !quiet && !verbose;
let pb = if show_progress {
    // ... 完全同现有代码
    Some(bar)
} else {
    None
};
```

`show_progress` 变量已经传入并行路径函数（`process_csv_parallel`、`process_sqlite_parallel`），该变量的语义变化自动传播到并行路径，无需修改并行函数签名。

#### 3. verbose 模式下的每文件 `eprintln!`（D-03）

在顺序路径的 for 循环（lines 175-204）中，进入每个文件前：

**位置参照：** for 循环头部，在 `interrupted.load` 检查之后，`process_log_file` 调用之前，插入：
```rust
if verbose {
    eprintln!("Processing: {}", log_file.display());
}
```

这里 `log_file` 类型是 `&PathBuf`，`.display()` 是标准模式（参见 `prescan.rs` 中已有的路径格式化用法）。

#### 4. verbose 模式下的摘要差异化（D-07）

**当前摘要代码（lines 210-227）：**
```rust
if !quiet {
    let elapsed = total_start.elapsed().as_secs_f64();
    let mode_label = if use_parallel { " [parallel]" } else { "" };
    let skip_label = if skipped_files > 0 {
        format!(", {skipped_files} skipped")
    } else {
        String::new()
    };
    eprintln!(
        "\n✓ SQL Log Export Task Completed{mode_label} in {elapsed:.2}s — {total_records} records total{skip_label}",
    );
    if run_stats.has_errors() {
        eprintln!(
            "  Errors: {} total ({} parse, {} export)",
            run_stats.total_errors, run_stats.parse_errors, run_stats.export_errors
        );
    }
}
```

D-07 要求 verbose 模式额外显示每文件记录数。具体格式由 planner 设计，但改造点在此 `if !quiet` 块内，将 `verbose` 分支嵌套进来即可。顺序路径中每文件的 `processed` 返回值（line 180）可用于统计。

---

## Shared Patterns

### bool 标志声明模式
**来源：** `src/cli/opts.rs` lines 34-36（`quiet` 字段）
**应用到：** `verbose` 字段改写
```rust
#[arg(short = 'q', global = true, conflicts_with = "verbose")]
pub(crate) quiet: bool,
```

### `Option<ProgressBar>` 条件创建模式
**来源：** `src/cli/run/mod.rs` lines 114-126
**应用到：** `show_progress` 条件扩展

现有模式已经是 `Option<ProgressBar>`，只需将条件从 `!quiet` 改为 `!quiet && !verbose`。所有使用 `pb.as_ref()` 的调用点（lines 194、228）无需修改。

### `eprintln!` stderr 输出模式
**来源：** `src/cli/run/mod.rs` lines 72-75（stdin warn eprintln）
```rust
eprintln!(
    "[WARN] Transaction-level filters with stdin: pre-scan disabled, \
     degrading to per-record matching."
);
```
**应用到：** verbose 每文件输出，格式为 `eprintln!("Processing: {}", path.display())`

### 测试中调用 handle_run 的模式
**来源：** `src/cli/run/tests.rs` lines 26、67、98 等
```rust
handle_run(&cfg, true, &Arc::new(AtomicBool::new(false)), None).unwrap();
```
签名变更后，所有测试调用需补充 `verbose: bool` 参数（位于 `quiet` 之后）：
```rust
handle_run(&cfg, /*quiet=*/true, /*verbose=*/false, &Arc::new(AtomicBool::new(false)), None).unwrap();
```

---

## No Analog Found

无——所有修改均在现有文件中，且现有模式均可直接沿用。

---

## Metadata

**Analog search scope:** `src/cli/opts.rs`, `src/main.rs`, `src/cli/run/mod.rs`, `src/cli/run/tests.rs`
**Files scanned:** 4
**Pattern extraction date:** 2026-05-31
