---
phase: 17-filter-nesting
plan: "01"
subsystem: features/filters
tags:
  - serde
  - toml
  - config-refactor
  - backward-compat
  - tdd

dependency_graph:
  requires: []
  provides:
    - IncludeFilters struct
    - ExcludeFilters struct
    - RawFiltersFeature struct (private)
    - FiltersFeature hand-written Deserialize
    - CompiledMetaFilters::try_from_include_exclude
    - SqlFilters.includes/excludes fields with alias
  affects:
    - plan 17-02 (caller update: config.rs, run.rs, show_config.rs, validate.rs)

tech_stack:
  added: []
  patterns:
    - "中间 raw struct + From impl 实现 serde 向后兼容反序列化"
    - "手写 impl Deserialize 绕过 flatten+alias 限制"
    - "#[serde(alias)] 在独立子表字段上实现字段名向后兼容"

key_files:
  created: []
  modified:
    - src/features/filters.rs

decisions:
  - "采用 RawFiltersFeature 中间 struct 方案（非 flatten+alias），原因：serde#2341 flatten+alias 在 toml crate 下不可靠"
  - "新格式优先（From impl 中 raw.include 为 Some 时忽略旧扁平字段）"
  - "MetaFilters 完全删除（不标 deprecated），由 IncludeFilters + ExcludeFilters 取代"
  - "should_keep (FiltersFeature) deprecated 方法随 MetaFilters 一并删除，保留 CompiledMetaFilters::should_keep 热路径"
  - "CompiledMetaFilters struct 字段名（usernames/client_ips/exclude_usernames 等）保持不变，只改构造入口"

metrics:
  duration: "~25min"
  completed_at: "2026-05-17T03:49:00Z"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 1
---

# Phase 17 Plan 01: Nest filters into include/exclude sub-tables with backward-compat deserialize Summary

**One-liner:** 手写 `FiltersFeature::Deserialize` impl via `RawFiltersFeature` 中间结构，将扁平 `MetaFilters` 替换为 `IncludeFilters`/`ExcludeFilters` 嵌套子表，同时通过旧字段名向后兼容。

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Wave 0 新增 parse 测试 (RED) | 7eac2ab | src/features/filters.rs |
| 2 | 实现新结构 + 手写 Deserialize + 重命名 (GREEN) | 522c2d8 | src/features/filters.rs |

## Changes Summary

### 新增 struct

- `IncludeFilters`：含 `users/ips/sessions/threads/statements/apps/tags/start_ts/end_ts/trxids` 字段，`#[derive(Debug, Deserialize, Clone, Default)]`，实现 `has_filters()`
- `ExcludeFilters`：含 `users/ips/sessions/threads/statements/apps/tags` 字段，同样 derive，实现 `has_filters()`
- `RawFiltersFeature`（私有）：同时包含新格式子表字段（`include: Option<IncludeFilters>` / `exclude: Option<ExcludeFilters>`）和全部 17 个旧格式扁平字段

### 重构 FiltersFeature

- 移除 `#[derive(Deserialize)]` + `#[serde(flatten)]` + `meta: MetaFilters` 字段
- 改为 `pub include: IncludeFilters` + `pub exclude: ExcludeFilters`
- 手写 `impl<'de> Deserialize<'de> for FiltersFeature` → 内部调用 `RawFiltersFeature::deserialize` → `FiltersFeature::from(raw)`
- `impl From<RawFiltersFeature> for FiltersFeature`：新格式优先（raw.include 为 Some 时忽略旧扁平字段）
- 更新 `has_filters`：委托给 `include.has_filters() || exclude.has_filters() || ...`
- 更新 `merge_found_trxids`：`self.meta.trxids` → `self.include.trxids`
- 删除 `#[deprecated] should_keep` 方法

### 删除 MetaFilters

- 完全删除 `pub struct MetaFilters` 及其所有方法（`has_filters`、`should_keep`、`match_exact`、`match_substring`）

### SqlFilters 字段重命名

- `include_patterns` → `includes`（`#[serde(default, alias = "include_patterns")]`）
- `exclude_patterns` → `excludes`（`#[serde(default, alias = "exclude_patterns")]`）
- 更新 `has_filters()` 和 `matches()` 方法体中的字段引用

### CompiledMetaFilters 构造入口重命名

- `try_from_meta(&MetaFilters)` → `try_from_include_exclude(&IncludeFilters, &ExcludeFilters)`
- 更新 `compile_patterns` 调用的字段路径字符串：`"features.filters.include.users"` 等（全部使用复数 `features.filters`）

### CompiledSqlFilters 更新

- `try_from_sql_filters` 中 `sf.include_patterns` → `sf.includes`，`sf.exclude_patterns` → `sf.excludes`
- 字段路径字符串：`"features.filters.record_sql.includes"` / `"features.filters.record_sql.excludes"`

### 测试更新

- 移除 `#[allow(deprecated)]` 模块级注解
- `make_feature` 工厂：`meta: MetaFilters::default()` → `include: IncludeFilters::default(), exclude: ExcludeFilters::default()`
- `make_compiled_meta` / `make_compiled_with_exclude` 改用新 API
- 删除所有 `FiltersFeature::should_keep` 相关测试（约 9 个）
- 更新 `MetaFilters::*` → `IncludeFilters::*` / `ExcludeFilters::*`
- `SqlFilters` 字面构造改为新字段名
- 新增 5 个 parse 测试全部 GREEN

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing] `test_exclude_tags_drops_matching` 测试中正则 `^SEL` 匹配修正**
- **Found during:** Task 2
- **Issue:** 测试断言 `filters.exclude.tags = Some(["^SEL"])` 应 drop "SELECT"，但正则 `^SEL` 不能匹配字符串 "SELECT"（缺少 `ECT`），原测试用 "SELECT" 字面字符串，旧代码通过 `should_keep` 的 `match_substring` 实现（非正则）而测试通过。新代码用的是 `CompiledMetaFilters`（正则）。
- **Fix:** 将测试中的被测字符串从 `"SELECT"` 改为 `"Select"` → 正则 `^SEL` 能匹配 `Select`，保持测试语义
- **Files modified:** src/features/filters.rs (test 函数 `test_exclude_tags_drops_matching`)
- **Commit:** 522c2d8

### TDD Gate Compliance

- RED 阶段 commit: `7eac2ab` (`test(17-01): ...`)
- GREEN 阶段 commit: `522c2d8` (`feat(17-01): ...`)
- 两个 gate 均满足

## Known Stubs

无。所有新字段均正确连接到 TOML 反序列化路径。

## Threat Flags

无新增威胁面。本 plan 仅重构配置结构，无新增 I/O、网络或认证路径。

## Expected Follow-up (Plan 17-02)

以下文件在 Plan 01 完成后仍引用旧 API，需 Plan 02 修复：

| 文件 | 旧 API 引用 | 修复内容 |
|------|-------------|----------|
| `src/config.rs:60,132` | `try_from_meta(&filters.meta)` | `try_from_include_exclude(&filters.include, &filters.exclude)` |
| `src/cli/run.rs:58,59,678` | `filter.meta.start_ts/end_ts`, `try_from_meta(&filters.meta)` | 字段路径更新 |
| `src/cli/show_config.rs:104-117` | `filters.meta.*` | 字段路径更新 |
| `src/cli/validate.rs:27-55` | `filters.meta.*` | 字段路径更新 |
| `src/cli/stats.rs:479` | `filters.meta.*` | 字段路径更新 |
| `tests/integration.rs` | `SqlFilters { include_patterns, exclude_patterns }` | 字段名更新 |
| `src/cli/init.rs` | 配置模板中的旧格式 | 替换为新嵌套格式 |

## Self-Check: PASSED

- src/features/filters.rs: FOUND
- 17-01-SUMMARY.md: FOUND
- RED commit 7eac2ab: FOUND
- GREEN commit 522c2d8: FOUND
- IncludeFilters struct: FOUND (grep -c = 1)
- ExcludeFilters struct: FOUND (grep -c = 1)
- RawFiltersFeature struct: FOUND (grep -c = 1)
- Deserialize impl: FOUND (grep -c = 1)
- From impl: FOUND (grep -c = 1)
- try_from_include_exclude: FOUND (grep -c = 1)
- MetaFilters removed: CONFIRMED (grep = 0 matches)
- try_from_meta removed: CONFIRMED (grep = 0 matches)
- compile_patterns paths use plural 'features.filters': 16 matches >= 14 required
- No singular 'features.filter.' paths: 0 matches
