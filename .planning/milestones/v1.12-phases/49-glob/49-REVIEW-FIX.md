---
phase: 49-glob
fixed_at: 2026-06-01T00:00:00Z
review_path: .planning/phases/49-glob/49-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 3
skipped: 2
status: partial
---

# Phase 49: Code Review Fix Report

**Fixed at:** 2026-06-01
**Source review:** .planning/phases/49-glob/49-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5
- Fixed: 3
- Skipped: 2

## Fixed Issues

### WR-01: `apply_cli_inputs_to_config` 静默忽略 `Some(vec![])`

**Files modified:** `src/main.rs`
**Commit:** 114fc2c
**Applied fix:** 将函数逻辑从 `if !inputs.is_empty()` 改为先检查空 Vec，添加 `log::warn!("--input provided but empty; using config inputs")` 后 return，再赋值替换。同时更新文档注释，明确说明 `Some(empty vec) keeps the config value and emits a warning`。

---

### WR-02: `scan_glob` 中 glob 错误映射为 `InvalidPath` 且提示不准确

**Files modified:** `src/parser.rs`
**Commit:** c805a9c
**Applied fix:** 将 glob 错误的 `reason` 字段从 `"invalid glob pattern: {e}"` 改为 `"invalid glob pattern: {e}. Check glob syntax (e.g. wildcards must not include unmatched brackets)"`，在错误消息中内嵌 glob 专属提示，让用户明白是 glob 语法问题而非路径格式问题。

---

### WR-03: `ParserError` 未使用 thiserror 且 `is_fatal` 分类疏漏

**Files modified:** `src/error.rs`
**Commit:** 3852771
**Applied fix:**
1. 删除手动 `impl fmt::Display for ParserError` 和 `impl std::error::Error for ParserError`，改用 `#[derive(Error)]` 并为每个变体添加 `#[error(...)]` 属性。`InvalidPath` 的可选 `line_number` 字段通过 `line_number.map_or_else(String::new, |n| format!(" (line {n})"))` 内联表达式处理，thiserror 2.x 支持此语法。
2. 将 `is_fatal` 中的 `Error::Parser(_) => false` 改为 `Error::Parser(e) => matches!(e, ParserError::ReadDirFailed { .. })`，使目录不可读错误被正确标记为致命。

## Skipped Issues

### IN-01: `_verbose` 参数在 `init_simple_logging` 和 `apply_verbosity_to_config` 中被忽略

**File:** `src/main.rs:27, 41`
**Reason:** 已在 Phase 46 修复。当前代码 `init_simple_logging` 签名为 `fn init_simple_logging(quiet: bool)`（无 `_verbose` 参数），`apply_verbosity_to_config` 已正确使用 `verbose` 和 `quiet` 参数。代码状态与 review 描述不符，无需再修复。

---

### IN-02: 集成测试 `test_e2e_field_projection` 用 `split(',')` 验证 CSV 字段数

**File:** `tests/integration.rs:737-744`
**Reason:** 已在 Phase 48 修复。当前代码注释说明"数据行只验证行数（不用 split(',').count()，SQL 含逗号时会误判）"，已改用行数验证而非字段数统计。代码状态与 review 描述不符，无需再修复。

---

_Fixed: 2026-06-01_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
