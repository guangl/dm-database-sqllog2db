---
phase: 03-doc-align
plan: "01"
subsystem: cli/opts
tags: [docs, cli, after_help, help-examples]
requirements_completed: [DOC-05]

dependency_graph:
  requires: []
  provides: [watch-help-2-examples, validate-help-2-examples]
  affects: [src/cli/opts.rs]

tech_stack:
  added: []
  patterns: [clap after_help multi-example format (4/8 space indent, blank-line separated)]

key_files:
  modified:
    - src/cli/opts.rs

decisions:
  - "Watch after_help extended with quiet-mode example (D-08): sqllog2db watch -c config.toml --quiet"
  - "Validate after_help extended with verbose-mode example (D-09): sqllog2db validate -c config.toml --verbose"
  - "Stats after_help preserved at 3 examples unchanged (D-10)"

metrics:
  duration: "164s (~2m)"
  tasks_completed: 2
  files_modified: 1
  completed_date: "2026-06-07T06:59:01Z"
---

# Phase 03 Plan 01: Add --help Examples for Watch and Validate Summary

Watch 和 Validate 子命令 `after_help` 各追加 1 个示例，DOC-05 requirement（每子命令 ≥2 个 EXAMPLES）完全满足，三道质量门禁（fmt / clippy / test）全绿。

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Watch variant 追加 quiet 模式示例（D-08） | 8695076 | src/cli/opts.rs |
| 2 | Validate variant 追加 verbose 模式示例（D-09）+ 三道质量门禁 | 8a0eb7a | src/cli/opts.rs |

## What Was Built

**src/cli/opts.rs** 中两处 `after_help` 字符串扩展：

1. **Watch variant** — 从 1 个示例扩展为 2 个：
   - 保留原示例：`Watch and process new log files automatically`
   - 新增：`Watch in quiet mode (suitable for cron/background): sqllog2db watch -c config.toml --quiet`

2. **Validate variant** — 从 1 个示例扩展为 2 个：
   - 保留原示例：`Validate a configuration file`
   - 新增：`Validate and show detailed field information: sqllog2db validate -c config.toml --verbose`

格式与 Stats variant（样板，3 个示例）完全对齐：描述行 4 空格缩进 + 冒号、命令行 8 空格缩进、示例之间空行分隔、末尾紧跟 `"`。

## Quality Gates

| Gate | Result |
|------|--------|
| `cargo fmt --check` | PASS — 无 diff |
| `cargo clippy --all-targets -- -D warnings` | PASS — 无 warning |
| `cargo test` | PASS — 909 个测试通过（390 lib + 421 lib 分组 + 3 + 87 + 1 + 7），0 失败 |

## Self-Check

对照 `must_haves.truths` 逐条验证：

- [x] `sqllog2db watch --help` 包含 `Watch in quiet mode (suitable for cron/background):` — VERIFIED
- [x] `sqllog2db watch --help` 包含 `sqllog2db watch -c config.toml --quiet` — VERIFIED
- [x] `sqllog2db validate --help` 包含 `Validate and show detailed field information:` — VERIFIED
- [x] `sqllog2db validate --help` 包含 `sqllog2db validate -c config.toml --verbose` — VERIFIED
- [x] `cargo clippy --all-targets -- -D warnings` 通过，无警告 — VERIFIED
- [x] `cargo fmt --check` 通过，无 diff — VERIFIED
- [x] Stats variant 的 3 个示例保持不变（D-10）— VERIFIED（stats --help 示例描述行 = 3）

## Self-Check: PASSED

所有 `must_haves.truths` 全部满足。

## Deviations from Plan

None — 计划按原文执行，无偏差。

## Known Stubs

None.

## Threat Flags

None — 本计划仅修改 CLI help 字符串，无新增网络端点、认证路径、文件访问或 schema 变更。
