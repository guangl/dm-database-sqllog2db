---
phase: 71-mod-rs-mod-rs-pub-use
fixed_at: 2026-06-07T00:00:00Z
review_path: .planning/phases/71-mod-rs-mod-rs-pub-use/71-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 4
skipped: 2
status: partial
---

# Phase 71: Code Review Fix Report

**Fixed at:** 2026-06-07
**Source review:** .planning/phases/71-mod-rs-mod-rs-pub-use/71-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 6
- Fixed: 4
- Skipped: 2

## Fixed Issues

### WR-01: prescan.rs 内联测试未迁移到 tests.rs

**Files modified:** `src/cli/run/prescan.rs`, `src/cli/run/tests.rs`
**Commit:** `0dbd9dc`
**Applied fix:** 将 `build_indicator_filters`、`build_sql_include_filters`、`build_sql_exclude_filters` 的可见性改为 `pub(super)`，将 prescan.rs 中的 9 个内联测试全部迁移到 `run/tests.rs`，通过 `super::prescan::` 路径访问。

### WR-02: watch/mod.rs dead pub use entries

**Files modified:** `src/cli/watch/mod.rs`
**Commit:** `d1cac21`
**Applied fix:** 移除了 `pub use dirs::collect_watch_dirs` 和 `pub use status::format_elapsed_hms` 两个死导出及其对应的 `#[allow(unused_imports)]` 属性。经验证，这两个符号在集成测试和任何外部代码中均未通过 `mod.rs` 路径使用。

### WR-03: orchestrator.rs handle_run 函数超过 40 行

**Files modified:** `src/cli/run/orchestrator.rs`
**Commit:** `f099445`
**Applied fix:** 从 `handle_run`（原 124 行）中提取了 `build_run_context`、`run_csv_parallel`、`run_sqlite_parallel`、`route_processing`、`finalize_run` 五个子函数，所有函数均控制在 40 行以内。

### WR-03: csv/writer.rs write_record_preparsed 函数超过 40 行

**Files modified:** `src/exporter/csv/writer.rs`
**Commit:** `728866b`
**Applied fix:** 从 `write_record_preparsed`（原 188 行）中提取了 `write_all_fields`（`FieldMask::ALL` 快速路径）和 `write_selected_fields`（自定义字段路径）两个子函数，使用闭包替代了原来的 `w_sep!` macro。

### IN-03: stats/mod.rs validate_time_str 不必要的公开重导出

**Files modified:** `src/stats/mod.rs`
**Commit:** `a1707bd`
**Applied fix:** 移除了 `pub use config::validate_time_str` 重导出。经验证，`validate_time_str` 没有任何外部消费者，仅在 `stats/config.rs` 内部使用，不需要从 `stats/mod.rs` 重导出。

## Skipped Issues

### IN-01: config/mod.rs unnecessary #[allow(unused_imports)]

**File:** `src/config/mod.rs:16-25`
**Reason:** 验证后发现 `#[allow(unused_imports)]` 确实是必要的。该项目同时有 lib target 和 bin target，在 bin target 的编译上下文中，`pub use` 重导出的符号（`StatsConfig`、`ErrorLogConfig`、`CsvExporterConfig` 等）未被 bin target 直接消费，触发 `unused_imports` lint（`-D warnings` 对所有 target 生效）。移除这些 allow 属性会导致 `cargo clippy --all-targets -- -D warnings` 失败。
**Original issue:** 4 个 pub use 各自独立使用 #[allow(unused_imports)] 压制，审查者认为 lib crate 的 pub use 不会触发该 lint。

### IN-02: watch/mod.rs multiple #[allow(unused_imports)]

**File:** `src/cli/watch/mod.rs:23-28`
**Reason:** 验证后发现剩余 3 个 `#[allow(unused_imports)]`（WatchLoopState、trigger_full_file、trigger_incremental）也是必要的。这些符号仅在集成测试 `tests/watch_incremental.rs` 中使用，在 bin target 编译上下文中未被直接引用，移除 allow 属性后会触发相同的 `unused_imports` lint。注意：WR-02 已成功移除了 collect_watch_dirs 和 format_elapsed_hms 这两个真正的死导出。

---

_Fixed: 2026-06-07_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
