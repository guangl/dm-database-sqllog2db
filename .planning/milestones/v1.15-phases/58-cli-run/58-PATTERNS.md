# Phase 58: cli/run 函数清理 - Pattern Map

**Mapped:** 2026-06-02
**Files analyzed:** 1 (src/cli/run/mod.rs)
**Analogs found:** 5 / 5 (全部来自同一文件内的子模块及目标文件本身)

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/cli/run/mod.rs` (`resolve_input_files`) | utility (private fn) | request-response | `src/cli/run/mod.rs` lines 34–60 (inline block) | exact — 直接提取 |
| `src/cli/run/mod.rs` (`merge_trxid_prescan`) | utility (private fn) | request-response | `src/cli/run/mod.rs` lines 62–92 (inline block) | exact — 直接提取 |
| `src/cli/run/mod.rs` (`make_progress_bar`) | utility (private fn) | request-response | `src/cli/run/mod.rs` lines 112–123 (inline block) | exact — 直接提取 |
| `src/cli/run/mod.rs` (`run_sequential`) | orchestrator (private fn) | streaming/CRUD | `src/cli/run/mod.rs` lines 183–229 (inline block) | exact — 直接提取 |
| `src/cli/run/mod.rs` (`print_run_summary`) | utility (private fn) | request-response | `src/cli/run/mod.rs` lines 230–252 (inline block) | exact — 直接提取 |
| `src/cli/run/mod.rs` (`run_csv_parallel`) | orchestrator (private fn) | streaming/parallel | `src/cli/run/mod.rs` lines 132–156 (inline block) | exact — 直接提取（研究报告 Pitfall 3 确认必须提取） |
| `src/cli/run/mod.rs` (`run_sqlite_parallel`) | orchestrator (private fn) | streaming/parallel | `src/cli/run/mod.rs` lines 157–182 (inline block) | exact — 直接提取 |

本 phase 只修改一个文件（`src/cli/run/mod.rs`），所有"analog"均为该文件内的现有代码块。

---

## Pattern Assignments

### `resolve_input_files` (utility, request-response)

**Analog:** `src/cli/run/mod.rs` lines 34–60

**Source block to extract** (lines 34–60):
```rust
let log_files = SqllogParser::new(cfg.sqllog.inputs.clone()).log_files()?;
let mut run_stats = ErrorStats::default();  // NOTE: run_stats 不属于此函数，只提取下面部分

// Stdin pipe mode: fall back when no log files found AND stdin is not a terminal.
#[cfg(target_os = "windows")]
let is_stdin_pipe = false;
#[cfg(not(target_os = "windows"))]
let is_stdin_pipe = log_files.is_empty() && !std::io::stdin().is_terminal();
let log_files = if is_stdin_pipe {
    info!("No log files found, reading from stdin (pipe mode)");
    vec![std::path::PathBuf::from("/dev/stdin")]
} else if log_files.is_empty() {
    #[cfg(target_os = "windows")]
    if !std::io::stdin().is_terminal() {
        warn!("Stdin pipe mode is not supported on Windows. No log files found.");
    }
    return Err(crate::error::Error::Parser(
        crate::error::ParserError::NoFilesFound {
            inputs: cfg.sqllog.inputs.clone(),
        },
    ));
} else {
    log_files
};
```

**目标函数签名：**
```rust
fn resolve_input_files(cfg: &Config) -> Result<(Vec<PathBuf>, bool)> {
    // 返回 (log_files, is_stdin_pipe)
}
```

**需要的额外 import：**
- `std::io::IsTerminal` — 已在文件顶部（line 6）
- `SqllogParser` — 已在文件顶部（line 4，隐含于 `use crate::parser::SqllogParser`）
- `log::{info, warn}` — 已在文件顶部（line 5）

**注意：** `run_stats` 的声明（`let mut run_stats = ErrorStats::default()`）保留在 `handle_run` 中，不属于此函数。

---

### `merge_trxid_prescan` (utility, request-response)

**Analog:** `src/cli/run/mod.rs` lines 62–92

**Source block to extract** (lines 64–92) — 当前使用了 `owned_cfg` 局部变量模式，提取后改为 D-03/D-04 的 `Option<Config>` 返回模式：

**当前代码（lines 64–92）:**
```rust
let owned_cfg;
let final_cfg: &Config = if cfg
    .filter
    .as_ref()
    .is_some_and(crate::pipeline::FiltersFeature::has_transaction_filters)
{
    if is_stdin_pipe {
        warn!(
            "Transaction-level filters are configured but stdin pipe mode \
             cannot pre-scan for transaction IDs. Degrading to per-record matching \
             (transaction integrity not guaranteed)."
        );
        eprintln!(
            "[WARN] Transaction-level filters with stdin: pre-scan disabled, \
             degrading to per-record matching."
        );
        cfg
    } else {
        let extra_trxids = scan_for_trxids_by_transaction_filters(&log_files, cfg, jobs)?;
        let mut tmp = cfg.clone();
        if let Some(f) = &mut tmp.filter {
            f.merge_found_trxids(extra_trxids);
        }
        owned_cfg = tmp;
        &owned_cfg
    }
} else {
    cfg
};
```

**目标函数签名（D-03/D-04）:**
```rust
fn merge_trxid_prescan(
    cfg: &Config,
    log_files: &[PathBuf],
    jobs: usize,
    is_stdin_pipe: bool,
) -> Result<Option<Config>> {
    // None = 无需预扫描，Some(merged_cfg) = 预扫描完成
}
```

**调用方模式（D-04，原 owned_cfg 替换为此模式）:**
```rust
let merged = merge_trxid_prescan(cfg, &log_files, jobs, is_stdin_pipe)?;
let final_cfg: &Config = merged.as_ref().unwrap_or(cfg);
```

**关键语义保留：**
- `warn!` 和 `eprintln!` 消息文案一字不差保留
- `scan_for_trxids_by_transaction_filters` 调用方式不变
- `merge_found_trxids` 调用不变

**需要的 use：**
- `crate::pipeline::FiltersFeature` — 已在 `filter_processor.rs` 中，但 `mod.rs` 中通过 `build_pipeline` 间接使用；此函数直接使用 `FiltersFeature::has_transaction_filters` 需确认可见性（trait 在 `crate::pipeline` 中 pub）

---

### `make_progress_bar` (utility, request-response)

**Analog:** `src/cli/run/mod.rs` lines 112–123

**Source block to extract** (lines 112–123):
```rust
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

**目标函数签名：**
```rust
fn make_progress_bar(show_progress: bool) -> Option<ProgressBar> {
    // ~12 行，完全在限制内
}
```

**需要的 import：**
- `indicatif::{ProgressBar, ProgressStyle}` — 已在文件顶部（line 18）

---

### `run_sequential` (orchestrator, streaming/CRUD)

**Analog:** `src/cli/run/mod.rs` lines 183–229（顺序处理 else 分支）

**Source block to extract** (lines 184–229):
```rust
let mut exporter_manager = ExporterManager::from_config(final_cfg)?;
exporter_manager.initialize()?;
info!("Parsing and exporting SQL logs...");
let mut params_buffer = crate::pipeline::normalizer::ParamBuffer::default();
let mut ns_scratch: Vec<u8> = Vec::with_capacity(4096);
let mut per_file_counts: Vec<(std::path::PathBuf, usize)> =
    Vec::with_capacity(log_files.len());
for (idx, log_file) in log_files.iter().enumerate() {
    if interrupted.load(Ordering::Relaxed) {
        break;
    }
    if verbose {
        eprintln!("Processing: {}", log_file.display());
    }
    let (processed, file_stats) = process_log_file(
        &log_file.to_string_lossy(),
        idx + 1,
        log_files.len(),
        &mut exporter_manager,
        &pipeline,
        show_progress,
        None,
        interrupted,
        do_normalize,
        placeholder_override,
        &mut params_buffer,
        &mut ns_scratch,
        true,
        pb.as_ref(),
    )?;
    total_records += processed;
    per_file_counts.push((log_file.clone(), processed));
    run_stats.merge(&file_stats);
    if file_stats.has_fatal() {
        return Err(Error::Export(crate::error::ExportError::WriteFailed {
            path: log_file.into(),
            reason: file_stats.fatal_error.unwrap_or_default(),
        }));
    }
}
exporter_manager.finalize()?;
if !quiet {
    exporter_manager.log_stats();
}
per_file_counts
```

**目标函数签名（含 clippy allow，参见 RESEARCH.md Pitfall 1）:**
```rust
#[allow(clippy::too_many_arguments)]
fn run_sequential(
    log_files: &[PathBuf],
    final_cfg: &Config,
    pipeline: &Pipeline,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    field_mask: FieldMask,
    ordered_indices: &[usize],
    verbose: bool,
    quiet: bool,
    show_progress: bool,
    pb: Option<&ProgressBar>,
    interrupted: &Arc<AtomicBool>,
) -> Result<(Vec<(PathBuf, usize)>, ErrorStats)>
```

**行数控制 (D-05)：**
- 函数提取后约 45 行，超出限制
- 优选方案：将 `exporter_manager.finalize()` + `exporter_manager.log_stats()` 移至调用方，减少 4 行
- 同时将 `has_fatal()` 的错误构造内联为单行表达式可再减 1 行
- 目标：函数体 ≤40 行
- 返回类型中包含 `ErrorStats`（`run_stats.merge` 在函数内部，返回累计的 file-level stats）

**所有权注意事项 (RESEARCH.md Pitfall 2)：**
- `total_records` 在调用方 `handle_run` 中更新，需从返回的 `Vec<(PathBuf, usize)>` 中 `.iter().map(|(_, c)| *c).sum()` 重新计算
- 若 finalize 移至调用方，函数返回 `(per_file_counts, run_stats)` 即可，`ExporterManager` 在函数内 drop（finalize 已在返回前调用 OR 移至调用方后由调用方持有并调用）

**需要的 import：**
- `ExporterManager` — 已在文件顶部（line 3）
- `crate::pipeline::normalizer::ParamBuffer` — 在 processor.rs 中 use，mod.rs 需直接引用
- `log::info` — 已在文件顶部（line 5）
- `Ordering` — 已在文件顶部（line 8）

---

### `print_run_summary` (utility, request-response)

**Analog:** `src/cli/run/mod.rs` lines 230–252

**Source block to extract** (lines 230–252):
```rust
if !quiet {
    let elapsed = total_start.elapsed().as_secs_f64();
    let mode_label = if use_parallel { " [parallel]" } else { "" };
    let skip_label = if skipped_files > 0 {
        format!(", {skipped_files} skipped")
    } else {
        String::new()
    };
    if verbose && !processed_files.is_empty() {
        for (path, count) in &processed_files {
            eprintln!("Processed: {} — {} records", path.display(), count);
        }
    }
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

**目标函数签名：**
```rust
fn print_run_summary(
    quiet: bool,
    verbose: bool,
    use_parallel: bool,
    elapsed: f64,
    processed_files: &[(PathBuf, usize)],
    total_records: usize,
    skipped_files: usize,
    run_stats: &ErrorStats,
)
```

**注意：** `elapsed` 需由调用方计算（`total_start.elapsed().as_secs_f64()`）后传入，`total_start` 保留在 `handle_run` 中。

---

### `run_csv_parallel` (orchestrator, streaming/parallel)

**Analog:** `src/cli/run/mod.rs` lines 132–156

**Source block to extract** (lines 132–156):
```rust
if verbose {
    eprintln!(
        "Processing {} files in parallel ({} jobs)",
        log_files.len(),
        jobs
    );
}
info!("Parsing and exporting SQL logs (parallel, {jobs} jobs)...");
let (csv_processed_files, parallel_skipped, csv_parallel_stats) = process_csv_parallel(
    &log_files,
    final_cfg,
    &pipeline,
    jobs,
    show_progress,
    interrupted,
    do_normalize,
    placeholder_override,
    field_mask,
    &ordered_indices,
)?;
run_stats.merge(&csv_parallel_stats);
total_records = csv_processed_files.iter().map(|(_, c)| *c).sum();
skipped_files = parallel_skipped;
csv_processed_files
```

**目标函数签名：**
```rust
fn run_csv_parallel(
    log_files: &[PathBuf],
    final_cfg: &Config,
    pipeline: &Pipeline,
    jobs: usize,
    show_progress: bool,
    interrupted: &Arc<AtomicBool>,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    field_mask: FieldMask,
    ordered_indices: &[usize],
    verbose: bool,
) -> Result<(Vec<(PathBuf, usize)>, usize, ErrorStats)>
// 返回 (processed_files, skipped_files, merged_stats)
```

**调用方消费模式（handle_run 中）：**
```rust
let (csv_processed_files, parallel_skipped, csv_stats) =
    run_csv_parallel(...)?;
run_stats.merge(&csv_stats);
total_records = csv_processed_files.iter().map(|(_, c)| *c).sum();
skipped_files = parallel_skipped;
csv_processed_files  // 作为 processed_files 的值
```

---

### `run_sqlite_parallel` (orchestrator, streaming/parallel)

**Analog:** `src/cli/run/mod.rs` lines 157–182

与 `run_csv_parallel` 结构完全对称，调用 `process_sqlite_parallel` 而非 `process_csv_parallel`。

**目标函数签名：**
```rust
fn run_sqlite_parallel(
    log_files: &[PathBuf],
    final_cfg: &Config,
    pipeline: &Pipeline,
    jobs: usize,
    show_progress: bool,
    interrupted: &Arc<AtomicBool>,
    do_normalize: bool,
    placeholder_override: Option<bool>,
    field_mask: FieldMask,
    ordered_indices: &[usize],
    verbose: bool,
) -> Result<(Vec<(PathBuf, usize)>, usize, ErrorStats)>
```

---

## Shared Patterns

### `#[allow(clippy::too_many_arguments)]` 模式
**Source:** `src/cli/run/mod.rs`（新增，参见 RESEARCH.md Pitfall 1）
**Apply to:** `run_sequential`（12 个参数，超过 clippy 默认阈值 7）
```rust
#[allow(clippy::too_many_arguments)]
fn run_sequential(...) -> ... { ... }
```

### `Option<Config>` + `unwrap_or` 借用模式（D-04）
**Source:** CONTEXT.md D-04
**Apply to:** `merge_trxid_prescan` 的调用方（`handle_run` 中）
```rust
let merged = merge_trxid_prescan(cfg, &log_files, jobs, is_stdin_pipe)?;
let final_cfg: &Config = merged.as_ref().unwrap_or(cfg);
```
`merged` 必须在 `final_cfg` 整个使用范围内保持存活（参见 RESEARCH.md Pitfall 4）。

### `Result<T>` + `?` 传播模式
**Source:** `src/cli/run/mod.rs` lines 26–260
**Apply to:** 所有返回 `Result<_>` 的提取函数
所有错误通过 `?` 向上传播，不在私有函数内打印摘要错误（除原有的 `warn!`/`eprintln!`）。

### `total_records` 聚合模式
**Source:** `src/cli/run/mod.rs` lines 154, 180
**Apply to:** `run_csv_parallel`、`run_sqlite_parallel` 的调用方
并行路径的 total_records 由调用方从返回的 `Vec<(PathBuf, usize)>` 重新计算：
```rust
total_records = processed_files.iter().map(|(_, c)| *c).sum();
```
顺序路径的 total_records 可在 `run_sequential` 内部累加后单独返回，或由调用方从结果 Vec 计算。

---

## handle_run 提取后的骨架（供 Planner 参考）

```rust
pub fn handle_run(cfg: &Config, quiet: bool, verbose: bool, interrupted: &Arc<AtomicBool>) -> Result<ErrorStats> {
    let total_start = Instant::now();
    let mut run_stats = ErrorStats::default();
    let (log_files, is_stdin_pipe) = resolve_input_files(cfg)?;
    let jobs = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let merged = merge_trxid_prescan(cfg, &log_files, jobs, is_stdin_pipe)?;
    let final_cfg: &Config = merged.as_ref().unwrap_or(cfg);
    let pipeline = build_pipeline(final_cfg);
    // [~18 行] field_mask, ordered_indices, do_normalize, placeholder_override
    let show_progress = !quiet && !verbose;
    let pb = make_progress_bar(show_progress);
    let mut total_records = 0usize;
    let mut skipped_files = 0usize;
    let use_csv_parallel = jobs > 1 && log_files.len() > 1 && !is_stdin_pipe && final_cfg.exporter.csv.is_some();
    let use_sqlite_parallel = jobs > 1 && log_files.len() > 1 && !is_stdin_pipe && final_cfg.exporter.sqlite.is_some();
    let use_parallel = use_csv_parallel || use_sqlite_parallel;
    let processed_files: Vec<(PathBuf, usize)> = if use_csv_parallel {
        let (files, skipped, stats) = run_csv_parallel(&log_files, final_cfg, &pipeline, jobs, show_progress, interrupted, do_normalize, placeholder_override, field_mask, &ordered_indices, verbose)?;
        run_stats.merge(&stats); total_records = files.iter().map(|(_, c)| *c).sum(); skipped_files = skipped; files
    } else if use_sqlite_parallel {
        let (files, skipped, stats) = run_sqlite_parallel(&log_files, final_cfg, &pipeline, jobs, show_progress, interrupted, do_normalize, placeholder_override, field_mask, &ordered_indices, verbose)?;
        run_stats.merge(&stats); total_records = files.iter().map(|(_, c)| *c).sum(); skipped_files = skipped; files
    } else {
        let (files, seq_stats) = run_sequential(&log_files, final_cfg, &pipeline, do_normalize, placeholder_override, field_mask, &ordered_indices, verbose, quiet, show_progress, pb.as_ref(), interrupted)?;
        run_stats.merge(&seq_stats); total_records = files.iter().map(|(_, c)| *c).sum(); files
    };
    print_run_summary(quiet, verbose, use_parallel, total_start.elapsed().as_secs_f64(), &processed_files, total_records, skipped_files, &run_stats);
    if let Some(pb) = &pb { pb.finish_and_clear(); }
    if interrupted.load(Ordering::Relaxed) { return Err(Error::Interrupted); }
    Ok(run_stats)
}
```

---

## No Analog Found

无。所有提取函数均来自目标文件内的现有代码块，无需参考外部 codebase 模式。

---

## Metadata

**Analog search scope:** `src/cli/run/mod.rs`（唯一修改文件）
**Files scanned:** 5（mod.rs, processor.rs, prescan.rs, parallel.rs, sqlite_parallel.rs）
**Pattern extraction date:** 2026-06-02
