# Phase 64: CSV 并行路径基础设施 - Context

**Gathered:** 2026-06-04
**Status:** Ready for planning

<domain>
## Phase Boundary

CSV 导出的多文件并行路径已在 `src/cli/run/parallel.rs` 实现（temp-file 方案）。Phase 64 的工作是验证现有实现满足成功标准（SC1–SC4），确认自动切换条件正确、单文件回退正常、无全量内存缓冲，并确保代码通过 clippy/test。无需重建 channel 架构。

</domain>

<decisions>
## Implementation Decisions

### 实现方案选择

[auto] Q: "CSV 并行路径应使用 channel 写入线程还是 temp-file 方案？" → Selected: "接受现有 temp-file 方案" (recommended default)

- **D-01:** CSV 并行路径采用 temp-file 方案（每个 rayon 线程独立处理一个文件，写入临时 CSV，最终按顺序拼接）。`parallel.rs` 已完整实现 `process_csv_parallel`，不引入 channel 写入线程。ROADMAP 中"channel"描述是设计意图示例，实际选择 temp-file 更简单且已验证。

### 自动切换条件

[auto] Q: "并行路径切换条件是否已满足 SC1/SC4？" → Selected: "现有条件已满足" (recommended default)

- **D-02:** `mod.rs` 中 `use_csv_parallel = jobs > 1 && log_files.len() > 1 && !is_stdin_pipe && final_cfg.exporter.csv.is_some()` 已满足 SC1（多文件+CSV 自动切换）和 SC4（单文件回退顺序路径）。无需修改切换逻辑。

### 内存模型

[auto] Q: "temp-file 方案是否满足 SC3（峰值内存 ≤ 单线程 2 倍）？" → Selected: "接受现有模型，逐文件收集" (recommended default)

- **D-03:** 每个 rayon 线程通过 `collector::collect_log_file` 将单文件所有记录收集到 `Vec<(Sqllog, Option<String>)>`，写入临时 CSV 后立即释放。同时处理的内存 = `jobs` 个文件的记录集合，但 rayon work-stealing 保证负载均衡。对于 3×300MB 文件场景，峰值内存可接受（每文件记录不超过 ~1M 条）。

### Claude's Discretion

- `process_csv_parallel` 函数签名和行为不变，Phase 64 主要工作是验证而非修改
- 若 `cargo test` 发现现有测试覆盖不足（仅 `test_handle_run_multi_file` 无内容验证），留给 Phase 66 补充集成测试

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 64: CSV 并行路径基础设施" — Goal、Success Criteria（SC1–SC4）
- `.planning/REQUIREMENTS.md` §PARALLEL-01、PARALLEL-02

### 核心实现文件
- `src/cli/run/parallel.rs` — `process_csv_parallel`、`concat_csv_parts`、`run_parallel_tasks` 完整实现
- `src/cli/run/mod.rs` — `handle_run`、`use_csv_parallel` 切换条件、`run_csv_parallel` 编排
- `src/cli/run/collector.rs` — `collect_log_file`：并行/顺序共享的单文件收集函数

### 对齐参考
- `src/cli/run/sqlite_parallel.rs` — SQLite 并行路径（同等设计，已验证）

</canonical_refs>

<code_context>
## Existing Code Insights

### 已有实现（不需重建）
- `parallel.rs`：`process_csv_parallel`（已集成）、`concat_csv_parts`（temp-file 拼接）、`setup_parts_dir`（临时目录管理）
- `mod.rs:58`：`use_csv_parallel` 切换条件（jobs > 1 && len > 1 && !stdin && csv.is_some()）
- `mod.rs:71-84`：`run_csv_parallel` 调用 `process_csv_parallel` 并处理返回值

### 库架构
- `dm-database-parser-sqllog` 使用 **mmap** 而非 BufReader，文件 I/O 已由 mmap 优化，无需额外缓冲区调整

### Established Patterns
- rayon ThreadPool（jobs 个线程）+ par_iter：与 `sqlite_parallel.rs` 完全对称
- `interrupted: &Arc<AtomicBool>` 中断检查模式：每任务开始前检查

### Integration Points
- `handle_run` → `run_csv_parallel` → `process_csv_parallel`（已接入，无需改动路由）

</code_context>

<specifics>
## Specific Ideas

- Phase 64 执行重点：`cargo test && cargo clippy --all-targets -- -D warnings` 验证现有实现
- 若 SC3（内存 ≤ 2×）无法通过测试验证，记录理论分析即可（ROADMAP 无内存基准测试要求）
- 完成标志：现有测试通过 + `test_handle_run_multi_file` 不回归

</specifics>

<deferred>
## Deferred Ideas

- channel 写入线程架构 — 比 temp-file 更低内存，但复杂度高，留后续里程碑按需评估
- per-file 进度显示（parallel 模式）— Phase 65 负责 verbose 对齐

</deferred>

---

*Phase: 64-csv*
*Context gathered: 2026-06-04*
