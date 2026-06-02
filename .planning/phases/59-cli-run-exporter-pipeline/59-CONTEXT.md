# Phase 59: cli/run 与 exporter/pipeline 结构整理 - Context

**Gathered:** 2026-06-03
**Status:** Ready for planning

<domain>
## Phase Boundary

识别并拆分 `src/cli/run/` 下所有超过 40 行的函数（`handle_run` 除外），消除 `parallel.rs` 与 `sqlite_parallel.rs` 之间的记录收集重复代码，使每个函数体不超过 40 行且职责单一。不修改公开 API，不引入新功能。

</domain>

<decisions>
## Implementation Decisions

### processor.rs — process_log_file 拆分

- **D-01:** 提取 `normalize_and_export` — 处理 passes=true 路径中的 normalize + export + 错误处理（fatal/non-fatal 分支）。pass 判断留在调用方（主循环内）。
- **D-02:** 提取 `setup_progress_bar` — 处理 `process_log_file` 开头的进度条初始化（file_name 拼接 + pb.set_message + pb.set_position）。
- **D-03:** 提取 `log_file_result` — 处理结尾的 warn 错误数 + info 耗时 + pb.set_message 完成状态。
- **D-04:** 主函数保留：`params_buffer.clear`、`include_pm`、`build_parser`、主循环骨架（pass/needs_processing 判断 + dispatch）。目标 <40 行。

### parallel.rs — process_csv_parallel 拆分

- **D-05:** 提取 `setup_parts_dir` — 准备临时目录（~25 行，含 fallback 到系统临时目录的逻辑）。
- **D-06:** 提取 `run_parallel_tasks` — rayon 线程池构建 + 并行 map（每文件写临时 CSV）。
- **D-07:** 提取 `collect_parallel_results` — 收集 `Vec<Result<TaskResult>>` 结果，处理首次错误、skipped 计数、stats 合并。
- **D-08:** 主函数只剩 `setup_parts_dir` + `run_parallel_tasks` + `collect_parallel_results` + `concat_csv_parts` 四个调用，约 25 行。

### 并行路径重复代码消除

- **D-09:** 新建 `src/cli/run/collector.rs`，将 `collect_log_file`（单文件 parse→filter→normalize→收集到 Vec）提升至此模块（`pub(super)`）。
- **D-10:** `sqlite_parallel.rs` 的 `collect_log_file` 移到 `collector.rs`；`parallel.rs` 的并行 map lambda 改为调用 `collector::collect_log_file`，再将 Vec 写入临时 CSV。
- **D-11:** CSV 并行路径由"流式写临时文件"改为"先收集 Vec 再写临时 CSV"，内存占用略增但逻辑统一。规划时需注意大文件场景的内存影响，必要时在注释中说明。

### run_sequential + FilterProcessor 小函数

- **D-12:** `run_sequential`（52 行）：提取 `run_file_loop` — for 循环体（逐文件调用 `process_log_file` + stats merge + fatal 检查）。主函数只剩 exporter init + `run_file_loop` + finalize + log_stats，约 20 行。
- **D-13:** `FilterProcessor::from_feature`（43 行）：提取 `build_include_groups(f)` 和 `build_exclude_groups(f)` 各 7 个 `build_or_group` 调用。主函数只剩 `base_filter` + 两次调用 + `trxid_set` + `has_meta_filters` 计算，约 20 行。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 59: cli/run 与 exporter/pipeline 结构整理" — Goal、Success Criteria（3 条）
- `.planning/REQUIREMENTS.md` §STRUCT-01、STRUCT-02

### 关键源文件（必须在实现前全量阅读）
- `src/cli/run/mod.rs` — handle_run 及其辅助函数（run_sequential、run_csv_parallel 等）
- `src/cli/run/processor.rs` — process_log_file（152 行，拆分主目标）
- `src/cli/run/parallel.rs` — process_csv_parallel（156 行，拆分主目标）+ concat_csv_parts
- `src/cli/run/sqlite_parallel.rs` — collect_log_file + process_sqlite_parallel
- `src/cli/run/filter_processor.rs` — FilterProcessor::from_feature（43 行）

### 参考模式
- `src/cli/run/prescan.rs` — 现有模块拆分范例（职责单一）
- `src/pipeline/mod.rs` — Pipeline / LogProcessor trait（process_log_file 依赖）

</canonical_refs>

<code_context>
## Existing Code Insights

### 超 40 行的目标函数
- `process_log_file` (`processor.rs`) — 152 行，主要是主循环（85 行）含 params_buffer 可变状态
- `process_csv_parallel` (`parallel.rs`) — 156 行，两个明显阶段（并行分发 + 结果收集）
- `concat_csv_parts` (`parallel.rs`) — 48 行，已接近合格，可考虑顺手拆
- `run_sequential` (`mod.rs`) — 52 行，循环体 ~25 行可提取
- `FilterProcessor::from_feature` (`filter_processor.rs`) — 43 行，14 个 build_or_group 调用

### 已存在的辅助函数（勿重复）
- `build_or_group` — filter_processor.rs 已有，用于单字段 OR 组构建
- `concat_csv_parts` — parallel.rs 已有，CSV 拼接逻辑不需重新实现
- `ExporterManager::from_csv` — exporter/mod.rs，CSV 并行路径用于每个任务

### 关键约束
- `params_buffer` 是跨记录的可变状态（ParamBuffer），不能设为 &self 方法
- `break 'outer` 在主循环中（配额 + fatal 错误），提取函数时需通过返回值传递控制流意图
- `#[allow(clippy::too_many_arguments)]` 标记在 run_csv_parallel、run_sqlite_parallel、process_sqlite_parallel，拆分后如参数仍多则保留

</code_context>

<specifics>
## Specific Ideas

- CSV 并行路径改为 collect-then-write 后，`run_parallel_tasks` 可直接调用 `collector::collect_log_file`，保持与 SQLite 路径对称。
- `collector.rs` 模块只包含 `collect_log_file`（及其 helper `process_record`，已在 sqlite_parallel.rs 中），职责边界清晰。

</specifics>

<deferred>
## Deferred Ideas

- 参数打包（引入 `ParallelRunConfig` struct 消除 too_many_arguments）— 超出本阶段范围，属于接口重构，留给后续里程碑。
- `ExporterManager::from_config` 中 CSV/SQLite 两段重复的 normalize/field_mask/ordered_indices 赋值 — 行数未超标，暂不拆分。

</deferred>

---

*Phase: 59-cli-run-exporter-pipeline*
*Context gathered: 2026-06-03*
