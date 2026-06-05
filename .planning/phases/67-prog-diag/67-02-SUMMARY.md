---
phase: 67-prog-diag
plan: "02"
subsystem: error
tags: [rust, error-stats, error-kind, parse-error, tdd]

requires:
  - phase: 67-01
    provides: "make_progress_bar/tick_progress 升级，processor.rs 现有 Err 路径"

provides:
  - "ErrorKind 枚举 (EncodingError/FieldMissing/ParseFailed)"
  - "classify_error_kind(raw) 启发式分类函数"
  - "truncate_to_120_chars(raw) UTF-8 安全截断"
  - "ParseErrorRecord struct (file_path/line_number/raw_truncated/kind)"
  - "ErrorStats.by_type HashMap<ErrorKind, u64>"
  - "ErrorStats.filtered_out u64"
  - "ErrorStats.parse_error_records Vec<ParseErrorRecord>"
  - "ErrorStats.add_parse_error_with_kind(kind)"
  - "ErrorStats.merge() 扩展聚合三新字段"
  - "Config.error: Option<ErrorLogConfig>"
  - "processor.rs process_log_file Err 路径收集 ParseErrorRecord（上限 10000）"

affects: [67-03]

tech-stack:
  added: []
  patterns: [TDD RED/GREEN, HashMap entry API, struct update syntax ..Default::default()]

key-files:
  created: []
  modified:
    - src/error.rs
    - src/config/mod.rs
    - src/cli/run/processor.rs

key-decisions:
  - "TDD RED/GREEN：因 pre-commit hook 运行 clippy，RED 测试无法单独提交，与 GREEN 合并为一个提交"
  - "dead_code 静默：ErrorLogConfig::file/Config::error/ParseErrorRecord 字段/ErrorKind::kind_display 在 Plan 03 才会被使用，用 #[allow(dead_code)] 过渡注解"
  - "kind_display 使用 value receiver self（ErrorKind 是 Copy 类型），符合 clippy trivially_copy_pass_by_ref 规则"
  - "测试中用 struct update syntax ErrorStats { filtered_out: 2, ..Default::default() } 替代字段赋值，满足 clippy field_reassign_with_default"

requirements-completed: [DIAG-01, DIAG-02]

duration: 11min
completed: "2026-06-05"
---

# Phase 67 Plan 02: ErrorStats 扩展 + ErrorKind 分类 + Config.error + 收集逻辑

**ErrorStats 新增 by_type/filtered_out/parse_error_records 三字段，引入 ErrorKind 枚举、ParseErrorRecord struct 及 classify_error_kind/truncate_to_120_chars 辅助函数，process_log_file Err 路径收集 ParseErrorRecord（上限 10000），Config.error 字段接入 TOML [error] 段。**

## Performance

- **Duration:** 11 min
- **Started:** 2026-06-05T06:24:35Z
- **Completed:** 2026-06-05T06:35:00Z
- **Tasks:** 3 (Task 1 RED + Task 2 GREEN + Task 3 收集逻辑，合并为 1 个 feat 提交)
- **Files modified:** 3

## Accomplishments

- ErrorKind 枚举和启发式分类函数（DIAG-02 分类底盘）
- UTF-8 字符安全截断函数 truncate_to_120_chars（DIAG-01 原文截断）
- ErrorStats 三新字段 + add_parse_error_with_kind + merge 扩展（并行路径聚合覆盖）
- Config.error: Option<ErrorLogConfig>（D-07：TOML [error] file 配置生效）
- process_log_file Err 路径收集完整 ParseErrorRecord，上限 10000 记录（T-67-03 DoS 缓解）
- 4 个 TDD 单元测试全 GREEN；cargo test --lib 341 passed

## Task Commits

由于 pre-commit hook 运行 cargo clippy，RED 测试引用未定义类型会导致 hook 失败，所以 RED 和 GREEN 合并为单一提交：

1. **Task 1+2+3 (RED+GREEN)** - `bc81d53` (feat)：ErrorKind/ParseErrorRecord/classify/truncate + Config.error + processor collection

**Plan metadata commit:** (待 final commit)

## Files Created/Modified

- `src/error.rs` — ErrorKind 枚举、ParseErrorRecord、classify_error_kind、truncate_to_120_chars；ErrorStats 扩展三字段、add_parse_error_with_kind、merge 扩展；4 个新单元测试
- `src/config/mod.rs` — ErrorLogConfig struct；Config.error: Option<ErrorLogConfig>
- `src/cli/run/processor.rs` — use 追加 classify_error_kind/truncate_to_120_chars/ParseErrorRecord/ParseError；Err 路径收集 ParseErrorRecord，add_parse_error_with_kind 替换 add_parse_error

## Decisions Made

- TDD RED 无法独立提交（pre-commit hook clippy 强制）：RED + GREEN 合并为一个 feat commit
- dead_code 过渡：Plan 03 才使用的 pub items 用 `#[allow(dead_code)]` 注解，Plan 03 使用后自动移除警告
- trivially_copy_pass_by_ref：kind_display 使用 `self` 而非 `&self`（ErrorKind 是 1 字节 Copy 类型）
- 测试赋值方式：用 struct update syntax 满足 clippy field_reassign_with_default lint

## Deviations from Plan

**1. [Rule 1 - Bug] TDD RED/GREEN 合并为单一提交**
- **Found during:** Task 1 (RED 提交尝试)
- **Issue:** pre-commit hook 运行 cargo clippy，RED 测试引用的 ErrorKind 等类型尚未定义，导致 hook 报告编译错误拒绝提交
- **Fix:** 直接进入 GREEN 阶段（Task 2+3），与 RED 合并为一个 feat 提交
- **Files modified:** src/error.rs, src/config/mod.rs, src/cli/run/processor.rs
- **Verification:** cargo test --lib 341 passed; cargo clippy --all-targets -- -D warnings 通过
- **Committed in:** bc81d53

---

**Total deviations:** 1 auto-handled（TDD hook 约束）
**Impact on plan:** 仅影响提交粒度（RED+GREEN 合并），不影响功能正确性；4 个测试全 GREEN 确认 TDD 语义保持。

## Issues Encountered

- `#[allow(dead_code)]` 注解在 `pub` struct 字段上的作用范围：必须放在字段级别（`#[allow(dead_code)] pub field: T`）而非 struct 级别，才能精确静默单个字段
- binary crate 的 dead_code lint 会分析所有 pub 但未被 bin 入口链到的符号；这与 library crate 行为不同

## Known Stubs

Plan 03 中 `write_error_log` 和摘要扩展才会真正读取 `ParseErrorRecord` 字段、`ErrorLogConfig::file`、`Config::error`；当前用 `#[allow(dead_code)]` 标注，Plan 03 实现后注解应移除。

## Threat Flags

- **T-67-03 mitigated:** `parse_error_records.len() < 10_000` 守卫在 processor.rs line 238，超出上限只计数不收集，防止 DoS
- **T-67-04 mitigated:** `truncate_to_120_chars` 在 processor.rs line 241 调用，raw 内容已截断后才写入 parse_error_records

## Next Phase Readiness

Plan 03 可直接使用：
- `run_stats.parse_error_records` — 批量写出 error log（D-10）
- `run_stats.by_type` — 摘要 errors by type 分布（D-16）
- `run_stats.filtered_out` — 摘要 filtered 统计（D-15/16）
- `cfg.error.as_ref()?.file` — error log 目标路径（D-07）

## Self-Check: PASSED

- [x] ErrorKind/ParseErrorRecord/classify_error_kind/truncate_to_120_chars 存在于 src/error.rs
- [x] ErrorStats.by_type/filtered_out/parse_error_records 字段存在
- [x] Config.error: Option<ErrorLogConfig> 存在于 src/config/mod.rs
- [x] processor.rs Err 路径使用 add_parse_error_with_kind + parse_error_records.push + len < 10_000
- [x] cargo clippy --all-targets -- -D warnings 通过
- [x] cargo test --lib 341 passed（含 4 个新测试）
- [x] bc81d53 commit 存在

---
*Phase: 67-prog-diag*
*Completed: 2026-06-05*
