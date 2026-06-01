---
phase: 48-logging
plan: 02
subsystem: cli
tags: [verbose, summary, per-file-counts, sqlite-parallel, csv-parallel, stderr]

# Dependency graph
requires:
  - phase: 48-logging
    plan: 01
    provides: processed_files Vec<(PathBuf, usize)> verbose/quiet 信号路径
provides:
  - verbose 摘要差异化：Processed: <path> — N records 每文件明细行
  - sqlite_parallel 返回值对齐到 csv_parallel 形态
  - 统一 processed_files: Vec<(PathBuf, usize)> 跨三条执行路径
  - 2 个端到端测试覆盖 verbose/default 摘要行为
affects: [49-glob]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - parallel_collect 返回 Vec<(PathBuf, rows)> 保留 path 关联
    - block-expression if/else if/else 返回统一 Vec<(PathBuf, usize)>，消除 unused_assignment lint
    - verbose 摘要在 if !quiet 块内输出，位于完成行之前

key-files:
  created: []
  modified:
    - src/cli/run/sqlite_parallel.rs
    - src/cli/run/mod.rs
    - tests/integration.rs

key-decisions:
  - "parallel_collect 改为返回 (PathBuf, rows) 元组，避免 zip 时 skipped 文件导致的索引错位"
  - "merge_and_write 函数移除，逻辑内联到 process_sqlite_parallel，减少不必要的抽象"
  - "processed_files 通过 block-expression 赋值，消除 unused_assignment clippy 错误"
  - "Task 1 和 Task 2 核心逻辑合并提交：两者因 clippy 约束不可分割"

# Metrics
duration: 25min
completed: 2026-06-01
---

# Phase 48 Plan 02: 摘要差异化（per-file 计数） Summary

**统一三条执行路径的每文件计数返回值，并在 verbose 模式摘要前输出 `Processed: <path> — N records` 明细行；端到端测试验证差异化行为**

## Performance

- **Duration:** 25 min
- **Started:** 2026-06-01T02:30:00Z
- **Completed:** 2026-06-01T02:55:00Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- `sqlite_parallel::parallel_collect` 返回 `Vec<(PathBuf, Vec<...>)>` 保留 path 与记录对应关系
- `process_sqlite_parallel` 签名从 `Result<(usize, usize)>` 改为 `Result<(Vec<(PathBuf, usize)>, usize)>`，与 `process_csv_parallel` 对齐
- `merge_and_write` 辅助函数移除，逻辑内联（减少间接层）
- `handle_run` 内部统一 `processed_files: Vec<(PathBuf, usize)>` via block-expression 赋值
- 顺序路径在 for 循环中收集 `per_file_counts` 并赋给 `processed_files`
- verbose 摘要块输出每文件 `Processed: <path> — N records` 行（位于完成摘要之前）
- 新增 `make_toml_config` 辅助函数减少测试重复代码
- 2 个端到端测试覆盖 verbose 明细输出 / 默认模式无明细 两条路径

## Task Commits

1. **Task 1+2 核心实现** - `8eb536d` (feat) — sqlite_parallel 返回值对齐 + 顺序路径计数收集 + verbose 摘要输出
2. **Task 2 端到端测试** - `7e30906` (test) — integration.rs 新增 2 个 verbose/default 摘要断言测试

## Files Created/Modified

- `src/cli/run/sqlite_parallel.rs` - parallel_collect 返回 (PathBuf, rows) 元组；process_sqlite_parallel 返回 Vec<(PathBuf, usize)>；移除 merge_and_write
- `src/cli/run/mod.rs` - 统一 processed_files via block-expression；顺序路径收集 per_file_counts；verbose 摘要插入 Processed: 明细行
- `tests/integration.rs` - 新增 make_toml_config 辅助函数 + test_cli_verbose_summary_includes_per_file_counts + test_cli_default_summary_omits_per_file_counts

## Decisions Made

- `parallel_collect` 改为返回 `(PathBuf, rows)` 元组对，保留文件与记录的一一对应，避免 skipped 文件导致 zip 索引错位
- `merge_and_write` 函数内联消除，减少函数调用层次和类型转换
- `processed_files` 通过 block-expression 三分支返回，消除 `-D unused-assignments` clippy 错误——Rust 无法静态证明 if/else if/else 覆盖所有路径时的赋值不被覆盖
- Task 1 和 Task 2 的摘要输出代码同次提交：两者因 clippy 不可分割（`processed_files` 在 Task 2 消费前 Task 1 提交会触发 unused variable 错误）

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] parallel_collect 修改为携带 PathBuf 以避免 zip 索引错位**
- **Found during:** Task 1 实现
- **Issue:** 原计划用 `log_files.iter().zip(collected.into_iter())` 对齐 path 与记录，但当文件被中断跳过时 `collected` 长度会小于 `log_files`，导致错误的 path-count 对应。
- **Fix:** 修改 `parallel_collect` 返回 `Vec<(PathBuf, Vec<...>)>` 而非 `Vec<Vec<...>>`，直接携带 path。
- **Files modified:** src/cli/run/sqlite_parallel.rs
- **Verification:** `cargo test` 全部通过

**2. [Rule 3 - Blocking] Task 1 和 Task 2 核心逻辑合并提交**
- **Found during:** Task 1 验证（cargo clippy）
- **Issue:** `processed_files` 变量在 Task 1 完成后未被消费，clippy `-D unused-variables` + `-D unused-assignments` 报错，无法独立通过质量门禁。
- **Fix:** 将 Task 2 的 `if verbose && !processed_files.is_empty()` 摘要输出代码随 Task 1 一起提交。
- **Files modified:** src/cli/run/mod.rs
- **Verification:** `cargo clippy --all-targets -- -D warnings` 退出码 0

---

**Total deviations:** 2 auto-fixed (Rule 3 - blocking)
**Impact on plan:** 两处偏差均为技术实现细节调整，不影响功能语义。最终行为与计划规格完全一致。

## Issues Encountered

无额外问题。pre-commit hook 输出超出限制（同 Plan 01 情况），但实际提交均成功（通过 git log 验证）。

## Next Phase Readiness

- Phase 48 三个 requirement（LOG-01、LOG-02、LOG-03）全部满足
- verbose/quiet 信号路径完全打通，per-file 计数在三条执行路径中统一收集
- Phase 49（glob 模式）可在 processed_files 基础上进一步扩展统计

## Self-Check: PASSED

- SUMMARY.md 存在于 .planning/phases/48-logging/48-02-SUMMARY.md
- Task 1 commit 8eb536d 存在
- Task 2 commit 7e30906 存在
- src/cli/run/mod.rs: `if verbose && !processed_files.is_empty()` 已确认（line 242）
- tests/integration.rs: test_cli_verbose_summary_includes_per_file_counts 已确认（line 1110）
- tests/integration.rs: test_cli_default_summary_omits_per_file_counts 已确认（line 1155）
- cargo clippy --all-targets -- -D warnings: PASSED
- cargo fmt --check: PASSED
- cargo test: 43 passed, 0 failed
- cargo build --release: PASSED

---
*Phase: 48-logging*
*Completed: 2026-06-01*
