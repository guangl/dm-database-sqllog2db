---
phase: 71-mod-rs-mod-rs-pub-use
reviewed: 2026-06-07T00:00:00Z
depth: standard
files_reviewed: 54
files_reviewed_list:
  - src/cli/run/error_log.rs
  - src/cli/run/input.rs
  - src/cli/run/mod.rs
  - src/cli/run/orchestrator.rs
  - src/cli/run/prescan.rs
  - src/cli/run/sequential.rs
  - src/cli/run/summary.rs
  - src/cli/run/tests.rs
  - src/cli/stats/handler.rs
  - src/cli/stats/mod.rs
  - src/cli/stats/tests.rs
  - src/cli/watch/append.rs
  - src/cli/watch/debounce.rs
  - src/cli/watch/dirs.rs
  - src/cli/watch/event.rs
  - src/cli/watch/handler.rs
  - src/cli/watch/mod.rs
  - src/cli/watch/state.rs
  - src/cli/watch/status.rs
  - src/cli/watch/tests.rs
  - src/cli/watch/trigger_full.rs
  - src/cli/watch/trigger_incremental.rs
  - src/cli/watch/watcher.rs
  - src/config/error_log.rs
  - src/config/mod.rs
  - src/config/root.rs
  - src/config/tests.rs
  - src/exporter/api.rs
  - src/exporter/csv/exporter.rs
  - src/exporter/csv/impls.rs
  - src/exporter/csv/mod.rs
  - src/exporter/csv/writer.rs
  - src/exporter/kind.rs
  - src/exporter/manager.rs
  - src/exporter/mod.rs
  - src/exporter/sqlite/exporter.rs
  - src/exporter/sqlite/impls.rs
  - src/exporter/sqlite/mod.rs
  - src/exporter/sqlite/pragma.rs
  - src/exporter/stats.rs
  - src/exporter/util.rs
  - src/pipeline/field_mask.rs
  - src/pipeline/filters/feature_ops.rs
  - src/pipeline/filters/indicator_ops.rs
  - src/pipeline/filters/mod.rs
  - src/pipeline/filters/sql_ops.rs
  - src/pipeline/filters/tests.rs
  - src/pipeline/mod.rs
  - src/pipeline/normalize_config.rs
  - src/pipeline/output_config.rs
  - src/pipeline/processor.rs
  - src/pipeline/tests.rs
  - src/stats/mod.rs
  - src/stats/runner.rs
  - src/stats/tests.rs
findings:
  critical: 0
  warning: 3
  info: 3
  total: 6
status: issues_found
---

# Phase 71: Code Review Report

**Reviewed:** 2026-06-07
**Depth:** standard
**Files Reviewed:** 54
**Status:** issues_found

## Summary

本次审查覆盖了 `mod.rs` 拆分重构后的全部 54 个文件，重点验证：可见性正确性、重导出完整性、逻辑残留与跨模块导入正确性。

整体结构重构执行正确：`cargo check` 与 `cargo clippy --all-targets -- -D warnings` 均通过，说明无编译期错误或警告。所有模块的公开 API（`pub use`）均正确从 `mod.rs` 向外导出，集成测试（`tests/watch_incremental.rs`、`tests/integration.rs`）使用的 pub API 均在对应 `mod.rs` 中声明。

发现 3 个 WARNING 和 3 个 INFO 级别问题，无 CRITICAL 问题。

## Warnings

### WR-01: prescan.rs 内联测试未迁移到 tests.rs，测试分散在两处

**File:** `src/cli/run/prescan.rs:141-321`
**Issue:** 本次重构目标是将测试统一迁移到独立的 `tests.rs`，但 `prescan.rs` 中仍保留了 9 个内联 `#[test]` 函数（测试私有函数 `build_indicator_filters`、`build_sql_include_filters`、`build_sql_exclude_filters`）。与此同时，`run/tests.rs` 中也有针对 `prescan` 的测试（`scan_log_file_for_matches`、`scan_for_trxids_by_transaction_filters`）。测试分散在两处，与本次重构意图不一致，增加维护认知负担。

根本原因：`build_indicator_filters` 等函数是私有函数（无 `pub`），无法从外部 `tests.rs` 访问，导致迁移受阻。

**Fix:** 将 `build_indicator_filters`、`build_sql_include_filters`、`build_sql_exclude_filters` 的可见性改为 `pub(super)`（仅对父模块 `run` 可见），然后将内联测试迁移至 `src/cli/run/tests.rs`，通过 `super::prescan::build_indicator_filters` 访问：

```rust
// prescan.rs 中改为 pub(super)
pub(super) fn build_indicator_filters(indicators: &IndicatorFilters) -> Vec<Filter> { ... }
pub(super) fn build_sql_include_filters(sf: &SqlFilters) -> Vec<Filter> { ... }
pub(super) fn build_sql_exclude_filters(sf: &SqlFilters) -> Vec<Filter> { ... }
```

---

### WR-02: watch/mod.rs 中 collect_watch_dirs 和 format_elapsed_hms 的 pub use 与实际消费路径不符

**File:** `src/cli/watch/mod.rs:24-32`
**Issue:** `mod.rs` 中对 `collect_watch_dirs` 和 `format_elapsed_hms` 的 `pub use` 均标注了 `#[allow(unused_imports)]`（共 5 个 `#[allow]` 压制，全部单独逐行声明），注释说明这些 API 仅供集成测试使用。

但实际上：
- `collect_watch_dirs` 在集成测试 `tests/watch_incremental.rs` 中**未使用**——该测试只引用了 `handle_watch`、`trigger_full_file`、`trigger_incremental`、`WatchLoopState`。
- `format_elapsed_hms` 同样在集成测试中**未使用**。
- 两者在内部 `watch/tests.rs` 中通过 `super::dirs::collect_watch_dirs`、`super::status::format_elapsed_hms` 直接访问，不需要通过 `mod.rs` 转发。

这意味着这两个 `pub use` 实际上是死 API，但被 `#[allow(unused_imports)]` 掩盖，无法被工具发现。

**Fix:** 移除对应的 `pub use` 行及其 `#[allow(unused_imports)]` 属性。`watch/tests.rs` 已通过 `super::` 路径直接访问，无需修改；若未来集成测试确实需要使用，届时再添加导出：

```rust
// 从 mod.rs 中删除以下内容：
#[allow(unused_imports)]
pub use dirs::collect_watch_dirs;
#[allow(unused_imports)]
pub use status::format_elapsed_hms;
```

---

### WR-03: orchestrator.rs 和 writer.rs 中存在超过 40 行限制的函数，与 CLAUDE.md 规范冲突

**File:** `src/cli/run/orchestrator.rs:19-142`，`src/exporter/csv/writer.rs:22-209`，`src/cli/run/processor.rs:193-255`

**Issue:** CLAUDE.md 明确要求"保持函数在 40 行以内"。以下函数严重超标：

| 函数 | 文件 | 行数 |
|---|---|---|
| `handle_run` | `orchestrator.rs:19-142` | ~124 行 |
| `write_record_preparsed` | `csv/writer.rs:22-209` | ~188 行 |
| `process_log_file` | `processor.rs:193-255` | ~63 行 |

这些函数在重构前就已经超标，本次拆分到独立文件后并未进一步拆分函数体，使得 CLAUDE.md 的规范持续被违反。

**Fix:** 对以上函数按职责进一步提取子函数。`write_record_preparsed` 可以将 `FieldMask::ALL` 快速路径和自定义字段路径各自提取为独立函数（已有 `#[rustfmt::skip]` 和 `#[allow(clippy::too_many_arguments)]` 暗示函数已过复杂）；`handle_run` 可提取并行/顺序路径的路由逻辑为子函数。

---

## Info

### IN-01: config/mod.rs 中 #[allow(unused_imports)] 逐行压制模式可简化

**File:** `src/config/mod.rs:16-25`
**Issue:** 4 个 `pub use` 各自独立使用 `#[allow(unused_imports)]` 压制，而 `pub use` 在 lib crate 的公开 API 中理论上不会触发 `unused_imports` lint（Rust 编译器不对公开 re-export 发出此警告）。这些 `#[allow]` 属性是多余的注解噪声，令阅读者误解这些 API 是否真的被外部消费。

**Fix:** 验证移除 `#[allow(unused_imports)]` 后 `cargo clippy` 是否仍然通过，若通过则删除这些属性。若确实需要，可合并为单一 `#[allow(unused_imports)]` 块覆盖整个区域。

---

### IN-02: watch/mod.rs 中 5 个 #[allow(unused_imports)] 逐行声明，可合并

**File:** `src/cli/watch/mod.rs:23-32`
**Issue:** 5 个独立的 `#[allow(unused_imports)]` 属性逐行包裹 5 个 `pub use`，视觉噪声较大。如果确实需要（例如在某些条件下确实触发 lint），可合并为模块级别的一次声明。

**Fix:** 将多个逐行 `#[allow]` 改为统一的模块级属性（如果合法），或如 WR-02 所述直接移除不必要的导出。

---

### IN-03: stats/mod.rs 中 StatsConfig 和 validate_time_str 的 pub use 带 allow(unused_imports)

**File:** `src/stats/mod.rs:17-20`
**Issue:** 注释说明这两个 re-export 是"for lib API consumers"，但同样被 `#[allow(unused_imports)]` 压制，与 IN-01 的问题相同——这些是 `pub use`，不应受 `unused_imports` lint 影响。`validate_time_str` 的公开程度值得审视：它是内部校验函数，对外公开可能泄漏实现细节，且当前无外部消费者。

**Fix:** 验证 `validate_time_str` 是否确实需要作为 lib 公开 API。若否，改为 `pub(crate)` 并移除 `#[allow(unused_imports)]`。若是，直接移除 `#[allow(unused_imports)]`（lib crate 的 `pub use` 不需要该属性）。

---

_Reviewed: 2026-06-07_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
