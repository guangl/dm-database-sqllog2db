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
  warning: 3
  info: 3
  total: 6
status: issues_found
---

# Phase 03: Code Review Report

**Reviewed:** 2026-06-07T00:00:00Z
**Depth:** standard
**Files Reviewed:** 2
**Status:** issues_found

## Summary

审查了 `src/cli/opts.rs`（clap CLI 结构定义，含子命令 after_help 示例）和 `README.md`（用户文档，含功能描述、退出码、关键模块列表）。未发现安全漏洞或运行时 bug。所有缺陷均在文档/help 文本层面：退出码描述与代码实现不符、validate `--verbose` 示例具有误导性、关键模块列表遗漏 CSV 并行路径模块、模块路径拼写错误。

---

## Warnings

### WR-01: README 声明的退出码与实现不符

**File:** `README.md:189`

**Issue:** README 写道：

> 退出码：0（成功）、2（配置错误）、3（文件/解析错误）、4（导出错误）、130（用户中断）

`src/main.rs`（第 24–26 行）实际只定义了三个非零退出码：

```rust
const EXIT_PARTIAL: i32 = 1;   // 有非致命错误（parse/export error_stats）
const EXIT_FATAL:   i32 = 2;   // 致命错误（配置/文件/解析/导出均映射到此）
const EXIT_INTERRUPTED: i32 = 130;
```

退出码 `3`（文件/解析错误）和 `4`（导出错误）在代码库中不存在。依赖这两个退出码做自动化判断的脚本将静默失效。此外退出码 `1`（部分成功）在 README 中完全缺失。

**Fix:** 将 README 第 189 行改为：

```
退出码：0（成功）、1（处理完成但有非致命错误）、2（致命错误，包含配置/文件/解析/导出）、130（用户中断）
```

---

### WR-02: `validate` after_help 示例声称 `--verbose` 显示详细字段信息，但该标志对 `validate` 无效

**File:** `src/cli/opts.rs:112-113`

**Issue:** `Validate` 子命令 `after_help` 包含：

```
Validate and show detailed field information:
    sqllog2db validate -c config.toml --verbose
```

`--verbose` 是全局标志。`src/main.rs`（第 130–139 行）在 `validate` 路径调用 `init_simple_logging(cli.quiet)`，该函数签名只接受 `quiet`，**不使用 `verbose`**。代码注释也明确写道："verbose flag is intentionally ignored: non-Run commands (init/validate) only support quiet suppression"。

`--verbose` 被 clap 接受但对 `validate` 输出没有任何影响，此示例误导用户认为它会"显示详细字段信息"。

**Fix:** 删除该示例或替换为准确内容：

```rust
after_help = "\
EXAMPLES:
    Validate a configuration file:
        sqllog2db validate -c config.toml

    Validate in quiet mode (suppress non-error output):
        sqllog2db validate -c config.toml --quiet"
```

---

### WR-03: README 关键模块列表遗漏 CSV 并行路径模块 `parallel.rs`

**File:** `README.md:74`

**Issue:** 关键模块章节只列出了 `cli/run/sqlite_parallel.rs`，没有列出 `cli/run/parallel.rs`。
实际上 `src/cli/run/parallel.rs` 的 `process_csv_parallel` 是 CSV 多文件并行导出的核心模块（`mod.rs:21,78` 中被引用）。功能特性描述（第 23 行）也仅提到 `sqlite_parallel.rs`，暗示只有 SQLite 支持并行，与实际不符。

**Fix:** 在关键模块列表中补充：

```
- **`cli/run/parallel.rs`**：CSV 导出的多文件并行解析路径（基于 rayon），解析错误通过 `log::warn!` 上报。
```

同时在功能特性 CSV 导出器描述中补充"多文件场景支持 rayon 并行解析路径（`parallel.rs`）"，与 SQLite 描述对称。

---

## Info

### IN-01: README 关键模块列表中 `config.rs` 路径错误，实际为 `config/mod.rs`

**File:** `README.md:80`

**Issue:** 关键模块列表写道：

```
- **`config.rs`**：所有配置结构体，支持 serde 反序列化、嵌套子表支持和 `validate_and_compile()` 预验证。
```

实际文件路径是 `src/config/mod.rs`（目录模块），不存在 `src/config.rs`。CLAUDE.md 对应条目已正确写作 `config/mod.rs`。此外 `validate_and_compile()` 函数名在代码库中不存在（实际方法为 `validate()`），也应一并更正。

**Fix:** 将该行改为：

```
- **`config/mod.rs`**：所有配置结构体，支持 serde 反序列化、嵌套子表支持和 `validate()` 校验。
```

---

### IN-02: `run` 子命令 stdin 示例使用 nginx 风格文件名 `access.log`

**File:** `src/cli/opts.rs:53-54`

**Issue:** `run` 子命令 after_help 的 stdin pipe 示例：

```
Pipe log data via stdin:
    cat access.log | sqllog2db run -c config.toml
```

`access.log` 是 nginx/Apache 访问日志的约定文件名，对达梦 SQL 日志工具而言容易造成混淆。

**Fix:** 使用与工具领域一致的文件名：

```
Pipe log data via stdin:
    cat sqllogs/2025-01-15.log | sqllog2db run -c config.toml
```

---

### IN-03: README 功能特性中 CSV 并行描述缺失导致对称性破坏

**File:** `README.md:22-23`

**Issue:** SQLite 导出器描述（第 23 行）明确提到"多文件场景支持 rayon 并行解析路径（`sqlite_parallel.rs`）"，
但 CSV 导出器描述（第 22 行）完全没有提及 `parallel.rs` 提供的相同能力。两者功能对等，文档描述不对称。（此条与 WR-03 互补，WR-03 关注关键模块列表，IN-03 关注功能特性描述。）

**Fix:** 在 CSV 导出器功能描述末尾补充：

```
多文件场景支持 rayon 并行解析路径（`parallel.rs`）。
```

---

_Reviewed: 2026-06-07T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
