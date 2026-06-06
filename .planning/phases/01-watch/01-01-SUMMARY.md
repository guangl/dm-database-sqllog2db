---
phase: 01-watch
plan: 01
subsystem: cli/watch + config + cli/run
tags: [watch, csv-append, error-log, exit-code, WATCH-07, WATCH-08, WATCH-09]
dependency_graph:
  requires: []
  provides: [WATCH-07, WATCH-08, WATCH-09]
  affects: [src/config/mod.rs, src/cli/run/mod.rs, src/cli/watch/mod.rs]
tech_stack:
  added: []
  patterns: [Config internal flag, OpenOptions dual-branch, force_append helper]
key_files:
  created: []
  modified:
    - src/config/mod.rs
    - src/cli/run/mod.rs
    - src/cli/watch/mod.rs
    - src/cli/run/tests.rs
    - tests/integration.rs
decisions:
  - "pub append_error_log（非 pub(crate)）以允许集成测试使用结构体字面量初始化语法"
  - "force_append_for_watch_trigger 辅助函数抽取消除 trigger_full_file + build_incremental_cfg 重复"
  - "现有集成测试 test_watch_exits_when_interrupted / test_watch_ignores_non_log_files 断言更新，对齐 WATCH-09 新语义"
metrics:
  duration_minutes: 35
  completed_date: "2026-06-06"
  tasks_completed: 3
  tasks_total: 3
  files_modified: 5
---

# Phase 01 Plan 01: watch 功能完善（WATCH-07/08/09）Summary

## One-liner

为 watch 子命令补齐三项功能短板：CSV 追加（WATCH-07）、error log 历史保留（WATCH-08）、Ctrl+C 退出码 130（WATCH-09）。

## What Was Built

### Task 1 — Config.append_error_log + write_error_log 双分支

- `src/config/mod.rs`：`Config` 结构体新增 `pub append_error_log: bool` 字段，带 `#[serde(skip)]`（不参与 TOML 反序列化），Rust `bool` 默认值 `false` 保持 run 路径覆盖写语义不变
- `src/cli/run/mod.rs`：`write_error_log` 文件打开段由 `File::create`（总截断）改为 `OpenOptions` if/else 双分支——`append_error_log=true` 时追加，`false` 时截断

### Task 2 — watch 触发函数注入 + handle_watch 退出码

- `src/cli/watch/mod.rs`：
  - 新增 `force_append_for_watch_trigger(&mut Config)` 私有辅助函数，统一注入 CSV `append=true` + `overwrite=false` + `append_error_log=true`
  - `trigger_full_file` 在 `inputs` 赋值后调用 `force_append_for_watch_trigger`
  - `build_incremental_cfg` 在 SQLite append 注入后调用 `force_append_for_watch_trigger`（SQLite 逻辑保持原位）
  - `handle_watch` 在 `print_final_summary` 之后、`Ok(())` 之前添加 `interrupted.load(Acquire)` 检查，中断时返回 `Err(Error::Interrupted)`
- `tests/integration.rs`：更新 `test_watch_exits_when_interrupted` + `test_watch_ignores_non_log_files` 断言，对齐 WATCH-09 新语义（期望 `Err(Interrupted)` 而非 `Ok(())`）

### Task 3 — 4 个 Wave 0 测试

- `src/cli/watch/mod.rs::tests`：
  - `test_watch_csv_append`：两次 `trigger_full_file` 后验证 CSV 行数累计、header 仅一行
  - `test_watch_error_log_append`：两次带解析错误的触发后验证 error log 含两条 `[ERROR]` 行
  - `test_handle_watch_returns_interrupted`：`interrupted=true` 时验证 `handle_watch` 返回 `Err(Error::Interrupted)`
- `src/cli/run/tests.rs`：
  - `test_write_error_log_run_still_truncates`：`append_error_log=false` 时验证旧内容被截断（run 路径防回归）

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] pub(crate) 可见性导致集成测试编译失败**

- **Found during:** Task 1 commit
- **Issue:** 计划要求 `pub(crate) append_error_log`，但集成测试 `tests/integration.rs` 和 `tests/watch_incremental.rs` 使用结构体字面量语法 `Config { ..Default::default() }`，当 `Config` 含私有字段时外部 crate 无法使用此语法
- **Fix:** 改为 `pub append_error_log: bool`，保持 `#[serde(skip)]` 避免 TOML 污染
- **Files modified:** `src/config/mod.rs`
- **Commit:** e85ced7

**2. [Rule 1 - Bug] 现有集成测试期望与 WATCH-09 新行为冲突**

- **Found during:** Task 2 commit
- **Issue:** `test_watch_exits_when_interrupted` 断言 `result.is_ok()`，`test_watch_ignores_non_log_files` 使用 `.unwrap()`，均期望 `handle_watch` 返回 `Ok(())` — 与新的 `Err(Interrupted)` 语义冲突
- **Fix:** 更新两个测试断言为 `matches!(result, Err(Error::Interrupted))`，并在 `test_watch_ignores_non_log_files` 中将 `.unwrap()` 改为 `let result = ... ; assert!(matches!(...))`
- **Files modified:** `tests/integration.rs`
- **Commit:** d933021

**3. [Rule 2 - Quality] doc comment identifier 缺少反引号**

- **Found during:** Task 1 pre-commit clippy hook
- **Issue:** doc 注释中 `cfg.append_error_log=true` 和 `write_error_log` 未加反引号，触发 `clippy::doc-markdown` 警告（项目启用 `-D warnings`）
- **Fix:** 添加反引号使其成为 inline code
- **Files modified:** `src/config/mod.rs`, `src/cli/run/mod.rs`
- **Commit:** e85ced7

## Verification Results

- `cargo build --release`: 通过（0 错误）
- `cargo clippy --all-targets -- -D warnings`: 通过（0 警告）
- `cargo fmt --check`: 通过
- `cargo test`: 全套通过
  - 新测试 4 个全部绿色
  - 现有 ~884 测试无回归（含 2 个 ignore）

## Success Criteria Status

- WATCH-07: test_watch_csv_append 绿，CSV 追加行为已实现
- WATCH-08: test_watch_error_log_append 绿（追加）+ test_write_error_log_run_still_truncates 绿（run 覆盖写不变）
- WATCH-09: test_handle_watch_returns_interrupted 绿，main.rs 现有 exit(130) 路径被触发

## Known Stubs

None — 所有功能已完整实现，无 placeholder 或 TODO 标记。

## Threat Flags

None — 修改点均在计划 `<threat_model>` 范围内：
- T-01-01（OpenOptions 路径 Tampering）：accept，路径已在 Phase 39/47 validate 检查
- T-01-02（error log 无限增长 DoS）：accept，Phase 3 DOC-04 计划用户文档告知 logrotate 配置
- T-01-03（追加模式信息披露）：accept，与单次写相同的敏感数据面

## Self-Check: PASSED

- src/config/mod.rs 已修改：pub append_error_log: bool 字段存在
- src/cli/run/mod.rs 已修改：OpenOptions 双分支存在，File::create 已移除
- src/cli/watch/mod.rs 已修改：force_append_for_watch_trigger 函数 + handle_watch 退出码检查 + 4 个新测试（含 3 个 wave-0）
- src/cli/run/tests.rs 已修改：test_write_error_log_run_still_truncates 存在
- tests/integration.rs 已修改：两个受影响的测试已更新断言
- Commits: e85ced7, d933021, f5f8f18（均已验证存在）
