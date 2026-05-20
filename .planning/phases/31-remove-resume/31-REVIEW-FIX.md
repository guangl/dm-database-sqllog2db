---
phase: 31-remove-resume
fixed_at: 2026-05-20T14:00:00Z
review_path: .planning/phases/31-remove-resume/31-REVIEW.md
iteration: 1
findings_in_scope: 8
fixed: 8
skipped: 0
status: all_fixed
---

# Phase 31: Code Review Fix Report

**Fixed at:** 2026-05-20T14:00:00Z
**Source review:** .planning/phases/31-remove-resume/31-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 8
- Fixed: 8
- Skipped: 0

## Fixed Issues

### BL-01: bench_filters.rs 全部 6 个场景使用废弃格式，基准测试结果无效

**Files modified:** `benches/bench_filters.rs`
**Commit:** a5ca5dc
**Applied fix:** 将 6 个配置函数 (`cfg_pipeline_passthrough`, `cfg_trxid_small`, `cfg_trxid_large`, `cfg_indicator_prescan`, `cfg_exclude_passthrough`, `cfg_exclude_active`) 中的 `[features.filters]` 格式迁移为当前 `[filter]` 格式。具体变更：
- `[features.filters]` 替换为 `[filter]`
- `start_ts`、`trxids` 移到 `[filter.include]` 子表
- `min_runtime_ms` 移到 `[filter.indicators]` 子表
- `exclude_usernames` 改为 `[filter.exclude]` 下的 `users` 字段

### WR-03: `process_csv_parallel` 的 `_quiet` 参数已成为死代码

**Files modified:** `src/cli/run/parallel.rs`, `src/cli/run/mod.rs`
**Commit:** 1ca7f99
**Applied fix:** 从 `process_csv_parallel` 函数签名中移除 `_quiet: bool` 参数及对应的调用实参 `quiet`。

### WR-04: `process_log_file` 的 `limit` 参数名存在误导性

**Files modified:** `src/cli/run/processor.rs`
**Commit:** 3c965ef
**Applied fix:** 将参数 `limit: Option<usize>` 重命名为 `remaining: Option<usize>` 以匹配其语义（跨文件的剩余配额），同步更新函数体中的引用和 doc comment。

### WR-05: `lang.rs` 模块级 `#![allow(dead_code)]` 抑制了合法的死代码检测

**Files modified:** `src/lang.rs`
**Commit:** 62615bd
**Applied fix:** 移除模块级 `#![allow(dead_code)]` 属性，改为在仅由 binary crate 使用的 `pub(crate) fn detect()` 和 `pub(crate) fn apply_zh()` 上添加独立的 `#[allow(dead_code)]` 注解。其私有辅助函数因有调用者而不会被标记为死代码。

### IN-01: `FileError::ReadFailed` 死代码变体

**Files modified:** `src/error.rs`
**Commit:** bc2213a
**Applied fix:** 移除 `FileError` 枚举中已无任何构造点的 `ReadFailed` 变体及相关 `#[allow(dead_code)]` 属性。

### IN-02: 文档和 README 中存在过时的模块路径引用

**Files modified:** `docs/architecture.md`, `README.md`
**Commit:** deb0051
**Applied fix:** 更新过时的模块路径引用：
- `src/features/` -> `src/pipeline/`
- `cli/run.rs` -> `cli/run/mod.rs`
- `features/mod.rs` -> `pipeline/mod.rs`
- `features/filters.rs` -> `pipeline/filters/mod.rs`

### IN-03: 基准测试 TOML 配置中的 `[error]` 段被静默忽略

**Files modified:** `benches/bench_filters.rs`, `benches/bench_csv.rs`, `benches/bench_sqlite.rs`
**Commit:** 9342289
**Applied fix:** 从 3 个基准测试文件的 TOML 字符串中移除已无实际作用的 `[error]` 配置段，避免误导维护者。

### IN-04: `validate_filter()` 与 `validate_and_compile()` 编译逻辑重复

**Files modified:** `src/config/validate.rs`
**Commit:** bb768eb
**Applied fix:** 重构 `validate()` 方法，改为委托给 `validate_and_compile()` 并丢弃编译结果。移除了重复的 `validate_filter()` 方法，消除了每次 validate 调用中的一次不必要的正则编译。

## Notes

- 由于工作分支 `milestone/v1.7` 在此代理运行期间有新的提交，`merge --ff-only` 无法自动快进。
- 修复提交位于临时分支 `gsd-reviewfix/31-82843` 上，需要手动合并：
  ```bash
  git merge gsd-reviewfix/31-82843
  git branch -d gsd-reviewfix/31-82843
  ```
- 所有 622 个测试通过，clippy 零警告。

---

_Fixed: 2026-05-20T14:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
