# Phase 58: cli/run 函数清理 - Research

**Researched:** 2026-06-02
**Domain:** Rust 代码重构 — 函数提取 / 行数约束
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** 从 `handle_run` 提取以下 5 个私有辅助函数（按语义边界划分）：
  1. `resolve_input_files(cfg: &Config) -> Result<(Vec<PathBuf>, bool)>` — stdin pipe mode 检测 + 日志文件列表解析（对应原 34–60 行，约 27 行）
  2. `merge_trxid_prescan(cfg: &Config, log_files: &[PathBuf], jobs: usize, is_stdin_pipe: bool) -> Result<Option<Config>>` — 事务过滤器条件性预扫描 + config 合并（对应原 62–92 行，约 31 行）
  3. `make_progress_bar(show_progress: bool) -> Option<ProgressBar>` — 进度条创建（对应原 112–123 行，约 12 行）
  4. `run_sequential(log_files: &[PathBuf], final_cfg: &Config, pipeline: &Pipeline, do_normalize: bool, placeholder_override: Option<bool>, field_mask: FieldMask, ordered_indices: &[usize], verbose: bool, quiet: bool, show_progress: bool, pb: Option<&ProgressBar>, interrupted: &Arc<AtomicBool>) -> Result<(Vec<(PathBuf, usize)>, ErrorStats)>` — 顺序处理路径（对应原 184–229 行）
  5. `print_run_summary(quiet: bool, verbose: bool, use_parallel: bool, elapsed: f64, processed_files: &[(PathBuf, usize)], total_records: usize, skipped_files: usize, run_stats: &ErrorStats)` — 摘要输出（对应原 230–252 行，约 23 行）
- **D-02:** 提取后 `handle_run` 本体不超过 40 行：调用上述函数 + 字段配置计算（约 18 行）+ 并行路径选择 + 进度条收尾。
- **D-03:** `merge_trxid_prescan` 返回 `Option<Config>`：`None` = 无需预扫描，`Some(merged_cfg)` = 预扫描完成。
- **D-04:** 调用方模式：`let merged = merge_trxid_prescan(...)?; let final_cfg: &Config = merged.as_ref().unwrap_or(cfg);`
- **D-05:** `run_sequential` 提取后约 46 行，需通过简化 fatal 错误处理或将 finalize/log_stats 后置到调用方来控制到 ≤40 行。
- **D-06:** 私有函数命名反映单一职责，禁止使用 `helper`、`util`、`misc` 等无意义后缀。
- **D-07:** 仅拆分 `src/cli/run/mod.rs`，不触碰子模块文件；提取出的私有函数定义在同一文件内。

### Claude's Discretion

无（所有关键决策已锁定）。

### Deferred Ideas (OUT OF SCOPE)

- 子模块函数（`process_log_file` 等）的参数重构
- `ProcessingParams` struct 封装多参数
- `src/cli/run/processor.rs` 内部函数长度检查

</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CLEAN-02 | cli/run 模块中超 40 行的函数提取为私有函数（仅拆分确实超长的，不做预防性拆分） | `handle_run` 当前 234 行，唯一目标。测量结果确认各语义块行数，提取方案在本文档详述。 |

</phase_requirements>

---

## Summary

Phase 58 是纯粹的代码重构：将 `src/cli/run/mod.rs` 中唯一的公共函数 `handle_run`（当前 234 行，第 26–260 行）拆分为 5 个私有辅助函数，使每个函数体不超过 40 行。CONTEXT.md 已锁定所有拆分决策，本次研究的工作是精确测量当前代码的行数分布，发现潜在的行数超限风险，并为 Planner 提供可执行的实现指引。

**关键发现：** `run_sequential` 提取后的函数体为 45 行（不含 else{} 括号），超出 40 行限制。D-05 提供了两种解决路径：①将 `exporter_manager.finalize()` + `log_stats()` 移至调用方（减少 4 行），②同时简化 `has_fatal()` 检查（再减 1 行），两者合用可精确达到 40 行。此外，`handle_run` 提取后的本体行数需要仔细规划：并行路径 arm（CSV 25 行 + SQLite 26 行）必须通过提取或内联折叠大幅缩短，否则 `handle_run` 本体本身也会超限。

**核心约束：** Phase 57 新增的 e2e 测试（`tests/integration.rs` 中的 `test_cli_run_csv_output_*`、`test_cli_run_sqlite_*`、`test_cli_init_*`）是本次重构的安全网，`cargo test`（68 个测试 + 1 个忽略）必须全部通过。

**Primary recommendation:** 按 CONTEXT.md D-01~D-07 严格实施，额外注意 `run_sequential` 的行数控制方案（D-05 首选：finalize/log_stats 移至调用方），以及 `handle_run` 中并行路径 arm 的折叠。

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| 文件输入解析 | CLI Handler (`handle_run`) | Parser (`SqllogParser`) | `resolve_input_files` 封装此逻辑 |
| 事务过滤器预扫描 | CLI Handler (`handle_run`) | Pipeline (`prescan.rs`) | `merge_trxid_prescan` 封装此逻辑 |
| 字段配置计算 | CLI Handler (`handle_run`) | Pipeline (`pipeline/mod.rs`) | 内联保留在 `handle_run`，依赖 `FieldMask`/`OutputConfig` |
| 进度条管理 | CLI Handler (`handle_run`) | — | `make_progress_bar` 封装创建；收尾保留在 `handle_run` |
| 并行路径分发 | CLI Handler (`handle_run`) | `parallel.rs`/`sqlite_parallel.rs` | 条件判断内联在 `handle_run` |
| 顺序处理路径 | `run_sequential` (extracted) | `processor.rs` | 循环 + ExporterManager 生命周期由此函数管理 |
| 运行摘要输出 | `print_run_summary` (extracted) | — | 纯格式化/打印，无副作用 |

---

## Standard Stack

### Core（无新依赖，本 Phase 全为重构）

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `indicatif` | already in Cargo.toml | `ProgressBar`/`ProgressStyle` 类型 | 现有依赖，`make_progress_bar` 返回类型 `Option<ProgressBar>` |
| `std::sync::Arc<AtomicBool>` | std | 中断信号传递 | 现有模式，`run_sequential` 签名中需要 |
| `thiserror` / `crate::error` | already in Cargo.toml | `Error`/`ErrorStats`/`Result` 类型 | 现有错误处理框架 |

**本 Phase 不引入任何新依赖。** [VERIFIED: 代码直接读取]

### 关键类型签名（提取函数时使用）

```rust
// resolve_input_files 返回类型
-> Result<(Vec<PathBuf>, bool)>   // (log_files, is_stdin_pipe)

// merge_trxid_prescan 返回类型（D-03/D-04）
-> Result<Option<Config>>         // None = 无需预扫描

// make_progress_bar 返回类型
-> Option<ProgressBar>

// run_sequential 返回类型
-> Result<(Vec<(PathBuf, usize)>, ErrorStats)>   // (per_file_counts, run_stats)
// 注：D-05 若选择移出 finalize，则不返回 ExporterManager（在函数内部 finalize）
//     或返回 (Vec<(PathBuf, usize)>, ErrorStats, ExporterManager)

// print_run_summary 无返回值
-> ()
```

[ASSUMED] — `placeholder_override` 类型为 `Option<bool>`（来自 `NormalizeConfig::placeholder_override()`，已确认签名）[VERIFIED: 代码直接读取]

---

## Package Legitimacy Audit

本 Phase 不安装任何新包，跳过此节。

---

## Architecture Patterns

### System Architecture Diagram

```
handle_run(cfg, quiet, verbose, interrupted)
    │
    ├─► resolve_input_files(cfg)
    │       → (log_files: Vec<PathBuf>, is_stdin_pipe: bool)
    │
    ├─► [inline] jobs = available_parallelism
    │
    ├─► merge_trxid_prescan(cfg, &log_files, jobs, is_stdin_pipe)
    │       → Option<Config>   [None = no prescan needed]
    │       → let final_cfg = merged.as_ref().unwrap_or(cfg)
    │
    ├─► [inline] build_pipeline + field config (18 lines)
    │       build_pipeline(final_cfg) → Pipeline
    │       field_mask, ordered_indices, do_normalize, placeholder_override
    │
    ├─► make_progress_bar(show_progress) → Option<ProgressBar>
    │
    ├─► [inline] parallel flags (use_csv_parallel, use_sqlite_parallel, use_parallel)
    │
    ├─► processed_files = if use_csv_parallel { process_csv_parallel(...) }
    │                      else if use_sqlite_parallel { process_sqlite_parallel(...) }
    │                      else { run_sequential(...) → (per_file_counts, stats) }
    │
    ├─► print_run_summary(quiet, verbose, use_parallel, elapsed, ...)
    │
    ├─► [inline] pb.finish_and_clear()
    ├─► [inline] interrupt check → Err(Error::Interrupted)
    └─► Ok(run_stats)
```

### Recommended Project Structure

```
src/cli/run/
├── mod.rs              — handle_run + 5 new private fns（唯一修改文件）
├── filter_processor.rs — 不修改
├── parallel.rs         — 不修改
├── prescan.rs          — 不修改
├── processor.rs        — 不修改
├── sqlite_parallel.rs  — 不修改
└── tests.rs            — 不修改
```

### Pattern 1: 语义块提取为私有函数

**What:** 将 `handle_run` 中有自然语义边界的代码块提取为私有函数（`fn name(...) -> ...`），保留在同一文件末尾。

**When to use:** 代码块：(1) 有清晰的单一职责，(2) 输入/输出在语义上是独立的，(3) 提取后不需要访问 `handle_run` 的局部变量（除参数传递外）。

**Example:**

```rust
// Source: CONTEXT.md D-04 pattern (CONTEXT.md:34-38) [ASSUMED]
fn merge_trxid_prescan(
    cfg: &Config,
    log_files: &[PathBuf],
    jobs: usize,
    is_stdin_pipe: bool,
) -> Result<Option<Config>> {
    if cfg.filter.as_ref().is_some_and(FiltersFeature::has_transaction_filters) {
        if is_stdin_pipe {
            warn!("Transaction-level filters...");
            eprintln!("[WARN] ...");
            return Ok(None);
        }
        let extra_trxids = scan_for_trxids_by_transaction_filters(log_files, cfg, jobs)?;
        let mut tmp = cfg.clone();
        if let Some(f) = &mut tmp.filter {
            f.merge_found_trxids(extra_trxids);
        }
        Ok(Some(tmp))
    } else {
        Ok(None)
    }
}

// 调用方 (D-04 pattern):
let merged = merge_trxid_prescan(cfg, &log_files, jobs, is_stdin_pipe)?;
let final_cfg: &Config = merged.as_ref().unwrap_or(cfg);
```

### Anti-Patterns to Avoid

- **提取后返回所有权再借用：** `run_sequential` 内部不能借用已 move 进去的 ExporterManager 再返回引用——如果 D-05 选择"移出 finalize"方案，需返回 `ExporterManager` 值而非引用。
- **`#[allow(clippy::too_many_arguments)]` 滥用：** `run_sequential` 签名有 12+ 个参数，clippy 会警告。应在该函数上标注此 allow，而不是全局关闭。
- **并行路径 arm 未提取导致 handle_run 超限：** 如果 CSV/SQLite 并行路径 arm（各 25/26 行）保留在 `handle_run` 内联，`handle_run` 本体必然超过 40 行（见下方行数分析）。

---

## Critical: handle_run 行数精确分析

### 当前各语义块行数（实测）[VERIFIED: 代码直接读取]

| 行号区间 | 语义块 | 实测行数 | 处置方式 |
|---------|--------|---------|---------|
| 34–60 | resolve_input_files 目标 | 27 | 提取为函数 → `resolve_input_files` |
| 61 | jobs 计算 | 1 | 内联保留 |
| 62–92 | merge_trxid_prescan 目标 | 31 | 提取为函数 → `merge_trxid_prescan` |
| 93–110 | 字段配置（pipeline + field_mask + ordered_indices + do_normalize + placeholder_override） | 18 | 内联保留（D-02 明确） |
| 111–123 | 进度条创建 | 13 | 提取为函数 → `make_progress_bar` |
| 124–131 | 并行标志 + use_parallel | 8 | 内联保留 |
| 132–156 | CSV 并行路径 arm | **25** | 见下方分析 |
| 157–182 | SQLite 并行路径 arm | **26** | 见下方分析 |
| 183–229 | 顺序处理路径 | **47** | 提取为 `run_sequential`，需 D-05 处理 |
| 230–252 | 摘要输出 | 23 | 提取为 `print_run_summary` |
| 253–260 | 进度条收尾 + 中断检查 + return | 8 | 内联保留 |

### handle_run 提取后的行数估算

提取 5 个函数后，`handle_run` 内联保留部分为：

```
fn handle_run(...) {                                   // 1 (fn signature 算 4 行)
    let total_start = Instant::now();                  // 1
    let mut run_stats = ErrorStats::default();          // 1
    let (log_files, is_stdin_pipe) = resolve_input_files(cfg)?;  // 1
    let jobs = ...;                                    // 1
    let merged = merge_trxid_prescan(...)?;            // 1
    let final_cfg: &Config = merged.as_ref().unwrap_or(cfg); // 1
    let pipeline = build_pipeline(final_cfg);          // 1
    // field_mask block                                // 3
    // ordered_indices block                           // 3
    // do_normalize block                              // 3
    // placeholder_override block                      // 3
    let show_progress = !quiet && !verbose;            // 1
    let pb = make_progress_bar(show_progress);         // 1
    let mut total_records = 0usize;                    // 1
    let mut skipped_files = 0usize;                    // 1
    // parallel flags (3 行 let + use_parallel)        // 4
    let processed_files = if use_csv_parallel {
        // CSV arm: 25 行 → 必须折叠或提取
    } else if use_sqlite_parallel {
        // SQLite arm: 26 行 → 必须折叠或提取
    } else {
        run_sequential(...)?                           // 2
    };
    print_run_summary(...);                            // 1
    if let Some(pb) = &pb { pb.finish_and_clear(); }  // 2
    if interrupted.load(Ordering::Relaxed) {           // 3
        return Err(Error::Interrupted);
    }
    Ok(run_stats)                                      // 1
}                                                      // 1
```

**结论：即使不算并行 arm，内联部分已约 36–37 行。CSV arm 25 行 + SQLite arm 26 行 = 51 行必须大幅压缩，否则 `handle_run` 本体远超 40 行。**

### 并行路径 arm 的折叠方案（Planner 必须选择一种）

**方案 A：提取为两个私有函数（推荐）**

```rust
fn run_csv_parallel(...) -> Result<(Vec<(PathBuf, usize)>, usize, ErrorStats)> { ... }  // ~20 行
fn run_sqlite_parallel(...) -> Result<(Vec<(PathBuf, usize)>, usize, ErrorStats)> { ... }  // ~20 行
```

`handle_run` 中各 arm 变为 ~4 行（调用 + stats 合并），总 arm 约 10 行。

**方案 B：提取一个通用 run_parallel 函数**

因 CSV 和 SQLite 并行函数签名完全相同，可通过函数指针或枚举区分。但增加复杂度，不推荐。

**方案 C：内联折叠（不提取）**

将 verbose log 折叠为单行，将 stats 合并折叠，可将每个 arm 从 25 行压缩到 ~12 行。但 `handle_run` 本体仍需加上 36（内联）+ 12 + 12 = 60 行，仍超限。

**结论：方案 A 是唯一能让 `handle_run` ≤40 行的可行路径。** D-02 只明确说"内联并行路径选择"，但实际并行 arm 的行数要求意味着必须提取为辅助函数。Planner 应将方案 A 作为计划的一部分，与 D-01 的 5 个函数一起作为附加提取。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 函数行数验证 | 自定义脚本 | `cargo test` + 代码审查 | 测试已覆盖行为，行数通过 PR review 把关 |
| owned_cfg 生命周期管理 | Cow<Config> | D-04 的 `Option<Config>` + `unwrap_or` 模式 | 更简单，CONTEXT.md 明确锁定 |

---

## Common Pitfalls

### Pitfall 1: run_sequential 签名触发 clippy::too_many_arguments

**What goes wrong:** `run_sequential` 有 12 个参数，超过 clippy 默认阈值（7 个），触发 `-D warnings` 失败。

**Why it happens:** `process_log_file` 本身有 14 个参数，`run_sequential` 需要全部传递，加上 loop 控制参数（verbose, quiet, interrupted 等）。

**How to avoid:** 在 `run_sequential` 函数上方添加：
```rust
#[allow(clippy::too_many_arguments)]
fn run_sequential(...) -> ... { ... }
```

**Warning signs:** `cargo clippy --all-targets -- -D warnings` 报 `too_many_arguments` 错误。

### Pitfall 2: run_sequential 内 ExporterManager 所有权冲突

**What goes wrong:** 若选择 D-05 "将 finalize/log_stats 移至调用方"，需要从 `run_sequential` 返回 `ExporterManager`，但函数签名需要调整为返回 `Result<(Vec<(PathBuf, usize)>, ErrorStats, ExporterManager)>`，调用方需要相应处理。

**Why it happens:** `ExporterManager` 是具有所有权的资源，内部 `finalize()` 需要 `&mut self`，不能借用后归还。

**How to avoid:** 若选择此方案，返回类型改为三元组，调用方接收并调用 finalize/log_stats。若想保持简洁，选择 D-05 方案一（简化 fatal 错误处理逻辑）更清晰。

### Pitfall 3: handle_run 本体因并行 arm 超限

**What goes wrong:** D-01 提取 5 个函数后，若 CSV/SQLite 并行 arm 保留 inline，`handle_run` 本体约 87 行，远超 40 行。

**Why it happens:** D-02 说"并行路径选择"内联，但 arm 内部有 25+26 行代码，不是简单的函数调用。

**How to avoid:** 将 CSV 并行 arm 和 SQLite 并行 arm 分别提取为 `run_csv_parallel` 和 `run_sqlite_parallel` 私有函数（见上方分析）。

### Pitfall 4: 编译期借用检查 — D-04 模式中 merged 生命周期

**What goes wrong:** `let final_cfg: &Config = merged.as_ref().unwrap_or(cfg);` 中，`merged`（类型 `Option<Config>`）必须在 `final_cfg` 的整个使用范围内保持存活。

**Why it happens:** `final_cfg` 借用了 `merged` 内部的 `Config`，若 `merged` drop 过早，编译报错。

**How to avoid:** `merged` 必须声明在 `final_cfg` 的外层 scope，不能在 if block 内声明后立即 drop。CONTEXT.md D-04 的写法已是正确的——`let merged = ...; let final_cfg = merged.as_ref().unwrap_or(cfg);` 两行紧邻声明。

### Pitfall 5: Windows cfg 条件编译块计入行数

**What goes wrong:** `resolve_input_files` 内含 `#[cfg(target_os = "windows")]` / `#[cfg(not(target_os = "windows"))]` 条件编译块（原 39–42 行），会因平台不同而计算不同行数。

**Why it happens:** `is_stdin_pipe` 的赋值使用了平台条件编译，在 macOS 上是 1 行，在 Windows 上是 2 行。

**How to avoid:** 计算函数行数时以非 Windows（最多行）的分支为准，实际 27 行没有问题，但需确认 `resolve_input_files` 提取后也 ≤40 行。

---

## Code Examples

### D-04 调用方模式（CONTEXT.md 锁定）

```rust
// Source: CONTEXT.md D-04 (58-CONTEXT.md:35-37)
let merged = merge_trxid_prescan(cfg, &log_files, jobs, is_stdin_pipe)?;
let final_cfg: &Config = merged.as_ref().unwrap_or(cfg);
```

### clippy::too_many_arguments 抑制

```rust
// Source: ASSUMED - standard Rust clippy allow pattern
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
) -> Result<(Vec<(PathBuf, usize)>, ErrorStats)> {
    // ...
}
```

### run_sequential 内 fatal error 处理（D-05 方案一：内联简化）

原代码（3 行）：
```rust
if file_stats.has_fatal() {
    return Err(Error::Export(crate::error::ExportError::WriteFailed {
        path: log_file.into(),
        reason: file_stats.fatal_error.unwrap_or_default(),
    }));
}
```

简化为（1-2 行）[ASSUMED — 需验证语义等价性]:
```rust
// 保持原 3 行，或抽取 fatal_to_err helper（1 行 call）
```

注：D-05 优先建议"将 finalize/log_stats 移至调用方"方案，可精确减少 4 行而不改变循环内逻辑。

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in) |
| Config file | Cargo.toml (no separate test config) |
| Quick run command | `cargo test` |
| Full suite command | `cargo test --all-targets` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CLEAN-02 | handle_run 行为无变化（CSV 路径） | e2e | `cargo test test_cli_run_csv_output_header_and_row_count` | ✅ `tests/integration.rs` |
| CLEAN-02 | handle_run 行为无变化（SQLite 路径） | e2e | `cargo test test_cli_run_sqlite_output_row_count` | ✅ `tests/integration.rs` |
| CLEAN-02 | 事务过滤器预扫描路径无变化 | integration | `cargo test test_handle_run_with_transaction_filters_prescans` | ✅ `tests/integration.rs` |
| CLEAN-02 | 多文件路径无变化 | integration | `cargo test test_handle_run_multi_file` | ✅ `tests/integration.rs` |
| CLEAN-02 | verbose 模式摘要路径无变化 | integration | `cargo test test_cli_verbose_summary_includes_per_file_counts` | ✅ `tests/integration.rs` |
| CLEAN-02 | clippy 无新警告 | static | `cargo clippy --all-targets -- -D warnings` | — |
| CLEAN-02 | fmt 检查通过 | static | `cargo fmt --check` | — |

### Sampling Rate

- **每次修改后：** `cargo test` （68 个测试，运行约 1 秒）
- **Phase gate：** `cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test`

### Wave 0 Gaps

无 — 现有测试基础设施完全覆盖本 Phase 所有行为验证需求。

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `placeholder_override` 参数类型为 `Option<bool>`（`NormalizeConfig::placeholder_override()` 的返回类型） | Code Examples, run_sequential 签名 | 编译失败；但已通过代码直接读取确认（实为 VERIFIED） |
| A2 | `#[allow(clippy::too_many_arguments)]` 是抑制 clippy too_many_arguments 的正确方式 | Common Pitfalls, Code Examples | clippy 仍报错；低风险，此为标准 Rust 模式 |
| A3 | 并行路径 arm 必须提取为 `run_csv_parallel`/`run_sqlite_parallel` 才能使 `handle_run` ≤40 行 | Critical 行数分析 | 如果 Planner 找到其他折叠方式使并行 arm 缩短到 ~2 行，则此推论错误；但技术上不可能将 25 行折叠到 2 行而不提取 |

**注：A1 实际已通过代码读取 VERIFIED。**

---

## Open Questions

1. **handle_run ≤40 行：D-02 的"并行路径选择"如何内联？**
   - What we know: CSV arm 25 行，SQLite arm 26 行，内联保留后 handle_run 远超 40 行
   - What's unclear: CONTEXT.md D-02 是否允许提取 `run_csv_parallel`/`run_sqlite_parallel`（D-01 只列了 5 个函数）
   - Recommendation: Planner 应将并行路径提取（方案 A）作为实现前提，在 PLAN.md 中明确记录此调整；D-01 的 5 个函数是最小集，并行 arm 提取是行数目标的隐含需求

2. **run_sequential 中 ExporterManager 的 finalize 方案（D-05）**
   - What we know: D-05 提供两种方案，两者都可使函数 ≤40 行
   - What's unclear: 方案二（移出 finalize）改变了 `run_sequential` 的语义边界，ExporterManager 需要作为返回值或调用方显式管理
   - Recommendation: 优先方案一（内联简化 fatal 检查）保持接口整洁；仅当方案一仍超限时选方案二

---

## Environment Availability

Step 2.6: 本 Phase 为纯代码重构，无外部依赖（不引入新工具/服务），跳过此节。

---

## Security Domain

本 Phase 是代码结构重构，不引入新的输入处理逻辑、加密操作、认证或访问控制变更，安全域不适用，跳过此节。

---

## Project Constraints (from CLAUDE.md)

| Directive | Impact on Phase |
|-----------|----------------|
| 函数不超过 40 行（每个 fn 关键字开头） | 本 Phase 的核心目标，所有提取函数均须满足 |
| `cargo clippy --all-targets -- -D warnings` 必须通过无警告 | `run_sequential` 签名 `too_many_arguments` 需 `#[allow]` 处理 |
| `cargo fmt` 必须通过 | 提取函数后运行 `cargo fmt` |
| `cargo test` 全部通过 | Phase 57 e2e 测试是安全网 |
| 描述性变量名，不用单字母 | 提取函数的参数名应清晰反映语义（如 `log_files`，不用 `files`） |
| Rust 为主语言 | 无额外影响 |

---

## Sources

### Primary (HIGH confidence)
- `src/cli/run/mod.rs` 完整源码 — 直接读取，行号精确 [VERIFIED: 代码直接读取]
- `tests/integration.rs` — Phase 57 e2e 测试完整结构 [VERIFIED: 代码直接读取]
- `.planning/phases/58-cli-run/58-CONTEXT.md` — 用户锁定决策 [VERIFIED: 直接读取]
- `.planning/REQUIREMENTS.md` — CLEAN-02 需求原文 [VERIFIED: 直接读取]
- `cargo test` 运行结果 — 68 passed, 0 failed [VERIFIED: 工具执行]
- `cargo clippy` 运行结果 — Finished 无警告 [VERIFIED: 工具执行]

### Secondary (MEDIUM confidence)
- `src/cli/run/processor.rs` — `process_log_file` 签名（14 参数，`placeholder_override: Option<bool>`）[VERIFIED: 代码直接读取]
- `src/pipeline/mod.rs` — `NormalizeConfig::placeholder_override()` 返回 `Option<bool>` [VERIFIED: 代码直接读取]
- `src/exporter/mod.rs` — `ExporterManager::finalize()` 和 `log_stats()` 签名 [VERIFIED: 代码直接读取]

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — 无新包，现有代码直接读取
- Architecture: HIGH — 完全基于 CONTEXT.md 锁定决策 + 实测行数
- Pitfalls: HIGH — 基于代码实测行数分析，编译器行为有据可查
- 行数分析: HIGH — 所有数字来自 `sed -n 'X,Yp' | wc -l` 实测

**Research date:** 2026-06-02
**Valid until:** 无限期（代码重构阶段，不依赖外部 API 或版本变化）
