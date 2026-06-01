# Phase 52: 统计输出与 Exporter 集成 - Context

**Gathered:** 2026-06-01
**Status:** Ready for planning

<domain>
## Phase Boundary

实现 `stats` 命令的核心逻辑：一次流式扫描日志文件，同时聚合慢 SQL TOP-N（按 elapsed 降序）和高频 SQL TOP-N（按调用次数降序），并将结果通过独立输出函数写入 config.toml 指定的 CSV（两个文件）或 SQLite（两张表）。

</domain>

<decisions>
## Implementation Decisions

### 聚合数据结构
- **D-01:** **慢 SQL**：使用 `BinaryHeap<Reverse<SlowSqlEntry>>` 最小堆，大小固定为 `N`（`--top N`）。流式处理时 O(M log N) 时间，O(N) 内存。扫描完成后从堆中取出并排序输出。
- **D-02:** **高频 SQL**：使用 `HashMap<String, AggState>` 一次扫描聚合，key 为 `normalize_sql(sql_text)` 的结果，value 为 `{ call_count: u64, total_elapsed: f64, max_elapsed: f32 }`。扫描完成后排序取 TOP-N。
- **D-03:** 两种聚合**合并为单次扫描**：每条 `Sqllog` 记录同时更新慢 SQL 堆和高频 HashMap，日志文件只读一遍。
- **D-04:** `SlowSqlEntry` 字段：`sql_text: String`, `elapsed: f32`（毫秒）, `timestamp: String`。对应 ROADMAP 输出字段。
- **D-05:** `AggState` 及高频 SQL 输出字段：`normalized_sql`, `call_count`, `avg_elapsed`（毫秒，= total_elapsed / call_count）, `max_elapsed`（毫秒）。

### 输出集成
- **D-06:** **不复用现有 `Exporter` trait**。`Exporter` 设计用于逐条导出 `Sqllog` 记录，与统计结果结构完全不同。在 `src/stats/output.rs`（或 `src/stats/mod.rs`）中实现独立输出函数：
  - `write_csv_stats(slow: &[SlowSqlRow], frequent: &[FrequentSqlRow], csv_dir: &Path) -> Result<()>`
  - `write_sqlite_stats(slow: &[SlowSqlRow], frequent: &[FrequentSqlRow], db_url: &str) -> Result<()>`

### CSV 输出命名
- **D-07:** 输出文件名**硬编码**为 `slow_sql.csv` 和 `frequent_sql.csv`，放在 config.toml `[csv] file` 路径的**同目录**下。例如 `[csv] file = "output/data.csv"` → 输出到 `output/slow_sql.csv` 和 `output/frequent_sql.csv`。
- **D-08:** 如果配置的 CSV 目录不存在，调用 `ensure_parent_dir` 创建（已有此工具函数在 `src/exporter/mod.rs`）。

### SQLite 输出表名
- **D-09:** SQLite 输出表名**硬编码**为 `slow_sql` 和 `frequent_sql`，写入 config.toml `[sqlite] database_url` 指定的数据库文件。
- **D-10:** SQLite 写入使用 `CREATE TABLE IF NOT EXISTS`（允许重复运行）或 `CREATE TABLE`（每次 stats 前 DROP 旧表）——**推荐 DROP + CREATE**，确保输出是最新一次 stats 的结果，不累积历史数据。

### 边界处理
- **D-11:** 日志记录不足 N 条时按实际数量输出（不补零）。
- **D-12:** `elapsed` 为 0 或负数的记录纳入慢 SQL 统计（不过滤），高频 SQL 也纳入（有助于发现零耗时的高频查询）。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 52: 统计输出与 Exporter 集成" — Goal、Success Criteria（5 条）
- `.planning/REQUIREMENTS.md` §STATS-03、STATS-04、STATS-05

### 上游阶段（必须在 Phase 50/51 完成后才能实现本阶段）
- `.planning/phases/50-sql/50-CONTEXT.md` — normalize_sql 的模块位置和实现方式
- `.planning/phases/51-stats-cli/51-CONTEXT.md` — stats 子命令参数和编排接口

### 现有 Exporter 参考（理解后不复用）
- `src/exporter/mod.rs` — Exporter trait、ExporterManager、ensure_parent_dir 工具函数
- `src/exporter/csv/writer.rs` — CSV 字段序列化方式（参考，不复用 trait）
- `src/exporter/sqlite/` — SQLite 写入模式（参考，不复用 trait）

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/exporter/mod.rs::ensure_parent_dir(path: &Path) -> std::io::Result<()>`：复用于创建 CSV 输出目录
- `src/exporter/mod.rs::f32_ms_to_i64(ms: f32) -> i64`：复用于 elapsed 毫秒转换
- `dm_database_parser_sqllog::Sqllog`：流式处理的记录类型，`sqllog.elapsed` 字段为 f32 毫秒

### Established Patterns
- 流式处理模式：参照 `src/cli/run/processor.rs` 的 `process_log_file` 函数
- SQLite 连接和事务：参照 `src/exporter/sqlite/` 中的写入模式
- 错误处理：`crate::error::Result<()>` + `thiserror` 变体，不用 `unwrap`

### Integration Points
- `src/cli/stats/mod.rs`（Phase 51 创建）调用本阶段的 `run_stats(cfg, top_n) -> Result<()>` 或类似入口
- `src/stats/normalize.rs`（Phase 50 创建）提供 `normalize_sql` 函数

</code_context>

<specifics>
## Specific Ideas

- ROADMAP Success Criteria 中明确指定了输出字段顺序和排序方向，直接作为输出模式的验收依据
- `--top 5` 场景：BinaryHeap 大小限制为 5，HashMap 聚合后排序取前 5，确保严格不超过 N 行

</specifics>

<deferred>
## Deferred Ideas

None — 讨论始终在阶段范围内。

</deferred>

---

*Phase: 52-统计输出与 Exporter 集成*
*Context gathered: 2026-06-01*
