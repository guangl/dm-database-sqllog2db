---
phase: 28-remove-charts-update-completions
plan: 02
type: execute
subsystem: cli
tags: [remove, self-update, dependency-cleanup]
requires: []
provides: [RM-02]
affects: [src/cli/update.rs, src/cli/mod.rs, src/cli/opts.rs, src/main.rs, src/error.rs, src/lang.rs, Cargo.toml]
tech-stack:
  added: []
  removed: [self_update 0.44.0, reqwest, rustls, compression-flate2]
  patterns: []
key-files:
  created: []
  modified:
    - src/cli/mod.rs (remove pub mod update)
    - src/cli/opts.rs (remove SelfUpdate variant)
    - src/main.rs (remove update refs in production and test code)
    - src/error.rs (remove Update variant and UpdateError enum)
    - src/lang.rs (remove self-update Chinese localization)
    - Cargo.toml (remove self_update dependency block)
  deleted:
    - src/cli/update.rs (97 lines, entire module)
decisions:
  - "测试代码的 UpdateError 引用在 Task 1 中一并删除（非 Task 2），因为 UpdateError 类型已不存在导致编译失败 — 这是任务间依赖导致的阻塞修复（Rule 3）"
  - "lang.rs 中 self-update 的 zh 本地化在 Task 1 中一并删除，因为 SelfUpdate 子命令已被删除，clap 会报 Command undefined panic"
metrics:
  duration: ~15 min
  completed: "2026-05-20"
  tasks: 2
  commits: 2
---

# Phase 28 Plan 02: 移除 self-update 自更新功能 (RM-02)

删除 src/cli/update.rs 文件及其所有引用，移除 SelfUpdate CLI 子命令，删除 UpdateError 错误类型，移除 self_update 依赖。精简后 sqllog2db --help 不再显示 self-update 子命令。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] 测试代码中的 UpdateError 引用和 lang.rs 本地化需同步删除**
- **Found during:** Task 1 commit (pre-commit hook `cargo test --all-targets`)
- **Issue:** 删除 UpdateError 类型后，测试代码仍引用它导致编译失败；删除 SelfUpdate 子命令后，lang.rs 中的 `.mut_subcommand("self-update", ...)` 导致 `clap::Command` panic
- **Fix:** 将测试模块的 `UpdateError` import 和 `test_exit_code_update_error` 函数一并删除；将 lang.rs 中 self-update 的中文本地化代码删除
- **Files modified:** src/main.rs (test module), src/lang.rs
- **Commit:** c1dae04

## Known Stubs

None.

## Threat Flags

None.

## Verification Results

| Check | Status |
|-------|--------|
| `src/cli/update.rs` deleted | PASS |
| `pub mod update;` removed from cli/mod.rs | PASS |
| SelfUpdate removed from opts.rs Commands | PASS |
| No SelfUpdate/check_for_updates/UpdateError in main.rs | PASS |
| No UpdateError/Error::Update in error.rs | PASS |
| No self_update in Cargo.toml | PASS |
| `--help` does not contain self-update | PASS |
| `cargo build` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo test` (832 tests) | PASS |
| `cargo fmt --check` | PASS |

## Self-Check

正在验证所有声明...

所有文件、提交和验证检查均已确认通过。
## Self-Check: PASSED
