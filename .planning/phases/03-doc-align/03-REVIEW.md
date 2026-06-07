---
phase: 03-doc-align
reviewed: 2026-06-07T00:00:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - src/cli/opts.rs
  - README.md
findings:
  critical: 0
  warning: 2
  info: 1
  total: 3
status: issues_found
---

# Phase 03: Code Review Report

**Reviewed:** 2026-06-07T00:00:00Z
**Depth:** standard
**Files Reviewed:** 2
**Status:** issues_found

## Summary

Two files were reviewed: `src/cli/opts.rs` (added `after_help` examples to `watch` and `validate` subcommands) and `README.md` (updated with `watch`, `init --interactive`, and `--quiet`/`--verbose` content). The source changes are small and mostly accurate. Two documentation claims that contradict the running implementation were found, either of which would mislead a user who relied on the documented behavior.

---

## Warnings

### WR-01: README documents non-existent exit codes 3 and 4

**File:** `README.md:189`
**Issue:** Line 189 states:
> 退出码：0（成功）、2（配置错误）、3（文件/解析错误）、4（导出错误）、130（用户中断）

The actual exit code constants in `src/main.rs` (lines 24–26) are:
```
EXIT_PARTIAL      = 1   (partial success — parse errors present)
EXIT_FATAL        = 2   (fatal error of any kind)
EXIT_INTERRUPTED  = 130 (Ctrl+C)
```
Exit codes 3 and 4 do not exist anywhere in the codebase. All fatal errors — whether config, file/parse, or export — resolve to `EXIT_FATAL` (2). Exit code 1 (partial success) is not mentioned in the README at all.

**Fix:** Replace line 189 with the accurate table:
```
退出码：0（成功）、1（处理完成但有非致命解析/导出错误）、2（致命错误，包含配置错误、文件错误、导出错误）、130（用户中断）。
```

---

### WR-02: `validate --verbose` example in opts.rs implies verbose has an effect on validate output, but it does not

**File:** `src/cli/opts.rs:112-113`
**Issue:** The `Validate` subcommand's `after_help` block includes:
```
    Validate and show detailed field information:
        sqllog2db validate -c config.toml --verbose
```
`--verbose` is a global flag that is explicitly stated in `src/main.rs` (lines 29–31) to be intentionally ignored for non-`Run`/`Stats`/`Watch` commands:
> "verbose flag is intentionally ignored: non-Run commands (init/validate) only support quiet suppression"

`handle_validate` (`src/cli/validate.rs`) takes only `&Config` and calls no verbose-sensitive code path. Passing `--verbose` to `validate` silently has zero effect, but the help text implies it will "show detailed field information."

**Fix:** Remove the misleading example, or replace it with a factually accurate one (e.g., quiet mode):
```rust
after_help = "\
EXAMPLES:
    Validate a configuration file:
        sqllog2db validate -c config.toml

    Validate in quiet mode (suppress non-error output):
        sqllog2db validate -c config.toml --quiet"
```

---

## Info

### IN-01: `run` subcommand example references `access.log`, not a DM SQL log

**File:** `src/cli/opts.rs:53-54`
**Issue:** The `run` subcommand's `after_help` pipe example reads:
```
    Pipe log data via stdin:
        cat access.log | sqllog2db run -c config.toml
```
`access.log` is a conventional name for HTTP/nginx access logs, not DaMeng SQL logs. This is cosmetically misleading — the tool only processes DM SQL log format. The functionality itself (stdin pipe) is real and correctly implemented.

**Fix:** Use a filename consistent with the tool's domain:
```
    cat sqllogs/2025-01-15.log | sqllog2db run -c config.toml
```

---

_Reviewed: 2026-06-07T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
