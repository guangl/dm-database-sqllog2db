---
phase: 67-prog-diag
plan: "03"
subsystem: cli/run
status: complete
tags: [rust, error-log, summary-stats, filtered-out, hint, tdd]

requires:
  - phase: 67-02
    provides: "ErrorStats.by_type/filtered_out/parse_error_records + Config.error"

provides:
  - "print_run_summary 扩展：errors by type 分布行 + filtered 统计行 + encoding/field hint"
  - "write_error_log 函数：BufWriter 覆盖写出 + 10k 上限 + 写失败 warn"
  - "normalize_and_export !passes 路径递增 filtered_out（D-15）"
  - "test_error_log_written：handle_run 集成测试验证 error log 写出"
  - "test_hint_output / test_run_summary：单元层防回归测试"

affects: []

tech-stack:
  added: []
  patterns: [TDD RED+GREEN 合并提交, BufWriter 批量写出, clippy::cast_precision_loss 豁免]

key-files:
  created: []
  modified:
    - src/cli/run/mod.rs
    - src/cli/run/processor.rs
    - src/cli/run/tests.rs
    - src/config/mod.rs
    - src/error.rs

key-decisions:
  - "TDD RED/GREEN 合并为单一提交（pre-commit hook 运行 cargo clippy，RED 阶段引用实现符号导致编译失败）"
  - "cast_precision_loss 用 #[allow] 豁免：filtered_out/total_read 为 u64，转 f64 用于百分比计算，精度损失在可接受范围（ROADMAP 指定 {pct:.1}% 格式）"
  - "ParseErrorRecord.file_path 保留 #[allow(dead_code)]：write_error_log 格式不含 file_path（与 PLAN D-09 规范一致），字段保留供未来格式扩展"
  - "test_error_log_written 中无效行放文件前面：dm-database-parser-sqllog 2.0.2 以 \\n20 时间戳为记录边界，后置无效行会被合并到前一条记录体中，前置无效行才独立触发 ParseError::InvalidFormat"

requirements-completed: [PROG-03, DIAG-03]

duration: 35min
completed: "2026-06-05"
---

# Phase 67 Plan 03: 摘要分组统计 + filtered 率 + hint + error log 写出

**print_run_summary 扩展 errors by type 分布输出、filtered 百分比统计及编码/字段 hint；新增 write_error_log 批量写出 ParseErrorRecord；normalize_and_export !passes 路径递增 filtered_out；三个集成/单元测试全 GREEN。**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-06-05T11:00:00Z
- **Completed:** 2026-06-05
- **Tasks:** Task 1 (RED) + Task 2 (GREEN)，合并为 1 个 feat 提交
- **Files modified:** 5

## Accomplishments

- `print_run_summary` 在 `has_errors()` 块内追加 `errors by type: encoding={n}, field_missing={n}, parse_failed={n}` 行（D-16/SC4）
- `print_run_summary` 追加 `filtered: {n} records ({pct:.1}% of {total} total)` 行（D-15/SC5）
- `print_run_summary` 追加编码 hint（`by_type[EncodingError] > 0`）和字段 hint（`by_type[FieldMissing] > 0`）（D-14）
- 新增 `write_error_log(cfg, &stats)` 私有函数：`BufWriter` + `File::create`（覆盖）+ 上限判断 + 写失败 `log::warn!`（D-10/D-09）
- `handle_run` 末尾在 `print_run_summary` 之后调用 `write_error_log(cfg, &run_stats)`
- `normalize_and_export` 的 `!passes` 路径追加 `file_stats.filtered_out += 1`（D-15）
- 移除 `src/config/mod.rs` 和 `src/error.rs` 中 Plan 02 的过渡 `#[allow(dead_code)]` 注解（相关字段已在 Plan 03 激活）
- 新增 `test_error_log_written`（完整 handle_run 集成测试，断言 errors.log 存在且含 `[ERROR] line` 和 `reason:`）
- 新增 `test_hint_output`（ErrorStats 字段直读 + print_run_summary 不 panic 防回归）
- 新增 `test_run_summary`（filtered_out=5 + print_run_summary 不 panic 防回归）
- `cargo test --lib` 344 passed，0 failed；Phase 67 全三个 Plan 测试无回归

## Task Commits

由于 pre-commit hook 运行 cargo clippy，RED 测试引用未实现符号会导致 hook 失败，RED 和 GREEN 合并为单一提交：

1. **Task 1+2 (RED+GREEN)** - `67feea0`：feat(67-03): 摘要分组统计 + filtered 率 + hint + error log 写出

## Files Created/Modified

- `src/cli/run/mod.rs` — print_run_summary 扩展（errors by type / filtered / hint 四行）+ write_error_log 新增 + handle_run 末尾调用
- `src/cli/run/processor.rs` — normalize_and_export !passes 路径追加 file_stats.filtered_out += 1
- `src/cli/run/tests.rs` — 新增 test_error_log_written / test_hint_output / test_run_summary
- `src/config/mod.rs` — 移除 ErrorLogConfig 和 Config.error 的过渡 #[allow(dead_code)] 注解
- `src/error.rs` — 移除 ErrorKind::kind_display 的 #[allow(dead_code)] 注解；ParseErrorRecord.file_path 保留注解（字段未在 write_error_log 格式中使用）

## Decisions Made

- TDD RED 无法独立提交（pre-commit hook clippy 强制）：RED + GREEN 合并为一个 feat commit
- cast_precision_loss 豁免：filtered_out/total_read 为 u64 转 f64 用于百分比，符合业务精度要求
- test_error_log_written 日志内容顺序：无效行放文件开头，确保解析器以独立记录处理并返回 InvalidFormat

## Deviations from Plan

**1. [Rule 1 - Bug] TDD RED/GREEN 合并为单一提交**
- **Found during:** Task 1 (RED 提交尝试)
- **Issue:** pre-commit hook 运行 cargo clippy，RED 测试引用的 write_error_log 等符号尚未定义，导致 hook 报告编译错误拒绝提交（与 Plan 02 相同情况）
- **Fix:** 直接进入 GREEN 阶段，与 RED 合并为一个 feat 提交
- **Files modified:** 全部 5 个文件
- **Verification:** cargo test --lib 344 passed; cargo clippy --all-targets -- -D warnings 通过
- **Committed in:** 67feea0

**2. [Rule 1 - Bug] test_error_log_written 日志内容调整**
- **Found during:** Task 2 (GREEN 测试验证)
- **Issue:** Plan 中无效行放在合法 SEL 行之后，但 dm-database-parser-sqllog 2.0.2 以 `\n20` 为记录边界，后置无效行被合并到前一条记录体中不触发 ParseError，导致 parse_error_records 为空，error log 未写出
- **Fix:** 将无效行调整到文件开头，使其成为独立记录，正确触发 ParseError::InvalidFormat
- **Files modified:** src/cli/run/tests.rs
- **Root cause:** 解析器行为与 Plan 假设不符（Plan 假设任意位置的无效行都会触发错误）

---

**Total deviations:** 2 auto-handled（均不影响功能正确性）

## Known Stubs

None.

## Threat Flags

- **T-67-07 mitigated:** `write_error_log` 使用 `BufWriter` + `parse_error_records.len() >= 10_000` 截断判断；最坏 10k 行 < 1MB 文本；写失败仅 `log::warn!` 不终止流程
- T-67-06 accepted: `cfg.error.file` 来源 TOML，用户自定义路径，与现有 logging/csv 配置模型一致
- T-67-08 accepted: hint 为固定中文字符串，无敏感信息

## Self-Check: PASSED

- [x] `file_stats.filtered_out += 1` 存在于 processor.rs
- [x] `fn write_error_log` 存在于 mod.rs
- [x] `write_error_log(cfg, &run_stats)` 调用存在于 mod.rs (handle_run)
- [x] `errors by type:` 存在于 mod.rs
- [x] `filtered: {} records` 存在于 mod.rs
- [x] `encoding_error — 建议检查` 存在于 mod.rs
- [x] `field_missing — 建议确认` 存在于 mod.rs
- [x] `[ERROR] line` 存在于 mod.rs
- [x] `[truncated at 10000 records]` 存在于 mod.rs
- [x] test_error_log_written / test_hint_output / test_run_summary 均存在于 tests.rs
- [x] cargo clippy --all-targets -- -D warnings 通过
- [x] cargo test --lib 344 passed（含 Plan 01 + Plan 02 + Plan 03 测试）
- [x] 67feea0 commit 存在

---
*Phase: 67-prog-diag*
*Completed: 2026-06-05*
