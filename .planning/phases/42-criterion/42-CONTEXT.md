# Phase 42: Criterion 基准测试基础设施 - Context

**Gathered:** 2026-05-24
**Status:** Ready for planning

<domain>
## Phase Boundary

在现有 `bench_csv.rs`、`bench_sqlite.rs`、`bench_filters.rs` 基础上，新增 `bench_parser.rs` 覆盖 parser 原始解析速度场景，使 benchmark 套件完整覆盖四大场景。确保 `cargo bench` 独立运行（不依赖外部文件），所有 benchmark group 包含 throughput 指标和 baseline 标注。

</domain>

<decisions>
## Implementation Decisions

### Benchmark 结构
- **D-01:** 新增 `benches/bench_parser.rs` 文件，专门测试 `dm-database-parser-sqllog` 的原始解析速度（仅解析，不含导出）。
- **D-02:** 同步在 `Cargo.toml` 中添加 `[[bench]] name = "bench_parser" harness = false`。
- **D-03:** 现有 `bench_filters.rs` 的 `no_pipeline` 场景测量的是 parse+CSV export，不算"parser 原始解析速度"，两者并存无冲突。

### 数据策略
- **D-04:** bench_parser.rs 使用合成数据（synthetic log），不依赖外部文件或环境变量，与现有 bench 风格一致。
- **D-05:** 合成数据格式与 `bench_csv.rs` / `bench_filters.rs` 中 `synthetic_log()` 函数保持一致（约 170 bytes/record 的达梦格式）。

### 指标要求
- **D-06:** 每个 benchmark group 必须包含 `Throughput` 设置（`criterion::Throughput::Elements(N)`），输出 records/sec 指标。
- **D-07:** baseline 标注：使用 `benches/baselines/` 目录（已有目录），通过 `CRITERION_HOME=benches/baselines` 管理。

### 四大场景覆盖确认
- CSV 导出吞吐量 → `bench_csv.rs`（已有）
- SQLite 导出吞吐量 → `bench_sqlite.rs`（已有）
- Filter 启用/禁用吞吐量 → `bench_filters.rs`（已有，`no_pipeline` vs filter 场景）
- Parser 原始解析速度 → `bench_parser.rs`（**新增**）

### Claude's Discretion
- bench_parser.rs 内部 benchmark group 命名（如 `parser_throughput` / `raw_parse`）
- 是否同时覆盖不同 record count（如 1K、10K、100K）的规模测试

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 现有 Benchmark 代码（必读，保持风格一致）
- `benches/bench_csv.rs` — CSV benchmark 模式，synthetic_log 函数，Config 构建方式
- `benches/bench_filters.rs` — filter benchmark 模式，7 个场景对比
- `benches/bench_sqlite.rs` — SQLite benchmark 模式
- `benches/BENCHMARKS.md` — baseline 管理说明，`CRITERION_HOME` 用法

### 配置
- `Cargo.toml` — `[[bench]]` 声明位置，`criterion` dev-dependency（已含 html_reports）

### Requirements
- `.planning/REQUIREMENTS.md` §BENCH-01

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `synthetic_log(record_count: usize) -> String` — bench_csv.rs 和 bench_filters.rs 中均有实现，bench_parser.rs 可直接复制或提取为共享 helper（Claude 判断）
- `criterion_group!` / `criterion_main!` 宏 — 现有 bench 文件均已使用，直接参照

### Established Patterns
- `BenchmarkId`、`Throughput::Elements`、`Criterion` 配置方式：参考 bench_csv.rs
- Config 构建：通过 TOML 字符串 + `toml::from_str` 解析，写入 tempfile 目录

### Integration Points
- `dm_database_sqllog2db::cli::run::handle_run` — 现有 bench 的入口（bench_parser.rs 不用这个，直接调用 parser 库）
- `dm_database_parser_sqllog::LogParserBuilder` — bench_parser.rs 的测试目标

</code_context>

<specifics>
## Specific Ideas

- bench_parser.rs 只测 `LogParserBuilder::new(path).build()?.iter().count()` 或类似的纯解析路径
- 合成数据写入 tempfile，parser 从文件读取（保持与真实场景一致）

</specifics>

<deferred>
## Deferred Ideas

- 真实文件（sqllogs/ 目录）的 benchmark 场景 → 已在 bench_csv/sqlite 中作为可选 skip 场景，不在本 Phase 扩展
- GitHub Actions CI 集成 benchmark → Phase 45

</deferred>

---

*Phase: 42-Criterion 基准测试基础设施*
*Context gathered: 2026-05-24*
