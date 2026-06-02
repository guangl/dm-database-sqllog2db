# Phase 58: cli/run 函数清理 - Context

**Gathered:** 2026-06-02
**Status:** Ready for planning

<domain>
## Phase Boundary

将 `src/cli/run/mod.rs` 中唯一的公开函数 `handle_run`（当前约 234 行，跨第 26–260 行）拆分为若干私有辅助函数，使每个函数体不超过 40 行。

**不包括**：任何行为变化、子模块修改（`sqlite_parallel.rs`、`prescan.rs`、`processor.rs`、`parallel.rs`、`filter_processor.rs`）、新依赖引入、性能改动。

</domain>

<decisions>
## Implementation Decisions

### 拆分方案

- **D-01:** 从 `handle_run` 提取以下 5 个私有辅助函数（按语义边界划分）：
  1. `resolve_input_files(cfg: &Config) -> Result<(Vec<PathBuf>, bool)>` — stdin pipe mode 检测 + 日志文件列表解析（对应原 34–60 行，约 27 行）
  2. `merge_trxid_prescan(cfg: &Config, log_files: &[PathBuf], jobs: usize, is_stdin_pipe: bool) -> Result<Option<Config>>` — 事务过滤器条件性预扫描 + config 合并（对应原 62–92 行，约 31 行）
  3. `make_progress_bar(show_progress: bool) -> Option<ProgressBar>` — 进度条创建（对应原 112–123 行，约 12 行）
  4. `run_sequential(log_files: &[PathBuf], final_cfg: &Config, pipeline: &Pipeline, do_normalize: bool, placeholder_override: Option<&str>, field_mask: FieldMask, ordered_indices: &[usize], verbose: bool, quiet: bool, show_progress: bool, pb: Option<&ProgressBar>, interrupted: &Arc<AtomicBool>) -> Result<(Vec<(PathBuf, usize)>, ErrorStats)>` — 顺序处理路径（对应原 184–229 行）
  5. `print_run_summary(quiet: bool, verbose: bool, use_parallel: bool, elapsed: f64, processed_files: &[(PathBuf, usize)], total_records: usize, skipped_files: usize, run_stats: &ErrorStats)` — 摘要输出（对应原 230–252 行，约 23 行）
- **D-02:** 提取后 `handle_run` 本体不超过 40 行：调用上述函数 + 字段配置计算（约 18 行）+ 并行路径选择 + 进度条收尾。

### 预扫描所有权处理

- **D-03:** `merge_trxid_prescan` 返回 `Option<Config>`：
  - `None` = 无需预扫描（无事务过滤器，或 stdin pipe 降级），调用方继续用原始 `cfg`
  - `Some(merged_cfg)` = 预扫描完成并合并了 trxid，调用方使用 `merged_cfg`
- **D-04:** 调用方模式：
  ```rust
  let merged = merge_trxid_prescan(cfg, &log_files, jobs, is_stdin_pipe)?;
  let final_cfg: &Config = merged.as_ref().unwrap_or(cfg);
  ```
  不使用 `Cow<Config>` 或 `owned_cfg` 局部变量，简化生命周期管理。

### 顺序处理路径行数控制

- **D-05:** `run_sequential` 提取后约 46 行。若仍超 40 行，通过以下任一方式控制：
  - 将 for 循环内的错误处理逻辑（`if file_stats.has_fatal()` + return）内联简化，或
  - 将 `exporter_manager.finalize()` 和 `log_stats()` 后置到调用方（因为调用方知道 quiet 值）
  - 目标：`run_sequential` 本体 ≤40 行，优先按可读性最佳方案决定

### 命名原则

- **D-06:** 私有函数命名反映单一职责，禁止使用 `helper`、`util`、`misc` 等无意义后缀
- **D-07:** 仅拆分 `src/cli/run/mod.rs`，不触碰子模块文件；提取出的私有函数定义在同一文件内（不新建子模块）

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求定义
- `.planning/ROADMAP.md` §"Phase 58: cli/run 函数清理" — 成功标准（3 条）、requirements 映射（CLEAN-02）
- `.planning/REQUIREMENTS.md` §CLEAN-02 — 完整需求描述

### 待修改文件（必读）
- `src/cli/run/mod.rs` — 唯一待修改文件，`handle_run` 函数（行 26–260）是唯一拆分目标

### 测试安全网（必读）
- `tests/integration.rs` — Phase 57 新增的 e2e CLI 全链路测试（run CSV/SQLite、init、stats from>to）是本次重构的安全网；重构后 `cargo test` 必须全部通过

### 相关子模块（了解即可，不修改）
- `src/cli/run/processor.rs` — `process_log_file` 函数，顺序路径中调用
- `src/cli/run/parallel.rs` — `process_csv_parallel` 函数，CSV 并行路径调用
- `src/cli/run/sqlite_parallel.rs` — `process_sqlite_parallel` 函数，SQLite 并行路径调用
- `src/cli/run/prescan.rs` — `scan_for_trxids_by_transaction_filters`，预扫描调用
- `src/cli/run/filter_processor.rs` — `build_pipeline`，管道构建

</canonical_refs>

<code_context>
## Existing Code Insights

### 待拆分函数结构（handle_run 行号分布）

| 行号 | 语义块 | 提取目标 |
|------|--------|----------|
| 34–60 | stdin pipe mode 检测 + log_files 解析 | `resolve_input_files` |
| 61 | jobs 计算（单行）| 内联保留 |
| 62–92 | 事务过滤器预扫描 + config 合并 | `merge_trxid_prescan` |
| 93–110 | 字段配置（FieldMask、ordered_indices、do_normalize、placeholder_override） | 内联保留（约 18 行） |
| 111–123 | 进度条创建 | `make_progress_bar` |
| 124–131 | 并行路径标志（use_csv_parallel 等） | 内联保留（约 8 行） |
| 132–156 | CSV 并行路径 | 内联调用 `process_csv_parallel` |
| 157–182 | SQLite 并行路径 | 内联调用 `process_sqlite_parallel` |
| 183–229 | 顺序处理路径（else 分支） | `run_sequential` |
| 230–252 | 摘要输出 | `print_run_summary` |
| 253–260 | 进度条收尾 + 中断检查 + return | 内联保留 |

### 关键类型
- `ProgressBar`、`ProgressStyle` — 来自 `indicatif`
- `ExporterManager` — `src/exporter/mod.rs`，在顺序路径中创建
- `Pipeline` — `src/pipeline/mod.rs`，`build_pipeline` 返回类型
- `FieldMask`、`FIELD_NAMES`、`OutputConfig` — `src/pipeline/mod.rs`
- `ParamBuffer` — `src/pipeline/normalizer`，顺序路径中作为 scratch buffer

### 成功条件验证方式
```bash
# 检查每个 fn 的行数（粗略方式）
cargo test          # e2e 测试全通过 = 行为无变化
cargo clippy --all-targets -- -D warnings  # 无新警告
```

</code_context>

<specifics>
## Specific Ideas

- `run_sequential` 的 for 循环内调用 `process_log_file` 有 14 个参数，导致调用本身占 ~16 行；可考虑减少缩进层级或用 `?` 链式简化，但不改 `process_log_file` 的签名（子模块不动）
- `merge_trxid_prescan` 中的 `warn!` + `eprintln!` 降级警告（stdin pipe + 事务过滤器组合）保留原始文案不变
- 成功标准 3 条：函数体 ≤40 行、命名清晰（不含 "helper"）、`cargo test` 全通过

</specifics>

<deferred>
## Deferred Ideas

- 子模块函数（`process_log_file` 等）的参数重构 → 本 phase 不触碰子模块
- `ProcessingParams` struct 封装多参数 → 超出本 phase 范围，属于更大规模的接口重构
- `src/cli/run/processor.rs` 内部函数长度检查 → 未来独立 phase

</deferred>

---

*Phase: 58-cli/run 函数清理*
*Context gathered: 2026-06-02*
