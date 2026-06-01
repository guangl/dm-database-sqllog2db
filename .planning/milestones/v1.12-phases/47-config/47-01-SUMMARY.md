---
phase: 47
plan: 01
subsystem: cli/validate
tags: [validate, user-output, error-format, config]
dependency_graph:
  requires: [46-01]
  provides: [validate-structured-output]
  affects: [src/cli/validate.rs, src/main.rs, tests/integration.rs]
tech_stack:
  added: []
  patterns: [println! for user-facing output, eprintln! for error output, exit code 2 for fatal]
key_files:
  created: []
  modified:
    - src/cli/validate.rs
    - src/main.rs
    - tests/integration.rs
decisions:
  - validate通过时只输出"Configuration valid."（静默通过，不输出[OK]列表）
  - validate失败时输出"[FAIL] reason\n  hint: ..."而非"[CRITICAL]"（validate专属格式）
  - handle_validate不再走日志路由，直接println!到stdout
metrics:
  duration: ~10min
  completed: "2026-05-31"
  tasks_completed: 1
  files_modified: 3
---

# Phase 47 Plan 01: validate 输出结构化重构 Summary

**One-liner:** 将 validate 命令从 log::info! 路由改为直接 println!/eprintln!，通过时静默输出 `Configuration valid.`，失败时渲染 `[FAIL] reason\n  hint: ...`。

## Tasks Completed

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 1 | validate输出结构化 + [FAIL]格式 + 集成测试 | 9fea196 | src/cli/validate.rs, src/main.rs, tests/integration.rs |

## What Was Built

### handle_validate 简化（src/cli/validate.rs）

原先 `handle_validate` 使用 `log::info!` 输出十几行日志，包含 sqllog 路径、日志级别、过滤器状态等详情，需要日志系统初始化且含时间戳前缀，不适合用户阅读。

新实现：完全移除所有 `log::info!` 调用，改为单行 `println!("Configuration valid.")`。函数只在 `cfg.validate()` 成功后被调用，所以只输出成功状态。

### main.rs Validate 分支失败处理

原先：`cfg.validate()?` 将错误传播到外层 `Err(e)` 分支，使用 `format_error_output` 渲染为 `[CRITICAL] Configuration error: ...`。

新实现：Validate 分支直接捕获 `cfg.validate()` 的错误，渲染为：
```
[FAIL] <error_message>
  hint: <suggestion>
```
然后 `std::process::exit(EXIT_FATAL)`，不再走通用的 `format_error_output` 路径。

### 集成测试（tests/integration.rs）

新增两个 CLI 进程测试（CONFIG-02）：

1. `test_cli_validate_valid_config_outputs_configuration_valid`：对有效配置验证退出码 0，stdout 含 `Configuration valid.`
2. `test_cli_validate_invalid_config_outputs_fail_prefix`：对无效配置（logging.level = "verbose"）验证退出码 2，stderr 含 `[FAIL]` 和 `  hint: `，不含 `[CRITICAL]` 或 `[ERROR]`

## Verification

- `cargo build`: 通过
- `cargo clippy --all-targets -- -D warnings`: 通过，无警告
- `cargo test`: 36 个测试全部通过（新增 2 个 CLI e2e 测试）
- `cargo fmt --check`: 通过

## Deviations from Plan

None - 计划完全按预期执行。

CONTEXT.md D-01 选择了直接 println!/eprintln! 方案（不走日志路由），已按此实现。D-03 静默通过策略已实现（不输出 [OK] 列表）。

## Known Stubs

None.

## Self-Check: PASSED

- [x] src/cli/validate.rs 已修改（handle_validate 简化为 println!）
- [x] src/main.rs 已修改（Validate 分支错误处理改为 [FAIL] 格式）
- [x] tests/integration.rs 已修改（新增两个 CLI e2e 测试）
- [x] 提交 9fea196 存在于 git log
