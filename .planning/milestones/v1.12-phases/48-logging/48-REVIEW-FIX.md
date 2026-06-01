---
phase: 48-logging
fixed_at: 2026-06-01T00:00:00Z
review_path: .planning/phases/48-logging/48-REVIEW.md
iteration: 1
findings_in_scope: 7
fixed: 5
skipped: 2
status: partial
---

# Phase 48: Code Review Fix Report

**Fixed at:** 2026-06-01
**Source review:** .planning/phases/48-logging/48-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 7
- Fixed: 5
- Skipped: 2

## Fixed Issues

### CR-01: 并行路径解析错误统计全部丢失，exit code 始终为 0

**Files modified:** `src/cli/run/parallel.rs`, `src/cli/run/sqlite_parallel.rs`, `src/cli/run/mod.rs`
**Commit:** 85c37f6
**Applied fix:**
- `process_csv_parallel` 返回类型从 `(Vec, usize)` 改为 `(Vec, usize, ErrorStats)`；在任务结果收集循环中将每个文件的 `file_stats` 合并入 `parallel_stats` 并返回
- `process_sqlite_parallel` 返回类型从 `(Vec, usize)` 改为 `(Vec, usize, ErrorStats)`；将 `total_parse_errors` 通过 `add_parse_error()` 转换为 `ErrorStats` 并返回
- `handle_run` 在 CSV 并行分支和 SQLite 并行分支分别调用 `run_stats.merge(&csv_parallel_stats)` / `run_stats.merge(&sqlite_parallel_stats)`，确保并行路径的解析错误计入全局统计、正确影响 exit code 和错误摘要打印

### CR-02: `parallel_collect` 静默丢弃所有非第一个文件的错误

**Files modified:** `src/cli/run/sqlite_parallel.rs`, `src/cli/run/parallel.rs`
**Commit:** 85c37f6 (与 CR-01 同一提交，两者改动高度耦合)
**Applied fix:**
- `sqlite_parallel.rs` 中 `parallel_collect` 的 `Err(_) => {}` 改为记录所有错误：`log::warn!("parallel collect error: {e}")` + `if first_err.is_none() { first_err = Some(e); }`
- `parallel.rs` 结果收集循环中 `Err(_) => {}` 同样改为 `log::warn!` 并保留 first_err 语义

### WR-03: Windows 平台 stdin 管道检测静默禁用无提示

**Files modified:** `src/cli/run/mod.rs`
**Commit:** 7c39ea4
**Applied fix:**
- 将单行 `is_stdin_pipe = ... && !cfg!(target_os = "windows")` 拆分为 `#[cfg(target_os = "windows")] let is_stdin_pipe = false;` 和 `#[cfg(not(target_os = "windows"))] let is_stdin_pipe = ...;`
- 在 `log_files.is_empty()` 分支内添加 `#[cfg(target_os = "windows")] if !std::io::stdin().is_terminal() { warn!("Stdin pipe mode is not supported on Windows. No log files found."); }`，向 Windows 用户明确说明不支持 stdin 管道模式

### WR-04: `test_handle_run_interrupted` 断言过弱

**Files modified:** `tests/integration.rs`, `src/lib.rs`, `src/error.rs`
**Commit:** f1159e9
**Applied fix:**
- 将 `src/lib.rs` 中 `pub(crate) mod error` 改为 `pub mod error`，使集成测试可以访问 `dm_database_sqllog2db::error::Error`
- 为 `ErrorStats::has_errors`, `ErrorStats::has_fatal`, `Error::is_fatal`, `Error::severity`, `Error::suggestion` 加 `#[must_use]`（模块公开后 clippy::must_use_candidate 对这五个方法生效）
- 测试断言从 `result.is_err()` 改为 `matches!(result, Err(dm_database_sqllog2db::error::Error::Interrupted))`，并配备描述性失败消息

### IN-01: `test_e2e_field_projection` 使用 `split(',').count()` 脆弱字段计数

**Files modified:** `tests/integration.rs`
**Commit:** 53a5c21
**Applied fix:**
- 删除遍历每行做 `line.split(',').count() == 3` 的 for 循环
- 保留 header 精确内容断言 (`"ts,username,sql"`) 和数据行行数断言 (`data_lines.len() == 3`)；header 已充分验证字段投影正确，无需对数据行再做逗号分割计数

## Skipped Issues

### WR-01: `_verbose` 参数在 `init_simple_logging` 和 `apply_verbosity_to_config` 中完全忽略

**File:** `src/main.rs:27-45`
**Reason:** skipped: code context differs from review — phase 46 已修复，当前 `init_simple_logging` 签名为 `fn init_simple_logging(quiet: bool)`，不含 `_verbose` 参数；`apply_verbosity_to_config` 同样已正确实现 verbose/quiet 分支
**Original issue:** 两个函数均接受 `_verbose: bool` 参数但忽略，verbose 对 init/validate 子命令无效

### WR-02: `opts.rs` 中遗留 `TODO` 注释产生误导性帮助文本

**File:** `src/cli/opts.rs:53-55`
**Reason:** skipped: code context differs from review — phase 47 已修复，当前 `after_help` 字符串中不含任何 TODO 注释，`--input` 标志已实现（`Run` 子命令含 `-i/--input` 参数）
**Original issue:** `--input` 标志未实现但 after_help 中有示例，具有误导性

---

_Fixed: 2026-06-01_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
