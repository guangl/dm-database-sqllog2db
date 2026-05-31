---
phase: 48-logging
reviewed: 2026-05-31T17:09:54Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - src/cli/opts.rs
  - src/main.rs
  - src/cli/run/mod.rs
  - src/cli/run/sqlite_parallel.rs
  - tests/integration.rs
findings:
  critical: 2
  warning: 4
  info: 1
  total: 7
status: issues_found
---

# Phase 48: Code Review Report

**Reviewed:** 2026-05-31T17:09:54Z
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

审查范围涵盖 CLI 选项定义（`opts.rs`）、入口点逻辑（`main.rs`）、主运行编排（`cli/run/mod.rs`）、SQLite 并行处理路径（`sqlite_parallel.rs`）以及集成测试（`tests/integration.rs`）。

整体结构清晰，错误处理分层合理，进度/日志输出分离设计良好。但发现两处行为级缺陷：并行路径的解析错误统计完全丢失（影响退出码和错误摘要正确性），以及多错误静默丢弃；另有数处 warning 级问题。

---

## Critical Issues

### CR-01: 并行路径（CSV + SQLite）解析错误统计全部丢失，exit code 始终为 0

**File:** `src/cli/run/mod.rs:136-185`

**Issue:** `handle_run` 初始化 `run_stats = ErrorStats::default()`，随后在并行路径（`use_csv_parallel` 和 `use_sqlite_parallel` 两个分支）中从未将解析错误合并进 `run_stats`。

- **CSV 并行路径**（第 136-160 行）：`process_csv_parallel` 内部调用 `process_log_file`，`_stats` 被丢弃（`parallel.rs` 第 146 行）；返回给 `handle_run` 的签名也不携带 `ErrorStats`，所以 `run_stats` 永远保持为默认值（零错误）。
- **SQLite 并行路径**（第 161-185 行）：`process_sqlite_parallel` 收集了 `total_parse_errors` 并打印一条 `warn!`，但返回类型 `(Vec<(PathBuf, usize)>, usize)` 不包含错误统计；`handle_run` 同样无法合并，`run_stats` 仍为零错误。

后果：
1. `run_stats.has_errors()` 在并行路径下永远为 `false`，第 250-255 行的错误摘要永远不会打印。
2. `main()` 中 `stats.has_errors()` 永远为 `false`，即使并行路径存在大量解析错误，exit code 也始终为 `0`（EXIT_CLEAN），违反退出码约定。

**Fix:**

最小侵入修复方案——让 `process_sqlite_parallel` 和 `process_csv_parallel` 返回错误统计，由 `handle_run` 合并：

```rust
// sqlite_parallel.rs: 修改返回类型
pub(super) fn process_sqlite_parallel(
    ...
) -> Result<(Vec<(PathBuf, usize)>, usize, ErrorStats)> {
    let (collected, skipped, total_parse_errors) = parallel_collect(...)?;
    let mut stats = ErrorStats::default();
    for _ in 0..total_parse_errors {
        stats.add_parse_error();
    }
    // ... 写入 SQLite ...
    Ok((per_file_counts, skipped, stats))
}

// mod.rs: SQLite 并行分支，合并错误统计
let (sqlite_processed_files, parallel_skipped, parallel_stats) =
    process_sqlite_parallel(...)?;
run_stats.merge(&parallel_stats);
total_records = sqlite_processed_files.iter().map(|(_, c)| *c).sum();
skipped_files = parallel_skipped;
```

CSV 并行路径同理：`process_csv_parallel` 应将各文件的 `_stats` 聚合后一并返回。

---

### CR-02: `parallel_collect` 静默丢弃所有非第一个文件的错误

**File:** `src/cli/run/sqlite_parallel.rs:163-165`

```rust
Err(_) => {}
```

**Issue:** `parallel_collect` 遇到第一个 `Err` 时保存为 `first_err`，后续文件的所有错误完全丢弃（`Err(_) => {}`）。由于 `parallel.rs` 中同样存在相同模式（第 182 行），这是两个并行路径的共同问题。

在多文件并行场景下，如果文件 A 因路径错误（`InvalidPath`）返回 `Err`，文件 B、C 的错误会被完全丢弃。最终返回的是文件 A 的错误，但调用方不知道还有其他错误；且在 `process_sqlite_parallel` 返回 `Err` 时，`total_parse_errors` 也被抛弃（代码在 `Err` 时提前 return，跳过了 warn! 行）。

这不仅隐藏了多文件错误，在极端情况下（如 rayon 线程崩溃传播）会造成静默数据丢失。

**Fix:**

```rust
// 将全部错误聚合到错误日志，而非静默丢弃
for result in results {
    match result {
        Ok(Some((path, rows, parse_errors))) => { ... }
        Ok(None) => skipped += 1,
        Err(e) => {
            // 记录全部错误而非只保留第一个
            log::warn!("parallel collect error: {e}");
            if first_err.is_none() {
                first_err = Some(e);
            }
        }
    }
}
```

---

## Warnings

### WR-01: `_verbose` 参数在 `init_simple_logging` 和 `apply_verbosity_to_config` 中完全忽略

**File:** `src/main.rs:27-45`

**Issue:** 两个函数均接受 `_verbose: bool` 参数（前缀下划线表明"已知不使用"），但用户传入 `--verbose` 时对 init/validate 子命令的日志级别毫无影响。这与 `opts.rs` 中 `--verbose` 的描述（"Show per-file processing details"）不完全吻合，但更重要的是函数签名暗示未来会使用该参数，造成误导。

若 verbose 对 init/validate 永远无效，应直接从函数签名中移除该参数，或添加注释说明设计意图。

**Fix:**

```rust
// 如果 verbose 对 init_simple_logging 永远无效：
fn init_simple_logging(quiet: bool) {
    let filter = if quiet { log::LevelFilter::Error } else { log::LevelFilter::Info };
    let _ = env_logger::Builder::from_default_env()
        .filter_level(filter)
        .try_init();
}

// 调用方相应简化
init_simple_logging(cli.quiet);
```

---

### WR-02: `opts.rs` 中遗留 `TODO` 注释产生误导性帮助文本

**File:** `src/cli/opts.rs:53-55`

```rust
// TODO(Phase 37): replace with actual stdin pipe example
Pipe log data via stdin (requires --input flag):
    sqllog2db run -c config.toml --input -
```

**Issue:** `--input` 标志从未实现（`Run` 子命令定义中只有 `-c/--config`）。此 TODO 出现在 `after_help` 字符串中，会被 clap 直接打印到用户的 `--help` 输出，显示一个不存在的功能示例，具有误导性。

**Fix:** 移除整个"Pipe log data via stdin"示例段落，直至 stdin 管道功能正式实现并通过 `--input` 标志暴露。

---

### WR-03: Windows 平台检测逻辑静默禁用 stdin 管道，无用户提示

**File:** `src/cli/run/mod.rs:46-48`

```rust
let is_stdin_pipe =
    log_files.is_empty() && !std::io::stdin().is_terminal() && !cfg!(target_os = "windows");
```

**Issue:** `cfg!(target_os = "windows")` 在 Windows 编译时展开为 `true`，导致 `is_stdin_pipe` 永远为 `false`。若 Windows 用户在无日志文件时通过管道传入数据，程序会走 `warn!("No log files found")` 分支静默返回 `Ok`（零记录），而不告知用户 stdin 模式在该平台不受支持。

**Fix:**

在 Windows 编译时，若检测到 stdin 为非终端但无文件，明确提示用户：

```rust
#[cfg(target_os = "windows")]
let is_stdin_pipe = false;
#[cfg(not(target_os = "windows"))]
let is_stdin_pipe = log_files.is_empty() && !std::io::stdin().is_terminal();

// 在 is_stdin_pipe=false 且 log_files 为空时
if log_files.is_empty() {
    #[cfg(target_os = "windows")]
    if !std::io::stdin().is_terminal() {
        warn!("Stdin pipe mode is not supported on Windows. No log files found.");
    } else {
        warn!("No log files found");
    }
    #[cfg(not(target_os = "windows"))]
    warn!("No log files found");
    return Ok(ErrorStats::default());
}
```

---

### WR-04: `test_handle_run_interrupted` 测试断言过弱，未验证具体错误类型

**File:** `tests/integration.rs:102-118`

**Issue:** 测试只检查 `result.is_err()`，未验证返回的是 `Error::Interrupted` 而非其他错误类型。若中断检测逻辑被重构（例如提前 return `Ok(empty_stats)` 或返回不同错误），测试不会失败。此外，该测试在 `interrupted=true` 且只有单个文件时，可能走顺序路径并在第 195 行（for 循环开头）直接 break，然后在第 260 行返回 `Err(Interrupted)`——这是正确的；但如果文件列表为空（早期 return），测试期望 `is_err()` 会失败，测试的健壮性依赖文件存在这一前提。

**Fix:**

```rust
use dm_database_sqllog2db::error::Error;

let result = handle_run(&cfg, true, false, &interrupted, None);
assert!(
    matches!(result, Err(Error::Interrupted)),
    "handle_run should return Err(Interrupted) when interrupt flag is pre-set, got: {result:?}"
);
```

---

## Info

### IN-01: `test_e2e_field_projection` 中的 CSV 字段计数假设在 SQL 含逗号时会误判

**File:** `tests/integration.rs:714-729`

**Issue:** 测试通过 `line.split(',').count()` 来验证字段数为 3，并在注释中承认"如果 SQL 中包含逗号，需改用 csv crate 正确解析带引号的字段"（第 715-716 行）。当前的测试数据恰好不含逗号，但这意味着测试对字段投影的验证是脆弱的——如果 `write_test_log` 将来修改生成含逗号的 SQL，该断言会误报失败。

**Fix:** 使用 `csv` crate 解析输出文件，或固定断言 header 行精确内容（已有此断言），对数据行仅验证记录数而不用简单 split 计算字段数。

---

_Reviewed: 2026-05-31T17:09:54Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
