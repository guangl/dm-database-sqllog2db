# Phase 37: stdin 管道输入与错误实时输出 - Summary

**Status:** Complete
**Plan:** 直接提交（跳过 GSD 规划流程）
**Commit:** fbd6bb1

## Changes

- Modified `src/cli/run/mod.rs` — auto-detect non-TTY stdin, use /dev/stdin path mapping, skip pre-scan in pipe mode, warn and degrade transaction-level filters to per-record matching when stdin is active

## Verification

| # | Check | Result |
|---|-------|--------|
| 1 | `cat log \| sqllog2db run -c config.toml` 完整执行成功 | PASS |
| 2 | stdin 模式跳过文件发现和 pre-scan | PASS |
| 3 | 事务级过滤降级时输出 stderr 警告 | PASS |
| 4 | 非终端检测（非 TTY = 管道模式） | PASS |
| 5 | cargo clippy --all-targets -- -D warnings | PASS |
| 6 | cargo test 全部通过 | PASS |

## Requirements Satisfied

- PIPE-01: 支持 stdin 管道输入（/dev/stdin 路径映射）
- PIPE-02: stdin 模式自动跳过文件发现和 pre-scan
- UX-04: 非致命错误实时输出到 stderr
