---
phase: 03-doc-align
reviewed: 2026-06-07T06:00:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - src/cli/opts.rs
  - README.md
findings:
  critical: 0
  warning: 0
  info: 1
  total: 1
status: issues_found
---

# Phase 03: Code Review Report (Re-review)

**Reviewed:** 2026-06-07T06:00:00Z
**Depth:** standard
**Files Reviewed:** 2
**Status:** issues_found

## Summary

对 `src/cli/opts.rs` 和 `README.md` 进行重新审查，验证前次报告中 6 个问题的修复情况，并以对抗性视角扫描是否引入新缺陷。

**修复验证结果：** 全部 6 个旧问题均已正确修复：

- **WR-01** — README 退出码更新为 `0/1/2/130`，与 `main.rs` 中的 `EXIT_PARTIAL=1`、`EXIT_FATAL=2`、`EXIT_INTERRUPTED=130` 常量完全一致。
- **WR-02** — `validate` after_help 的 `--verbose` 示例已替换为 `--quiet` 示例，与 `main.rs` 中 `validate` 路径只调用 `init_simple_logging(cli.quiet)` 的行为一致。
- **WR-03** — README 关键模块列表第 74 行已补充 `cli/run/parallel.rs` 条目。
- **IN-01** — README 关键模块列表中 `config.rs` 已更正为 `config/mod.rs`，`validate_and_compile()` 已更正为 `validate()`。
- **IN-02** — opts.rs stdin 示例文件名已从 `access.log` 更正为 `sqllogs/2025-01-15.log`。
- **IN-03** — README 第 22 行 CSV 导出器描述末尾已补充 `parallel.rs` 并行路径说明。

本次发现 1 个新的 Info 级别问题。

---

## Info

### IN-01: README 关键模块列表中新增的 `parallel.rs` 条目描述与 `sqlite_parallel.rs` 条目不对称

**File:** `README.md:74`

**Issue:** 修复 WR-03 时新增的 `parallel.rs` 条目（第 74 行）描述为：

```
- **`cli/run/parallel.rs`**：CSV 导出的多文件并行解析路径（基于 rayon），解析错误通过 `log::warn!` 上报。
```

与下方的 `sqlite_parallel.rs` 条目（第 75 行）描述完全对称，格式和内容一致。

然而，功能特性区域（第 22 行）的 CSV 导出器描述修复后为：

```
多文件场景支持 rayon 并行解析路径（`parallel.rs`）。
```

而第 23 行 SQLite 导出器描述为：

```
多文件场景支持 rayon 并行解析路径（`sqlite_parallel.rs`）。
```

两行格式和内容完全一致，对称已正确实现。

实际上两处修复均正确，无功能问题。此 IN 条目记录唯一一个微小的信息完整性观察：`CLAUDE.md`（项目内部参考文档，非本次审查范围）第 43-50 行的 Key Modules 列表仍未包含 `cli/run/parallel.rs`，与 README 修复后的状态不同步。该文件不在本次审查范围内，仅作记录，不要求在本次 PR 中修复。

**Fix:** 可选地在 `CLAUDE.md` 的 Key Modules 部分补充：

```
- **`cli/run/parallel.rs`** — multi-file parallel parse path for CSV export (rayon); parse errors logged via `log::warn!`
```

---

_Reviewed: 2026-06-07T06:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
