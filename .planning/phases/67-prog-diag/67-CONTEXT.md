# Phase 67: 进度/摘要与诊断增强 - Context

**Gathered:** 2026-06-05
**Status:** Ready for planning

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
- **D-03:** 从 `tick_progress` 中移除 `pb.inc(1024)` 调用（避免污染文件计数位置）；改为每文件完成后在 `log_file_result` 调用 `pb.inc(1)` 推进计数器并更新 ETA。
- **D-04:** records/sec 在 `tick_progress` 中计算并嵌入消息：传入 `file_start: Instant`，计算 `records_in_file as f64 / file_start.elapsed().as_secs_f64()`，通过 `pb.set_message()` 更新（格式：`"{filename} | {rec_per_s:.0}k rec/s"`）。
- **D-05:** 非 TTY 降级：indicatif `ProgressBar` 在非 TTY 环境下自动隐藏（`ProgressDrawTarget::hidden()`），无需额外判断。
- **D-06:** `make_progress_bar(show_progress: bool, total_files: usize)` — 当 `!show_progress` 时返回 `None`（现有行为不变）。

### Error Log 写入 (DIAG-01)

[auto] Q: "DIAG-01：error log 格式和写入方式？" → Selected: "ErrorStats 收集 ParseErrorRecord，完成后批量写入 [error] file；Config 新增 error 段" (recommended default)

- **D-07:** `Config` 新增字段 `pub error: Option<ErrorLogConfig>`，`ErrorLogConfig { pub file: String }` 新建结构体（serde `Deserialize`，`#[serde(default)]`）。测试 TOML 中已有 `[error] file = ...` 配置，之前被静默忽略，加上结构体后开始生效。
- **D-08:** `ErrorStats` 新增字段 `pub parse_error_records: Vec<ParseErrorRecord>`，`ParseErrorRecord = { file_path: String, line_number: u64, raw_truncated: String, kind: ErrorKind }`。`merge()` 方法 extend 该 Vec。
- **D-09:** error log 行格式：`[ERROR] line {line_number}: {raw_truncated}  reason: {err_msg}`，其中 `raw_truncated` 为原始内容前 120 字符（UTF-8 字符安全截断，`&raw[..raw.char_indices().nth(120).map_or(raw.len(), |(i,_)| i)]`）。
- **D-10:** 写 error log 时机：在 `handle_run` 完成所有文件处理后，若 `cfg.error` 有值且 `stats.parse_error_records` 非空，批量写入文件（`BufWriter`，覆盖模式）。写入失败用 `log::warn!` 记录，不终止主流程。

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
- **D-16:** 导出摘要新增两行（在 `main.rs` 的 "Completed with N error(s)" 条件块内）：
  - `"  errors by type: encoding={n}, field_missing={n}, parse_failed={n}"`（仅当 total_errors > 0 时）
  - `"  filtered: {n} records ({pct:.1}% of {total_read} total)"`（仅当 filtered_out > 0 时）

### Claude's Discretion

- records/sec 格式：千位分隔，保留整数。例 `1234 rec/s`，超过 10k 显示为 `12k rec/s`。
- 并行路径 (`run_csv_parallel` / `run_sqlite_parallel`) 不显示进度条（现有行为），PROG-01/02 只影响顺序路径。
- `parse_error_records` Vec 上限：若 `parse_errors > 10000`，截止收集（避免极端情况 OOM），并在 error log 末尾写 `"[truncated at 10000 records]"`。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 67: 进度/摘要与诊断增强" — Goal、Success Criteria（SC1–SC5）
- `.planning/REQUIREMENTS.md` §PROG-01、PROG-02、PROG-03、DIAG-01、DIAG-02、DIAG-03

### 核心实现文件
- `src/cli/run/mod.rs` — `make_progress_bar()`（line ~204）、`run_sequential()`（line ~295）、`handle_run()` 摘要输出（line ~390）
- `src/cli/run/processor.rs` — `process_log_file()`、`tick_progress()`（line ~155）、`setup_progress_bar()`（line ~105）、`log_file_result()`（line ~123）
- `src/error.rs` — `ErrorStats` struct（需扩展 `by_type`、`filtered_out`、`parse_error_records`）、`ErrorKind` 枚举（新增）
- `src/config/mod.rs` — `Config` struct（需新增 `error: Option<ErrorLogConfig>`）
- `src/scanner.rs` — `scan_files()` 的 parse error 处理路径（需传递 raw + line_number）

### 外部依赖
- `~/.cargo/registry/src/.../dm-database-parser-sqllog-2.0.2/src/error.rs` — `ParseError::InvalidFormat { raw: String, line_number: u64 }` — 提供行号和原始内容，是 DIAG-01 的数据来源
- `indicatif` crate — `ProgressBar::new(len)` 支持 `{pos}/{len}` 和 `{eta}`；非 TTY 自动隐藏

### 参考模式
- `.planning/phases/66-compat/66-CONTEXT.md` — Phase 66 决策（并行路径约定）
- `src/cli/run/parallel.rs` — 并行路径（ErrorStats.merge() 模式）

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `make_progress_bar(show_progress: bool)` (`src/cli/run/mod.rs:204`) — 只需改签名和内部实现，现有调用点只传 `show_progress`（需同时传 `total_files`）
- `tick_progress(pb, records_in_file, interrupted)` (`src/cli/run/processor.rs:155`) — 需新增 `file_start: Instant` 参数
- `ErrorStats.merge()` (`src/error.rs`) — 已有 merge 模式，扩展字段时遵循同一模式

### Established Patterns
- `indicatif::ProgressBar` 已在 `src/cli/run/mod.rs:20` 和 `processor.rs:6` import
- `ErrorStats` 用 `pub` 字段直接访问（无 getter），新字段遵循相同约定
- parse error 路径：`scanner.rs` 中 `Err(err) => { stats.add_parse_error(); log::warn!(...) }` — 需同时收集 raw + line_number

### Integration Points
- `scan_files()` (`src/scanner.rs`) 的 `Err(err)` 路径：`err` 是 `Error::Parser(ParserError::InvalidPath {...})` 但迭代器的 `Err` 是 `dm_database_parser_sqllog::ParseError`；需确认 parse iterator 返回类型
- 并行路径 (`process_csv_parallel` / `process_sqlite_parallel`) 也调用 scanner：若需要收集 `parse_error_records`，`ErrorStats::merge()` 已能聚合
- `normalize_and_export` (`processor.rs:45`) 的 `!passes` 路径：在此递增 `filtered_out`（访问 `file_stats: &mut ErrorStats`）

</code_context>

<specifics>
## Specific Ideas

- records/sec 在 `tick_progress` 中每 1024 条更新一次（现有节奏），不需要更高频率
- error log 文件覆盖写入（不追加），每次运行生成新文件
- 并行路径（`run_csv_parallel`/`run_sqlite_parallel`）返回的 `ErrorStats` 同样需要 `parse_error_records`，但进度条只影响顺序路径

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 67-进度/摘要与诊断增强*
*Context gathered: 2026-06-05*
