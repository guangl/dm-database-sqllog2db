---
phase: 71-mod-rs-mod-rs-pub-use
plan: "05"
subsystem: config
tags: [refactor, module-split, config]
dependency_graph:
  requires: []
  provides: [config/error_log.rs, config/root.rs, config/tests.rs]
  affects: [src/config/mod.rs]
tech_stack:
  added: []
  patterns: [mod-declarations-only, pub-use-reexport]
key_files:
  created:
    - src/config/error_log.rs
    - src/config/root.rs
    - src/config/tests.rs
  modified:
    - src/config/mod.rs
decisions:
  - "#[allow(unused_imports)] 用于 pub use 声明：binary crate 中测试代码消费的 pub use 在非测试编译上下文触发 unused_imports，与原始代码行为一致，使用 allow 属性明确记录意图"
metrics:
  duration: "6 minutes"
  completed: "2026-06-07T12:04:00Z"
---

# Phase 71 Plan 05: Config mod.rs 拆分为 root + error_log + tests Summary

**One-liner:** 将 config/mod.rs（194 行）拆为 error_log.rs（ErrorLogConfig）+ root.rs（Config struct + from_file）+ tests.rs（13 个测试），mod.rs 缩减至 25 行仅含 mod 声明与 pub use。

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | 拆分 config/mod.rs 到 root.rs + error_log.rs + tests.rs | 0bcc15d | src/config/{mod.rs,error_log.rs,root.rs,tests.rs} |

## Decisions Made

1. **`#[allow(unused_imports)]` 处理 binary crate pub use 警告**：原始 mod.rs 中 `pub use` 因 `Config` struct 内联使用而不报 unused；拆分后 mod.rs 无内联引用，Rust 对非测试编译上下文中仅被测试消费的 `pub use` 报 `unused_imports`。使用 `#[allow(unused_imports)]` 明确记录"这是公开 API 重导出"的意图，与原始代码语义等价。

2. **tests.rs 去掉 `use super::*`**：原测试内联在 mod.rs 时用 `use super::*` + 具名导入共存，独立文件后 `super::*` 与具名导入重叠导致 `unused import: super::*`，改为仅保留具名导入路径更清晰。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `unused_imports` clippy 警告**

- **Found during:** Task 1 Step 5
- **Issue:** 拆分后 mod.rs 中 `pub use StatsConfig`、`pub use ErrorLogConfig`、`pub use ExporterConfig`、`pub use SqllogConfig` 在非测试编译上下文中触发 `unused_imports -D warnings` 失败。原因：这些类型在非测试代码中的消费者（`preflight.rs`、`run/tests.rs`）均在 `#[cfg(test)]` 块内，非测试编译时无引用。
- **Fix:** 对纯测试消费的 4 个 `pub use` 声明添加 `#[allow(unused_imports)]`，明确标注为 API 重导出意图。
- **Files modified:** `src/config/mod.rs`
- **Commit:** 0bcc15d

**2. [Rule 1 - Bug] `use super::*` 在 tests.rs 中报 unused**

- **Found during:** Task 1 Step 5
- **Issue:** tests.rs 初始版本有 `use super::*` + `use crate::config::{...}` 双重导入，`super::*` 被认为 unused（因具名导入已覆盖所有需要的类型）。
- **Fix:** 移除 `use super::*`，保留具名 `use crate::config::{...}` 导入。
- **Files modified:** `src/config/tests.rs`
- **Commit:** 0bcc15d

## Self-Check

### Files exist:
- `src/config/error_log.rs` — FOUND
- `src/config/root.rs` — FOUND
- `src/config/tests.rs` — FOUND
- `src/config/mod.rs` — FOUND (modified)

### Commits exist:
- `0bcc15d` — FOUND

### Verification:
- `src/config/mod.rs` 行数：25 行（含注释），无 fn/struct/impl
- `cargo test`：全部通过（395 lib + integration tests）
- `cargo clippy --all-targets -- -D warnings`：0 错误
- `cargo run -- init -o /tmp/_71_test_config.toml --force`：exit 0
- `cargo run -- validate -c /tmp/_71_test_config.toml`：exit 0

## Self-Check: PASSED
