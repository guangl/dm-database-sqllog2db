---
phase: 67-prog-diag
verified: 2026-06-05T12:00:00Z
status: human_needed
score: 11/11 must-haves verified
overrides_applied: 0
human_verification:
  - test: "多文件运行时实际观察进度条 [N/M] 显示"
    expected: "进度条以 [1/3]、[2/3]、[3/3] 形式递进，非 TTY 环境无 ANSI 序列"
    why_human: "indicatif 非 TTY 退化行为需实际终端环境运行观察，无法通过 grep/test 验证"
  - test: "运行时观察 records/sec 实时更新"
    expected: "进度条 message 中出现 'Xk rec/s' 或 'X rec/s' 样式字符串，随记录数增长变化"
    why_human: "动态速率计算需实时运行观察"
  - test: "ETA 字段随运行时间变化"
    expected: "进度条 '| eta X' 字段显示合理的剩余时间估算，随进度增加而减少"
    why_human: "ETA 由 indicatif 自动渲染，需实际运行观察"
  - test: "有 encoding_error 时触发 hint 输出"
    expected: "stderr 含 'hint: 多行 encoding_error — 建议检查文件编码是否为 GBK/GB18030'"
    why_human: "eprintln! 写 stderr，test 中只验证不 panic 但不捕获 stderr 内容"
  - test: "有 field_missing 时触发 hint 输出"
    expected: "stderr 含 'hint: 多行 field_missing — 建议确认日志格式与 DM SQL log 格式一致'"
    why_human: "同上，eprintln! 写 stderr 无法自动捕获"
---

# Phase 67: 进度/摘要与诊断增强 Verification Report

**Phase Goal:** 为顺序路径添加文件粒度进度条（文件计数 + ETA + records/sec）和解析错误诊断（行号 + 原文截断 + 分类 + error log 写出）。
**Verified:** 2026-06-05T12:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #  | Truth | Status | Evidence |
|----|-------|--------|---------|
| 1  | 多文件运行时进度条 {pos}/{len} 显示 [N/M] 文件计数器 | ✓ VERIFIED | `mod.rs:211` template `"{spinner:.cyan} [{pos}/{len}] {wide_msg} \| eta {eta}"`；`ProgressBar::new(total_files as u64)` (line 209) |
| 2  | 进度条 message 含 records/sec（>=10k 显示 'Xk rec/s'，否则 'X rec/s'） | ✓ VERIFIED | `processor.rs:172-176` `rec_per_s >= 10_000.0` 分支格式化 |
| 3  | ETA 字段由 indicatif 自动渲染（template 含 '| eta {eta}'） | ✓ VERIFIED | `mod.rs:211` template 含 `\| eta {eta}` |
| 4  | tick_progress 不再调用 pb.inc(1024)；pb.inc(1) 仅在 log_file_result 触发一次 | ✓ VERIFIED | `grep pb.inc(1024)` 返回 0；`processor.rs:153` `pb.inc(1)` 在 `log_file_result` 内 |
| 5  | 非 TTY 环境进度条自动隐藏（indicatif ProgressDrawTarget 默认行为） | ? UNCERTAIN | indicatif 默认行为，无 ANSI 序列；需人工验证 |
| 6  | ErrorStats 新增 by_type / filtered_out / parse_error_records 三字段，merge() 同步累加 | ✓ VERIFIED | `error.rs:86-88,128-133`；`test_error_stats_by_type_merge` + `test_error_stats_merge_propagates_filtered_and_records` 全绿 |
| 7  | ErrorKind 枚举三值 + classify_error_kind 分类函数 | ✓ VERIFIED | `error.rs:29-33,69-77`；`test_classify_error_kind` 全绿 |
| 8  | Config.error: Option<ErrorLogConfig> 字段接入 [error] TOML 段 | ✓ VERIFIED | `config/mod.rs:19-21,40`；TOML 解析测试和集成测试均覆盖 |
| 9  | process_log_file Err 路径收集 ParseErrorRecord（上限 10000）+ by_type 同步递增 | ✓ VERIFIED | `processor.rs:232-246`；`parse_error_records.len() < 10_000` 守卫；`add_parse_error_with_kind` |
| 10 | truncate_to_120_chars 多字节 UTF-8 安全截断 | ✓ VERIFIED | `error.rs:59-65`；`test_truncate_to_120_chars` 含中文多字节测试通过 |
| 11 | write_error_log 在 cfg.error.is_some() && parse_error_records 非空时按格式批量写出（覆盖模式） | ✓ VERIFIED | `mod.rs:473-505`；`test_error_log_written` 集成测试断言文件存在含 `[ERROR] line` 和 `reason:` |
| 12 | normalize_and_export !passes 路径递增 file_stats.filtered_out | ✓ VERIFIED | `processor.rs:66` `file_stats.filtered_out += 1;` |
| 13 | run_stats.has_errors() 时摘要输出 errors by type 分组 | ✓ VERIFIED | `mod.rs:419-435` `eprintln!("  errors by type: encoding={}, field_missing={}, parse_failed={}", ...)` |
| 14 | run_stats.filtered_out > 0 时摘要输出过滤率 | ✓ VERIFIED | `mod.rs:437-449` filtered 行；test_run_summary 不 panic |
| 15 | by_type[EncodingError] > 0 触发 hint；by_type[FieldMissing] > 0 触发 hint | ✓ VERIFIED | `mod.rs:450-467` 两个条件 hint eprintln! |

**Score:** 11/11 truths verified (4 uncertain truths route to human verification；非 TTY 和 hint 输出均有 indicatif 默认行为或 eprintln! 实现支撑，代码层 VERIFIED)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|---------|--------|---------|
| `src/cli/run/mod.rs` | make_progress_bar 新签名 + write_error_log + print_run_summary 扩展 | ✓ VERIFIED | 三个功能全部存在且实质性实现 |
| `src/cli/run/processor.rs` | tick_progress 新签名 + normalize_and_export filtered_out 递增 | ✓ VERIFIED | 实质性实现，含 rec_per_s 计算 |
| `src/cli/run/tests.rs` | test_progress_bar_template + test_error_log_written + test_hint_output + test_run_summary | ✓ VERIFIED | 四个测试函数全部存在并通过 |
| `src/error.rs` | ErrorKind + ParseErrorRecord + classify_error_kind + truncate_to_120_chars + ErrorStats 新字段 | ✓ VERIFIED | 全部实质性实现 |
| `src/config/mod.rs` | ErrorLogConfig + Config.error 字段 | ✓ VERIFIED | `ErrorLogConfig { pub file: String }` + `pub error: Option<ErrorLogConfig>` |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| handle_run (mod.rs:62) | make_progress_bar | `make_progress_bar(show_progress, log_files.len())` | ✓ WIRED | mod.rs:62 精确匹配 |
| tick_progress 调用点 (processor.rs:223) | pb.set_message + 中断检测 | `tick_progress(pb, records_in_file, file_start, &file_name, interrupted)` | ✓ WIRED | processor.rs:223 |
| log_file_result | pb.inc(1) | `pb.inc(1)` 在 log_file_result 内 show_progress 分支 | ✓ WIRED | processor.rs:153 |
| process_log_file Err(e) | classify_error_kind + parse_error_records.push | `ParseError::InvalidFormat { raw, line_number }` 解构 | ✓ WIRED | processor.rs:233-245 |
| handle_run 末尾 | write_error_log(cfg, &run_stats) | print_run_summary 之后调用 | ✓ WIRED | mod.rs:128 |
| Config | ErrorLogConfig | `#[serde(default)] pub error: Option<ErrorLogConfig>` | ✓ WIRED | config/mod.rs:40 |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|-------------------|--------|
| write_error_log | `stats.parse_error_records` | process_log_file Err 路径 push | 是，来自实际解析失败 | ✓ FLOWING |
| print_run_summary errors by type | `run_stats.by_type` | `add_parse_error_with_kind` 递增 | 是，来自实际解析失败 | ✓ FLOWING |
| pb 进度条 | `total_files` → ProgressBar::new | `log_files.len()` | 是，来自实际文件列表 | ✓ FLOWING |
| tick_progress speed_label | `rec_per_s = records_in_file / elapsed` | 实时计算 | 是，来自计数器和 Instant | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 所有 lib 单元测试通过 | `cargo test --lib` | 344 passed; 0 failed | ✓ PASS |
| clippy 无警告 | `cargo clippy --all-targets -- -D warnings` | Finished (无输出) | ✓ PASS |
| test_progress_bar_template | `cargo test --lib cli::run::tests::test_progress_bar_template` | ok | ✓ PASS |
| test_progress_bar_disabled | `cargo test --lib cli::run::tests::test_progress_bar_disabled` | ok | ✓ PASS |
| test_error_log_written | `cargo test --lib cli::run::tests::test_error_log_written` | ok | ✓ PASS |
| test_hint_output | `cargo test --lib cli::run::tests::test_hint_output` | ok | ✓ PASS |
| test_run_summary | `cargo test --lib cli::run::tests::test_run_summary` | ok | ✓ PASS |
| test_classify_error_kind | `cargo test --lib error::tests::test_classify_error_kind` | ok | ✓ PASS |
| test_truncate_to_120_chars | `cargo test --lib error::tests::test_truncate_to_120_chars` | ok | ✓ PASS |
| test_error_stats_by_type_merge | `cargo test --lib error::tests::test_error_stats_by_type_merge` | ok | ✓ PASS |
| test_error_stats_merge_propagates_filtered_and_records | `cargo test --lib error::tests::test_error_stats_merge_propagates_filtered_and_records` | ok | ✓ PASS |

### Probe Execution

Step 7c SKIPPED — 本 phase 无 probe-*.sh 文件。

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|------------|------------|-------------|--------|---------|
| PROG-01 | 67-01-PLAN.md | 多文件进度条 [当前/总数] 文件计数器 | ✓ SATISFIED | `ProgressBar::new(total_files as u64)` + `[{pos}/{len}]` template |
| PROG-02 | 67-01-PLAN.md | 进度条显示 records/sec 和 ETA | ✓ SATISFIED | tick_progress 计算 rec_per_s；template 含 `eta {eta}` |
| PROG-03 | 67-03-PLAN.md | 摘要新增过滤率与错误类型分布 | ✓ SATISFIED | `errors by type: encoding/field_missing/parse_failed` + `filtered: N records` |
| DIAG-01 | 67-02-PLAN.md | error log 包含行号和原始内容前 120 字符 | ✓ SATISFIED | `[ERROR] line {}: {}  reason: {}` 格式；truncate_to_120_chars |
| DIAG-02 | 67-02-PLAN.md | 摘要按错误类型分组统计 | ✓ SATISFIED | ErrorKind + by_type HashMap；print_run_summary 输出各类型计数 |
| DIAG-03 | 67-03-PLAN.md | 常见错误触发具体 hint | ✓ SATISFIED | encoding_error/field_missing 两个 hint eprintln! |

**注意：** REQUIREMENTS.md 中 PROG-01 和 PROG-02 状态仍标为 `[ ]` Pending，而实际代码已完全实现。这是文档状态不一致（WARNING 级别，不影响代码功能）。

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| - | - | 无 TBD/FIXME/XXX debt marker | - | 无 |

扫描范围：`src/cli/run/mod.rs`、`src/cli/run/processor.rs`、`src/error.rs`、`src/config/mod.rs`，全部清洁。

### Human Verification Required

#### 1. 非 TTY 环境进度条不输出 ANSI 序列

**Test:** `cargo run -- run -c config.toml 2>&1 | cat` 在管道（非 TTY）环境运行
**Expected:** 无 ANSI 控制码出现在 stderr 输出，进度条安静退化
**Why human:** indicatif 的 `ProgressDrawTarget` 非 TTY 行为需实际终端环境观察

#### 2. 多文件 [N/M] 计数器实时递进

**Test:** 准备 3 个以上 .log 文件，运行 `cargo run -- run -c config.toml`，观察进度条
**Expected:** 进度条依次显示 `[1/3]`、`[2/3]`、`[3/3]`，与文件处理顺序吻合
**Why human:** TTY 动态渲染效果无法通过 grep/test 捕获

#### 3. records/sec 实时更新

**Test:** 对大文件（>10k 条）运行，观察进度条 message 部分
**Expected:** message 含 `Xk rec/s` 或 `X rec/s` 样式字符串，随时间变化
**Why human:** 动态速率只在实际运行时可见

#### 4. encoding_error hint 触发

**Test:** 构造含 UTF-8 replacement character（`�`）的日志文件并运行
**Expected:** stderr 摘要含 `hint: 多行 encoding_error — 建议检查文件编码是否为 GBK/GB18030`
**Why human:** eprintln! 写 stderr，测试仅验证不 panic，不捕获 stderr 文本内容

#### 5. field_missing hint 触发

**Test:** 构造以 `(EP[` 开头但格式不完整的日志行并运行
**Expected:** stderr 摘要含 `hint: 多行 field_missing — 建议确认日志格式与 DM SQL log 格式一致`
**Why human:** 同上

### Gaps Summary

无阻塞 gap。所有代码层 must-have 均已 VERIFIED：
- 进度条三要素（[N/M]、records/sec、ETA）全部实现且测试通过
- error log 行号 + 截断 + 分类 + 写出全部实现且 `test_error_log_written` 端到端测试通过
- 摘要 by_type 分布 + 过滤率 + hint 全部实现

剩余 5 项均为 **需人工验证的 UI/交互行为**（进度条视觉效果、hint 的 stderr 输出），无 BLOCKER。

**REQUIREMENTS.md 文档不一致（WARNING）：** PROG-01 和 PROG-02 在 REQUIREMENTS.md 中仍标为 `[ ]` Pending，与实际代码不一致。代码已完整实现这两个需求。建议在 Phase 67 归档时同步更新 REQUIREMENTS.md 的状态标记。

---

_Verified: 2026-06-05T12:00:00Z_
_Verifier: Claude (gsd-verifier)_
