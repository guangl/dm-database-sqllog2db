---
phase: 63-test-coverage
plan: "03"
subsystem: testing
tags: [rust, unit-tests, error-handling, coverage, prescan]

# Dependency graph
requires: []
provides:
  - "src/error.rs mod tests 扩展：覆盖 ConfigError/FileError/ExportError/ParserError/Error::Io/Error::Interrupted 全变体的 is_fatal/severity/suggestion 方法"
  - "src/cli/run/prescan.rs 新增 mod tests 块：覆盖 build_indicator_filters 双分支（min_row_count=0/正值/空）与 build_sql_exclude_filters 多元素/空分支"
affects: [63-test-coverage]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "错误变体直接构造 + 方法断言模式（contains 子串检查 suggestion）"
    - "私有函数通过 super:: 在 mod tests 中访问的模式"

key-files:
  created: []
  modified:
    - src/error.rs
    - src/cli/run/prescan.rs

key-decisions:
  - "error.rs 中 suggestion 断言使用子串（contains），而非完整字符串等价，以允许措辞微调而测试仍通过"
  - "prescan.rs mod tests 只测试三个私有 build_* 纯函数，不涉及文件系统或 rayon 线程池，保持测试快速且无外部依赖"

patterns-established:
  - "测试 is_fatal/severity/suggestion 三合一：同一 test fn 内依次断言三个方法，减少重复构造代码"
  - "prescan 私有函数测试：use super::*; 即可访问 build_indicator_filters 等 fn-level 私有函数"

requirements-completed:
  - TEST-02

# Metrics
duration: 6min
completed: 2026-06-03
---

# Phase 63 Plan 03: 错误变体方法覆盖与 prescan 私有函数测试 Summary

**为 error.rs 全 5 类错误变体追加 16 个 is_fatal/severity/suggestion 单元测试，并在 prescan.rs 新建 mod tests 块覆盖 build_indicator_filters 的 min_row_count=0/正值/空三分支及 build_sql_exclude_filters 双分支**

## Performance

- **Duration:** 6 min
- **Started:** 2026-06-03T08:52:47Z
- **Completed:** 2026-06-03T08:58:33Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- error.rs mod tests 从 3 个扩展至 19 个测试，新增 16 个覆盖 ConfigError(5)/FileError(3)/ExportError(2)/ParserError(3)/Error::Io/Error::Interrupted/ErrorSeverity Display
- prescan.rs 新建 #[cfg(test)] mod tests 块，包含 6 个测试函数覆盖私有 build_indicator_filters 与 build_sql_exclude_filters/build_sql_include_filters
- 全部 291 个库测试通过，三道质量门禁（clippy/fmt/test）全绿

## Task Commits

1. **Task 1: src/error.rs mod tests 末尾追加错误变体方法测试** - `1e2f916` (test)
2. **Task 2: src/cli/run/prescan.rs 末尾新建 mod tests 块** - `3cd6d88` (test)

## Files Created/Modified

- `src/error.rs` — 在现有 mod tests 块末尾追加 16 个测试函数（lines 289–590），覆盖所有错误变体方法
- `src/cli/run/prescan.rs` — 文件末尾新增 #[cfg(test)] mod tests 块（lines 139–217），含 6 个 build_* 函数测试

## 新增测试函数清单

### src/error.rs（新增 16 个，原有 3 个不变）

| 测试函数 | 覆盖变体 | 覆盖方法 |
|---------|---------|---------|
| `test_config_not_found_is_fatal_critical_suggestion` | ConfigError::NotFound | is_fatal/severity/suggestion |
| `test_config_parse_failed_suggestion_mentions_toml` | ConfigError::ParseFailed | is_fatal/severity/suggestion |
| `test_config_invalid_log_level_suggestion` | ConfigError::InvalidLogLevel | is_fatal/suggestion |
| `test_config_invalid_value_suggestion` | ConfigError::InvalidValue | is_fatal/suggestion |
| `test_config_no_exporters_suggestion` | ConfigError::NoExporters | is_fatal/suggestion |
| `test_file_already_exists_is_fatal` | FileError::AlreadyExists | is_fatal/severity/suggestion |
| `test_file_write_failed_not_fatal_error_severity` | FileError::WriteFailed | is_fatal/severity/suggestion |
| `test_file_create_directory_failed_is_fatal` | FileError::CreateDirectoryFailed | is_fatal/severity/suggestion |
| `test_export_write_failed_not_fatal_error_severity` | ExportError::WriteFailed | is_fatal/severity/suggestion |
| `test_export_database_failed_is_fatal_critical` | ExportError::DatabaseFailed | is_fatal/severity/suggestion |
| `test_io_error_is_fatal_critical` | Error::Io | is_fatal/severity/suggestion |
| `test_interrupted_is_fatal_critical` | Error::Interrupted | is_fatal/severity/suggestion |
| `test_parser_path_not_found_suggestion` | ParserError::PathNotFound | is_fatal/severity/suggestion |
| `test_parser_invalid_path_suggestion` | ParserError::InvalidPath | is_fatal/suggestion |
| `test_parser_read_dir_failed_is_fatal` | ParserError::ReadDirFailed | is_fatal/suggestion |
| `test_error_severity_display_strings` | ErrorSeverity Display | format!("{}", ...) |

### src/cli/run/prescan.rs（新增 6 个）

| 测试函数 | 覆盖函数 | 覆盖路径 |
|---------|---------|---------|
| `test_build_indicator_filters_min_row_count_zero` | build_indicator_filters | prescan.rs:16-17（min_r==0 → build()） |
| `test_build_indicator_filters_min_row_count_positive` | build_indicator_filters | prescan.rs:18-20（min_r>0 → rowcount_gt(min_r-1)） |
| `test_build_indicator_filters_empty_returns_empty` | build_indicator_filters | prescan.rs:8-29（所有 Option 为 None） |
| `test_build_sql_exclude_filters_multiple_returns_correct_count` | build_sql_exclude_filters | prescan.rs:40-47（非空 excludes） |
| `test_build_sql_exclude_filters_none_returns_empty` | build_sql_exclude_filters | prescan.rs:42（unwrap_or(&[]) None 路径） |
| `test_build_sql_include_filters_multiple` | build_sql_include_filters | prescan.rs:31-38（非空 includes） |

## Decisions Made

- suggestion 断言使用子串匹配（`contains()`），而非完整字符串等价，使测试对措辞调整有容错性
- prescan.rs 测试只覆盖纯函数（build_* 系列），不测试依赖文件系统的 scan_log_file_for_matches（按 PATTERNS.md D-04：非 UTF-8 路径 warn 分支标注为难以测试）

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- `cargo fmt` 在两个文件中均需要对部分 `assert!` 调用重新换行格式化（单行过长），通过运行 `cargo fmt` 自动修正，不影响逻辑。

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 03 的两个低覆盖区域（error.rs 78% / prescan.rs 70%）已补测，覆盖率预计提升显著
- Plan 04 可继续处理其他低覆盖区域

## Self-Check: PASSED

- [x] `src/error.rs` 存在且包含 19 个测试函数
- [x] `src/cli/run/prescan.rs` 存在且包含 6 个 test_build_* 函数
- [x] Task 1 commit `1e2f916` 存在
- [x] Task 2 commit `3cd6d88` 存在
- [x] `cargo test --lib` 291 passed; 0 failed
- [x] `cargo clippy --all-targets -- -D warnings` 通过
- [x] `cargo fmt --check` 通过

---
*Phase: 63-test-coverage*
*Completed: 2026-06-03*
