---
phase: 46-errors
plan: 01
subsystem: cli
tags: [rust, error-handling, stderr, thiserror, hint-prefix]

# Dependency graph
requires: []
provides:
  - "format_error_output helper function in src/main.rs"
  - "hint: prefix on all fatal error stderr output (replacing Suggestion:)"
  - "Error::Io suggestion() confirmed non-empty with 'filesystem' text (D-03)"
  - "Unit tests: test_error_io_suggestion_non_empty, test_error_print_format_uses_hint_prefix"
  - "Integration test: test_cli_error_uses_hint_prefix (end-to-end stderr verification)"
affects: [47-config, 48-log, 49-glob]

# Tech tracking
tech-stack:
  added: []
  patterns: ["format_error_output(e: &Error) -> String — extract multi-line error string for testability"]

key-files:
  created: []
  modified:
    - src/main.rs
    - tests/integration.rs

key-decisions:
  - "D-04 scoping honored: no thiserror Display text modified; ERROR-01 covered by existing Display formats"
  - "format_error_output extracted as pure function so hint prefix can be unit-tested without subprocess"
  - "Integration test uses CARGO_BIN_EXE + std::process::Command instead of assert_cmd to avoid new dependency"

patterns-established:
  - "Error print pattern: format_error_output returns '[SEVERITY] msg\\n  hint: suggestion', main() calls eprintln!"
  - "hint: prefix (lowercase, two leading spaces) is the canonical hint format for all fatal errors"

requirements-completed: [ERROR-01, ERROR-02]

# Metrics
duration: 25min
completed: 2026-05-31
---

# Phase 46 Plan 01: 错误信息优化 Summary

**统一致命错误 stderr 输出为 `hint:` 前缀，提取 `format_error_output` 辅助函数，新增 2 项单元测试 + 1 项端到端集成测试覆盖 ERROR-01/ERROR-02**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-05-31T13:51:00Z
- **Completed:** 2026-05-31T14:16:10Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- 将 `src/main.rs` 致命错误分支的 `eprintln!("  Suggestion: {suggestion}")` 替换为通过 `format_error_output(&e)` 输出 `  hint: {suggestion}` 格式
- 提取 `format_error_output(error: &Error) -> String` 纯函数，返回 `[SEVERITY] msg\n  hint: suggestion`，使单元测试可以直接断言输出内容而无需子进程
- 确认 `Error::Io(_)` 的 `suggestion()` 文本与 D-03 一致：`"Check filesystem permissions and disk space."`
- 新增 3 项测试（2 个单元测试 + 1 个集成测试），全套测试（215+ 项）零回归

## Task Commits

1. **Task 1: 替换错误展示前缀为 hint 并补全 Io hint 回归测试** - `132cd36` (feat)
2. **Task 2: 端到端 CLI 验证 hint 输出** - `9429863` (feat)

## Files Created/Modified

- `src/main.rs` — 新增 `format_error_output` 函数；致命错误分支改用该函数；新增 `test_error_io_suggestion_non_empty` 和 `test_error_print_format_uses_hint_prefix` 单元测试
- `tests/integration.rs` — 新增 `test_cli_error_uses_hint_prefix` 集成测试，通过真实 CLI 二进制验证 stderr 包含 `  hint: ` 且不含 `Suggestion:`

## Decisions Made

- **format_error_output 纯函数化（D-01 延伸）：** 将错误格式化逻辑从 main() 分支提取为可测函数，满足 hint 前缀可单元测试断言的要求，同时保持 main() < 40 行规范
- **不引入 assert_cmd 依赖：** 集成测试改用 `std::process::Command` + `CARGO_BIN_EXE_sqllog2db`，避免新增 crate 依赖，与现有集成测试风格一致
- **error.rs 保持不变：** `Error::Io` suggestion 文本已符合 D-03，无需修改；所有其他 thiserror Display 文本依 D-04 决策保持不变

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] clippy doc-markdown 报告缺少反引号**
- **Found during:** Task 2 提交时 pre-commit hook 运行 clippy
- **Issue:** 集成测试注释 `Config::ParseFailed` 未加反引号，clippy::doc_markdown 报 error
- **Fix:** 将注释中 `Config::ParseFailed` 改为 `` `Config::ParseFailed` ``
- **Files modified:** tests/integration.rs
- **Verification:** cargo clippy --all-targets -- -D warnings 退出码 0
- **Committed in:** `9429863`（Task 2 提交中包含）

---

**Total deviations:** 1 auto-fixed (Rule 1 - clippy doc-markdown)
**Impact on plan:** 轻微格式修复，无逻辑影响。

## Issues Encountered

TDD 的 RED 阶段提交被 pre-commit hook 拒绝（因为 `format_error_output` 不存在导致编译失败）。直接合并 RED+GREEN 到单次 feat 提交，符合项目实际约束。

## Next Phase Readiness

- Phase 46 完整实现 ERROR-01 + ERROR-02 需求
- `format_error_output` 函数为后续错误展示改进提供统一入口
- Phase 47（配置文件体验）可依赖本 phase 建立的 hint 格式规范

## Self-Check: PASSED

- `src/main.rs` 存在 `format_error_output` 函数: FOUND
- `grep 'hint: {hint}' src/main.rs` 输出 1 行: FOUND (line 68)
- `grep -c 'Suggestion:' src/main.rs` = 2（均在测试断言字符串中，非打印代码）: CONFIRMED
- `Error::Io(_) => "Check filesystem permissions` in error.rs: FOUND (line 156)
- Commit 132cd36: FOUND
- Commit 9429863: FOUND
- cargo test --quiet: ALL PASSED
- cargo clippy --all-targets -- -D warnings: PASSED
- cargo build --release: PASSED

---
*Phase: 46-errors*
*Completed: 2026-05-31*
