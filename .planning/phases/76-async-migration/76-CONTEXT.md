# Phase 76: 异步解析路径迁移 - Context

**Gathered:** 2026-06-11
**Status:** Ready for planning

<domain>
## Phase Boundary

将所有 `dm-database-parser-sqllog` 解析调用点从同步 API 迁移到 `AsyncLogParser`（async API），添加 tokio 运行时并迁移全部调用点，保持功能与性能不退化。

**重要：实现已提前完成。** commit `65c24fd feat: complete async migration — replace async_rt bridge with AsyncLogParser` 已完成本 phase 全部改动。本 CONTEXT.md 记录已做决策，供 planner/verifier 用于验收对齐。

本 phase 不涉及：并行路径重构（Phase 75，已完成）、内存分配优化（Phase 74，已完成）、新增 benchmark（Phase 72/73，已完成）。

</domain>

<decisions>
## Implementation Decisions

### Tokio 运行时接入策略

[auto] Q: "如何在 CLI 主入口接入 tokio 运行时？" → Selected: "#[tokio::main] + multi_thread flavor" (推荐默认)

- **D-01:** `src/main.rs` 使用 `#[tokio::main]` 宏，`main()` 和 `run()` 均为 `async fn`
  - `features = ["rt-multi-thread", "macros"]` 已写入 `Cargo.toml`
  - multi-thread flavor 支持 `block_in_place`（rayon 路径必须），single-thread 不支持
- **D-02:** 不使用 `Runtime::new().block_on()` 作为主入口（保留给 bench 测试用，见 D-07）

### Rayon/Tokio 混合路径桥接

[auto] Q: "rayon worker 线程如何驱动 async 解析器？" → Selected: "block_in_place + Handle::current().block_on()" (推荐默认)

- **D-03:** `parallel.rs` 和 `prescan.rs` 中的 rayon 路径采用：
  ```rust
  let handle = tokio::runtime::Handle::current();
  let records = tokio::task::block_in_place(|| {
      handle.block_on(AsyncLogParser::new(file).parse())
  });
  ```
  `block_in_place` 通知 tokio 运行时当前线程将阻塞，避免占用 tokio worker；`block_on` 在当前线程上同步驱动 async future
- **D-04:** 不在 rayon 任务内部创建新的 `Runtime`（会 panic：嵌套 runtime）；不使用 `futures::executor::block_on`（不集成 tokio reactor，会死锁）

### 顺序路径与 Scanner

[auto] Q: "非 rayon 路径如何处理 async？" → Selected: "native async fn + .await" (推荐默认)

- **D-05:** `sequential.rs`、`collector.rs`、`processor.rs`、`scanner.rs` 全部改为 `async fn`，`.await` AsyncLogParser 结果
  - 调用链：`orchestrator.rs → sequential.rs → processor.rs → AsyncLogParser`，全链路 async
  - `sqlite_parallel.rs` 使用 `async fn` + `.await`（SQLite 写入本身串行，无 rayon）

### 错误处理策略

[auto] Q: "async 解析路径的错误如何处理？" → Selected: "graceful warn + skip（与现有策略一致）" (推荐默认)

- **D-06:** AsyncLogParser 解析错误：`log::warn!` 记录，跳过该文件，继续处理下一文件
  - 与 Phase 36 确立的"非致命解析错误不中断处理"策略保持一致
  - 注意：`AsyncLogParser` 不追踪逐条解析错误（不写 error log），`parse_errors` 统计恒为 0（测试已更新）

### Bench 文件适配

[auto] Q: "criterion bench 如何在 sync 闭包中使用 async 解析？" → Selected: "Runtime::new().block_on() per bench" (推荐默认)

- **D-07:** bench 文件（`bench_csv.rs`、`bench_sqlite.rs`、`bench_filters.rs`、`bench_parser.rs`）在 bench setup 中用 `tokio::runtime::Runtime::new().unwrap().block_on(...)` 驱动 async 解析，criterion harness 本身保持同步不变

### 测试迁移策略

[auto] Q: "如何将现有 #[test] 迁移到 async？" → Selected: "#[tokio::test] + multi_thread flavor" (推荐默认)

- **D-08:** 需要 rayon 的测试（如 parallel 路径）使用 `#[tokio::test(flavor = "multi_thread")]`；纯 async 不涉及 rayon 的测试使用标准 `#[tokio::test]`
  - 已迁移：`tests/integration.rs`（187 行差异）、`tests/watch_incremental.rs`、`src/cli/run/tests.rs`、`src/cli/watch/tests.rs`、`src/cli/stats/tests.rs`、`src/stats/tests.rs`、`tests/jemalloc_peak.rs`

### Claude's Discretion

- bench 文件中 `Runtime::new().unwrap()` 是否改为 `expect("tokio runtime")`——planner/verifier 在 clippy 检查时决定
- 各 async fn 签名上 `#[allow(clippy::too_many_arguments)]` 是否清理——属于代码风格，不影响功能正确性

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 核心改动文件
- `src/main.rs` — `#[tokio::main]`、`async fn main()`、`async fn run()` 入口
- `src/cli/run/sequential.rs` — 顺序路径，全链路 `async fn` + `.await`
- `src/cli/run/parallel.rs` — rayon CSV 并行路径，`block_in_place + Handle.block_on`
- `src/cli/run/sqlite_parallel.rs` — SQLite 并行路径，`async fn` + `.await`
- `src/cli/run/prescan.rs` — pre-scan 路径，`block_in_place + Handle.block_on`
- `src/cli/run/collector.rs` — 单文件收集，`async fn` + `.await`
- `src/cli/run/processor.rs` — 单文件处理，`async fn` + `.await`
- `src/scanner.rs` — watch 路径扫描，`async fn scan_files` + `.await`

### 依赖配置
- `Cargo.toml` — `dm-database-parser-sqllog = { version = "2.0.4", features = ["async"] }` + `tokio = { version = "1", features = ["rt-multi-thread", "macros"] }`

### 需求与验收标准
- `.planning/ROADMAP.md` §"Phase 76: 异步解析路径迁移" — Goal、Success Criteria 1–5
- `.planning/REQUIREMENTS.md` §ASYNC-01

### 前序参考
- `.planning/phases/74-memory-alloc/74-CONTEXT.md` — 性能保底策略（MEM-01/02 优化已到位，Phase 76 无性能退化风险）
- `.planning/phases/75-parallel-shared/75-01-PLAN.md` — record_iter 共享模块，Phase 76 async 路径已整合

</canonical_refs>

<code_context>
## Existing Code Insights

### 已完成迁移的调用点
- `AsyncLogParser::new(path).parse().await` — sequential、collector、processor、scanner、sqlite_parallel 均使用此模式
- `handle.block_on(AsyncLogParser::new(file).parse())` — parallel、prescan（rayon 路径）使用此桥接模式
- 全部 `use dm_database_parser_sqllog::AsyncLogParser` 导入已到位

### 无裸 unwrap
- 所有 async 错误路径使用 `log::warn!` + 跳过，或 `?` 传播
- `unwrap_or`/`unwrap_or_else`/`unwrap_or_default` 均有兜底，不属于错误路径裸用

### 测试覆盖
- 407 lib + 87 integration + 7 watch_incremental + 1 jemalloc 共 502 个测试（来自 commit 65c24fd 说明）
- `parse_errors` 统计断言已从"应有非零"改为"恒为 0"（AsyncLogParser 行为差异）

### 性能基线
- v1.20 criterion baseline 已存档于 `benches/baselines/`（Phase 72 成果）
- 验收时用 `CRITERION_HOME=benches/baselines cargo bench -- --baseline v1.20` 对比

</code_context>

<specifics>
## Specific Ideas

- REQUIREMENTS.md ASYNC-01 原文："crate 已原生支持 async，添加 tokio 运行时并迁移调用点"——该描述已完全落地
- Success Criteria #2 中的 "~1.55M records/sec 真实文件" 是 v1.19 基线参考值，来自 CLAUDE.md Benchmark 段落
- 二进制体积增量合理性：tokio multi-thread runtime 典型增量 ~500KB，profile.release 中 `lto = "fat"` + `strip = "symbols"` 会显著压缩最终增量

</specifics>

<deferred>
## Deferred Ideas

- flamegraph CPU 热点分析（PROF-01）— Future phase，async 迁移后再 profile
- heaptrack 峰值内存 profiling（PROF-02）— Future phase
- 将 AsyncLogParser 错误细节重新暴露到 error log（AsyncLogParser 不支持，需 upstream 功能）— Future phase 或 upstream PR

</deferred>

---

*Phase: 76-async-migration*
*Context gathered: 2026-06-11*
