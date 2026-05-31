---
phase: 49-glob
reviewed: 2026-06-01T00:00:00Z
depth: quick
files_reviewed: 7
files_reviewed_list:
  - src/config/sqllog.rs
  - src/error.rs
  - src/parser.rs
  - src/cli/opts.rs
  - src/main.rs
  - src/cli/init.rs
  - tests/integration.rs
findings:
  critical: 0
  warning: 3
  info: 2
  total: 5
status: issues_found
---

# Phase 49: Code Review Report

**Reviewed:** 2026-06-01
**Depth:** quick
**Files Reviewed:** 7
**Status:** issues_found

## Summary

本次审查覆盖 glob 输入支持的核心文件：配置结构、错误类型、解析器、CLI 选项、主入口、init 命令和集成测试。
无硬编码密钥、危险函数调用或 debug artifact。
发现 3 个 WARNING 级别问题（两个逻辑边界缺陷、一个错误分类不一致），以及 2 个 INFO 级别问题。

## Warnings

### WR-01: `apply_cli_inputs_to_config` 静默忽略 `Some(vec![])` —— 行为难以调试

**File:** `src/main.rs:49-55`
**Issue:** 当用户传入 `--input` 且随后 clap 解析产生空 Vec 时（理论上不会发生，但代码层面可能被调用方传入），函数静默保留 config 值，不报告任何警告。更重要的是该函数文档注释写的是"CLI inputs completely replace config inputs when Some"，但 `Some(vec![])` 时不替换，与文档语义矛盾，属于隐性行为。若未来有人向此函数传入 `Some(vec![])` 以"清空"输入，将得到完全相反的效果（保留旧值），导致静默数据错误。

```rust
fn apply_cli_inputs_to_config(cfg: &mut Config, cli_inputs: Option<Vec<String>>) {
    if let Some(inputs) = cli_inputs {
        // 改为：始终替换，并在空时返回错误或 warn
        if inputs.is_empty() {
            // 选项 A: 报 warning 然后保留 config
            log::warn!("--input provided but empty; using config inputs");
            return;
        }
        cfg.sqllog.inputs = inputs;
    }
}
```

若保留静默行为，应至少修正注释，明确说明 "Some(empty vec) keeps config value"。

---

### WR-02: `scan_glob` 中 glob 错误被映射为 `InvalidPath` 而非专用错误变体，suggestion 内容不准确

**File:** `src/parser.rs:108-114`
**Issue:** glob 模式解析失败（`glob::glob` 返回 `PatternError`）时，错误被包装为 `ParserError::InvalidPath { path, reason: "invalid glob pattern: ..." }`。`suggestion()` 对应的提示是 `"Check the path format or try an absolute path."` — 这对 glob 错误没有意义，用户应收到的提示应是"检查 glob 语法"而非"尝试绝对路径"。

```rust
// 当前：InvalidPath 的 suggestion 为 "Check the path format or try an absolute path."
// 修复选项 A（最小改动）：在 reason 字段内嵌入 glob 提示
reason: format!("invalid glob pattern: {e}. Check glob syntax (e.g. wildcards must not include unmatched brackets)"),

// 修复选项 B（彻底）：在 ParserError 中添加 InvalidGlobPattern 变体，
// 并在 suggestion() 中为其返回专属提示。
```

---

### WR-03: `ParserError` 未实现 `thiserror::Error`，与其他错误类型不一致，且 `is_fatal` 分类存在疏漏

**File:** `src/error.rs:203-248`
**Issue:**
1. 其余所有错误枚举（`ConfigError`、`FileError`、`ExportError`）均使用 `#[derive(Error)]`，而 `ParserError` 手工实现 `Display` 并单独实现 `std::error::Error`。这是唯一的例外，在未来添加新变体时很容易遗漏 `Display` 分支。

2. `is_fatal` 对 `Error::Parser(_)` 整体返回 `false`——但 `ParserError::ReadDirFailed` 语义上相当于目录无法访问，属于无法继续的操作，与 `PathNotFound` 同属"运行时无法找到文件"。当前所有 `Parser` 错误都被标记为非致命并仅写入错误日志，但 `ReadDirFailed` 可能意味着整个输入目录不可读，导致 0 条记录被处理而程序仍以"成功"退出。

```rust
// 修复 1：迁移到 thiserror
#[derive(Debug, Error)]
pub enum ParserError {
    #[error("Path not found: {path}")]
    PathNotFound { path: PathBuf },
    // ...
}

// 修复 2（视业务需求决定）：
pub fn is_fatal(&self) -> bool {
    match self {
        // ...
        Error::Parser(e) => matches!(e, ParserError::ReadDirFailed { .. }),
    }
}
```

## Info

### IN-01: `_verbose` 参数在 `init_simple_logging` 和 `apply_verbosity_to_config` 中被忽略

**File:** `src/main.rs:27, 41`
**Issue:** 两个函数都声明了 `_verbose: bool` 参数（加下划线表示"已知未用"），但 `--verbose` 对非 Run 子命令的日志级别没有任何效果。用户在 `init` 或 `validate` 时使用 `-v` 会被静默忽略，而 CLI help 显示 `-v` 是全局标志（`global = true`），用户可能期望它影响所有子命令。
建议：要么在 `init_simple_logging` 中使用 `verbose` 参数（如设置 `Debug` 级别），要么在 help 文本中说明 `-v` 仅对 `run` 有效。

---

### IN-02: 集成测试 `test_e2e_field_projection` 用 `split(',')` 验证 CSV 字段数

**File:** `tests/integration.rs:737-744`
**Issue:** 注释已自承局限性："如果 SQL 中包含逗号，需改用 csv crate 正确解析"。当前测试构造的 SQL 恰好不含逗号，但这是一个脆弱的假设——测试辅助函数 `write_test_log` 生成的 SQL 格式若在未来变更（加入 `IN (1, 2)` 之类子句），该断言将误报失败而不是捕获真正的 bug。建议使用 `csv` crate 解析 CSV 行再统计字段数。

---

_Reviewed: 2026-06-01_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: quick_
