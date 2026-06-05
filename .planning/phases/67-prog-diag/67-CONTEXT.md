# Phase 67: 进度/摘要与诊断增强 - Context

**Gathered:** 2026-06-05
**Status:** Complete (updated post-implementation 2026-06-05)

<domain>
## Phase Boundary

为用户提供更丰富的运行时反馈与错误诊断：多文件进度条显示 `[N/M]` 文件计数器 + records/sec + ETA（PROG-01/02）；error log 每条记录携带行号与原文前 120 字符（DIAG-01）；导出摘要按错误类型分组统计 + 过滤率（PROG-03/DIAG-02）；常见错误模式触发具体 hint（DIAG-03）。

</domain>

<decisions>
## Implementation Decisions

### 进度条升级 (PROG-01/02)

[auto] Q: "如何实现文件计数器 [N/M] + records/sec + ETA？" → Selected: "ProgressBar::new(total_files) + {pos}/{len} template + records/sec in message + indicatif ETA" (recommended default)

- **D-01:** 将 `make_progress_bar()` 从 `ProgressBar::new_spinner()` 改为 `ProgressBar::new(total_files as u64)`，函数签名增加 `total_files: usize` 参数。
- **D-02:** 进度条 template 改为 `"{spinner:.cyan} [{pos}/{len}] {wide_msg} | eta {eta}"`，其中 `{pos}/{len}` 显示文件计数器，`{eta}` 由 indicatif 基于文件完成速率自动计算。
- **D-03:** 从 `tick_progress` 中移除 `pb.inc(1024)` 调用（避免污染文件计数位置）；改为每文件完成后在 `log_file_result` 调用 `pb.inc(1)` 推进计数器并更新 ETA。**注意：** `setup_progress_bar` 不能调用 `pb.set_position(0)` — 这会在每个新文件开始时重置文件计数器（CR-01 修复）。
- **D-04:** records/sec 在 `tick_progress` 中计算并嵌入消息：传入 `file_start: Instant`，计算 `records_in_file as f64 / file_start.elapsed().as_secs_f64()`，通过 `pb.set_message()` 更新（格式：`"{filename} | {rec_per_s:.0}k rec/s"` 或 `"{filename} | {rec_per_s:.0} rec/s"`）。**注意：** `records_in_file == 0` 时直接 return false，避免 `trailing_zeros()` 返回 64 导致误触发（WR-02 修复）。
- **D-05:** 非 TTY 降级：indicatif `ProgressBar` 在非 TTY 环境下自动隐藏（`ProgressDrawTarget::hidden()`），无需额外判断。
- **D-06:** `make_progress_bar(show_progress: bool, total_files: usize)` — 当 `!show_progress` 时返回 `None`（现有行为不变）。

### Error Log 写入 (DIAG-01)

[auto] Q: "DIAG-01：error log 格式和写入方式？" → Selected: "ErrorStats 收集 ParseErrorRecord，完成后批量写入 [error] file；Config 新增 error 段" (recommended default)

- **D-07:** `Config` 新增字段 `pub error: Option<ErrorLogConfig>`，`ErrorLogConfig { pub file: String }` 新建结构体（serde `Deserialize`，`#[serde(default)]`）。`Config::validate()` 必须校验 `error.file` 不能为空字符串或纯空白（WR-04 修复）。
- **D-08:** `ErrorStats` 新增字段 `pub parse_error_records: Vec<ParseErrorRecord>`，`ParseErrorRecord = { line_number: u64, raw_truncated: String, kind: ErrorKind }`。**注意：`file_path` 字段已移除**（IN-01 决策：不读取、不写入 error log，保留 dead code 无意义 → 删除）。`merge()` 方法 extend 该 Vec，但 merge 时也要遵守 10,000 上限（WR-01 修复）。
- **D-09:** error log 行格式：`[ERROR] line {line_number}: {raw_truncated}  reason: {kind_display}`，其中 `raw_truncated` 为原始内容前 120 字符（UTF-8 字符安全截断，`truncate_to_120_chars()`）。
- **D-10:** 写 error log 时机：在 `handle_run` 完成所有文件处理后，调用 `write_error_log(final_cfg, &run_stats)`（使用 `final_cfg` 而非 `cfg`，与 `handle_run` 其余代码一致，WR-03 修复）。若 `parse_error_records.len() >= 10_000`，在文件末尾写 `"[truncated; showing first 10000 of {stats.parse_errors} total parse errors]"`（IN-02 改进：包含总数）。写入失败用 `log::warn!` 记录，不终止主流程。

### ErrorKind 分类 (DIAG-02)

[auto] Q: "如何从 ParseError.raw 推断 ErrorKind？" → Selected: "启发式分类：含 FFFD replacement char → EncodingError；以 (EP[ 开头但不完整 → FieldMissing；其他 → ParseFailed" (recommended default)

- **D-11:** 新建 `ErrorKind` 枚举（`src/error.rs`）：`EncodingError / FieldMissing / ParseFailed`（derive `Debug, Clone, Copy, PartialEq, Eq, Hash`）。
- **D-12:** 分类函数 `classify_error_kind(raw: &str) -> ErrorKind`：
  1. `raw.contains('\u{FFFD}')` → `EncodingError`
  2. `raw.starts_with("(EP[")` → `FieldMissing`（DM 格式但字段不完整）
  3. 否则 → `ParseFailed`
- **D-13:** `ErrorStats` 新增 `pub by_type: HashMap<ErrorKind, u64>`；`add_parse_error_with_kind(kind)` 方法同步递增 `parse_errors` 和 `by_type[kind]`。`merge()` 更新 HashMap（entry API）。

### Hint 触发与摘要 (DIAG-03/PROG-03)

[auto] Q: "DIAG-03 hint 阈值？PROG-03 过滤率统计位置？" → Selected: "count > 0 即触发 hint；过滤率加入 ErrorStats.filtered_out + 摘要显示" (recommended default)

- **D-14:** Hint 触发：只要 `by_type[EncodingError] > 0` 即输出编码 hint；`by_type[FieldMissing] > 0` 即输出字段 hint。Hint 在最终摘要后 `eprintln!` 到 stderr（与现有 "Completed with N error(s)" 消息相邻）。
  - 编码 hint：`"  hint: 多行 encoding_error — 建议检查文件编码是否为 GBK/GB18030"`
  - 字段 hint：`"  hint: 多行 field_missing — 建议确认日志格式与 DM SQL log 格式一致"`
- **D-15:** `ErrorStats` 新增 `pub filtered_out: u64` 字段；在 `normalize_and_export` 的 `!passes` 路径中递增。`merge()` 相加。
- **D-16:** 导出摘要新增两行（在 `print_run_summary` 中）：
  - `"  errors by type: encoding={n}, field_missing={n}, parse_failed={n}"`（仅当 total_errors > 0 时）
  - `"  filtered: {n} records ({pct:.1}% of {total_read} total)"`（仅当 filtered_out > 0 时）

### Claude's Discretion

- records/sec 格式：`>= 10_000.0` 显示为 `Xk rec/s`，否则 `X rec/s`（整数）。
- 并行路径 (`run_csv_parallel` / `run_sqlite_parallel`) 不显示进度条（现有行为），PROG-01/02 只影响顺序路径。
- `parse_error_records` Vec 上限：每文件处理时若 `parse_error_records.len() < 10_000` 才追加；merge 时也守 10,000 全局上限（WR-01 修复）。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 67: 进度/摘要与诊断增强" — Goal、Success Criteria（SC1–SC5）
- `.planning/REQUIREMENTS.md` §PROG-01、PROG-02、PROG-03、DIAG-01、DIAG-02、DIAG-03

### 核心实现文件
- `src/cli/run/mod.rs` — `make_progress_bar()`（line ~204）、`run_sequential()`（line ~295）、`handle_run()` 摘要输出（line ~390）、`write_error_log()`（line ~473）
- `src/cli/run/processor.rs` — `process_log_file()`、`tick_progress()`（line ~155）、`setup_progress_bar()`（line ~105）、`log_file_result()`（line ~123）
- `src/error.rs` — `ErrorStats` struct（含 `by_type`、`filtered_out`、`parse_error_records`）、`ErrorKind` 枚举、`classify_error_kind`、`truncate_to_120_chars`
- `src/config/mod.rs` — `Config` struct（含 `error: Option<ErrorLogConfig>`）、`ErrorLogConfig`

### 外部依赖
- `indicatif` crate — `ProgressBar::new(len)` 支持 `{pos}/{len}` 和 `{eta}`；非 TTY 自动隐藏
- `dm-database-parser-sqllog` — `ParseError::InvalidFormat { raw: String, line_number: u64 }` — 行号和原始内容数据来源

### 参考模式
- `.planning/phases/66-compat/66-CONTEXT.md` — Phase 66 决策（并行路径约定）
- `src/cli/run/parallel.rs` — 并行路径（ErrorStats.merge() 模式）

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `make_progress_bar(show_progress: bool, total_files: usize)` (`src/cli/run/mod.rs:~204`) — 签名已更新为包含 total_files
- `tick_progress(pb, records_in_file, file_start, file_name, interrupted)` (`src/cli/run/processor.rs:~155`) — 签名已更新为包含 `file_start: Instant`
- `ErrorStats.merge()` (`src/error.rs`) — 已支持 by_type/filtered_out/parse_error_records 合并，含 10,000 上限

### Established Patterns
- `indicatif::ProgressBar` 已在 `src/cli/run/mod.rs` 和 `processor.rs` import
- `ErrorStats` 用 `pub` 字段直接访问（无 getter）
- parse error 路径：`process_log_file` 中 `Err(e)` 解构 `ParseError::InvalidFormat { raw, line_number }`，调用 `add_parse_error_with_kind`

### Integration Points
- `normalize_and_export` (`processor.rs:~45`) 的 `!passes` 路径：递增 `file_stats.filtered_out`
- `handle_run` 末尾：调用 `print_run_summary` 然后 `write_error_log(final_cfg, &run_stats)`
- `Config::validate()` 中校验 `error.file` 非空白

</code_context>

<specifics>
## Specific Ideas

- records/sec 在 `tick_progress` 中每 1024 条更新一次（`trailing_zeros() >= 10` 检测）；`records_in_file == 0` 时直接 return false
- error log 文件覆盖写入（不追加），每次运行生成新文件
- 并行路径（`run_csv_parallel`/`run_sqlite_parallel`）返回的 `ErrorStats` 同样包含 `parse_error_records`，通过 `merge()` 聚合

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 67-进度/摘要与诊断增强*
*Context gathered: 2026-06-05*
*Updated post-implementation: 2026-06-05 — removed file_path from ParseErrorRecord (IN-01), improved truncated footer (IN-02), confirmed final_cfg usage (WR-03)*
