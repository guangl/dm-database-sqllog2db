# Phase 65: 行为等价性保障 - Context

**Gathered:** 2026-06-04
**Status:** Ready for planning

<domain>
## Phase Boundary

三项工作：(1) 验证并行路径 CSV 字段格式/过滤结果与顺序路径完全等价（PARALLEL-03/04）；(2) 在 verbose 模式下为并行路径添加逐文件进度输出（PARALLEL-05）；(3) IO-01 确认：库使用 mmap，BufReader 缓冲区要求已由架构满足，无需修改。

</domain>

<decisions>
## Implementation Decisions

### BufReader 缓冲区（IO-01）

[auto] Q: "如何满足 BufReader ≥ 64KB 要求？" → Selected: "确认 mmap 已满足，记录原因，无需代码修改" (recommended default)

- **D-01:** `dm-database-parser-sqllog` 使用 `memmap2::Mmap` 内存映射文件，完全绕过 BufReader/系统缓冲。mmap 效果等价于无限缓冲区（整文件映射到地址空间，OS 按需 page-in）。IO-01 需求（减少系统调用）已由 mmap 架构满足，无需在 `collector.rs` 添加任何 BufReader 包装。在 Phase 65 PLAN 中记录此分析即可。

### 并行路径 verbose 输出（PARALLEL-05）

[auto] Q: "如何在 verbose 模式下输出每个文件的处理进度？" → Selected: "在 run_parallel_tasks 中添加 verbose eprintln" (recommended default)

- **D-02:** 当前 `run_csv_parallel`（`mod.rs`）在 verbose 时输出 "Processing N files in parallel (M jobs)"，但无逐文件输出。在 `parallel.rs` 的 `run_parallel_tasks` 内，每个 rayon 任务开始时添加：
  ```rust
  verbose.then(|| eprintln!("Processing: {}", file.display()));
  ```
  参数需通过 `process_csv_parallel` → `run_parallel_tasks` 传递 `verbose: bool`。模式对齐 `processor.rs:352` 的顺序路径实现。

- **D-03:** `--quiet` 抑制行为：`handle_run` 已通过 `show_progress`/`quiet` 控制摘要输出，并行路径不额外 eprintln，quiet 语义自然满足。

### 字段格式/过滤等价性（PARALLEL-03/04）

[auto] Q: "如何验证并行路径与顺序路径输出等价？" → Selected: "代码审查确认共享 collector，无需运行时对比" (recommended default)

- **D-04:** 并行路径调用 `collector::collect_log_file`（与顺序路径 `processor.rs` 使用同一函数），过滤 pipeline 和归一化逻辑完全共享。字段序列化通过 `write_records_to_csv` → `CsvExporter`，与顺序路径 `ExporterManager` 使用相同 writer。等价性由架构保证，Phase 65 的工作是代码审查验证而非运行时对比（运行时对比留给 Phase 66 集成测试）。

### Claude's Discretion

- `verbose` 参数传递路径：`process_csv_parallel` 需新增 `verbose: bool` 参数，或通过 `Config` 获取。推荐直接传参（与 `sqlite_parallel.rs` 保持对称）。
- 处理摘要（总行数/错误数）的并行累加：`run_stats.merge(&stats)` 已在 `mod.rs` 实现，无需修改。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 65: 行为等价性保障" — Goal、Success Criteria（SC1–SC4）
- `.planning/REQUIREMENTS.md` §PARALLEL-03、PARALLEL-04、PARALLEL-05、IO-01

### 关键文件
- `src/cli/run/parallel.rs` — `run_parallel_tasks`（需添加 verbose eprintln）、`process_csv_parallel`（需新增 verbose 参数）
- `src/cli/run/mod.rs` — `run_csv_parallel`（verbose 参数传递入口，line 230+）
- `src/cli/run/processor.rs:352` — 顺序路径 verbose eprintln 参考实现
- `src/cli/run/collector.rs` — 共享收集函数（等价性架构证明）

### 库架构证明（IO-01）
- `/Users/guang/.cargo/registry/src/.../dm-database-parser-sqllog-1.0.0/src/parser.rs` — `LogParser` 使用 `memmap2::Mmap`，无 BufReader

</canonical_refs>

<code_context>
## Existing Code Insights

### Verbose 参考实现
- `processor.rs:352`：`verbose.then(|| eprintln!("Processing: {}", log_file.display()))` — 逐文件格式
- `mod.rs:232-236`：`run_csv_parallel` 已接收 `verbose` 参数并传给内部函数

### 需要修改的调用链
1. `process_csv_parallel`（`parallel.rs`）：新增 `verbose: bool` 参数
2. `run_parallel_tasks`（`parallel.rs`）：新增 `verbose: bool`，在任务开始时 eprintln
3. `run_csv_parallel`（`mod.rs`）：透传 `verbose` 到 `process_csv_parallel`

### 共享架构（等价性保证）
- `collector::collect_log_file` — 并行/顺序共享，pipeline 和归一化一致
- `CsvExporter` — 并行路径 `write_records_to_csv` 使用同一 writer 实现

### mmap 架构（IO-01 满足）
- `LogParserBuilder` → `LogParser::from_path` → `Mmap::map` — 无 BufReader

</code_context>

<specifics>
## Specific Ideas

- verbose 输出格式与顺序路径对齐：`"Processing: {path}"` 而非自定义格式
- SC4 中"BufReader 缓冲区 ≥ 64KB 代码可审查"：在 PLAN 中添加一条"记录 mmap 满足 IO-01"任务，输出分析文档

</specifics>

<deferred>
## Deferred Ideas

- MultiProgress 多行进度条（并行模式每文件独立进度条）— 过度工程，留后续里程碑
- 运行时内存基准（`--bench` 对比并行/顺序峰值）— 可选，Phase 66 若有余力可加

</deferred>

---

*Phase: 65-parity*
*Context gathered: 2026-06-04*
