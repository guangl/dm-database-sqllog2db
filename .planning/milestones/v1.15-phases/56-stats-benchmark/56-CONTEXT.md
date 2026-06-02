# Phase 56: stats 模块清理与 benchmark 稳定化 - Context

**Gathered:** 2026-06-02
**Status:** Ready for planning

<domain>
## Phase Boundary

确认 stats 模块代码整洁（无遗留占位符、函数合规），抽取公共文件扫描模块供 run 和 stats 共用（含 parse error 处理一致性），并补充 benchmark CI 文档说明。

**不包括**：e2e 测试扩展（Phase 57）、cli/run 函数拆分（Phase 58）

</domain>

<decisions>
## Implementation Decisions

### 公共文件扫描模块抽取

- **D-01:** 新建独立模块（`src/scanner.rs` 或类似命名），将文件扫描逻辑（包含 parse error 写入 error log、错误计数）抽取为公共函数，`run` 和 `stats` 共用
- **D-02:** 当前 `src/stats/mod.rs` 的 `scan_files_into_accumulator` 函数中的 `log::warn!` 处理改为走公共模块的 error log 路径，与 `run` 命令对齐
- **D-03:** `run` 命令的文件扫描部分同步重构为调用公共模块（保持行为不变，仅提取）

### Benchmark 文档

- **D-04:** `benches/BENCHMARKS.md` 新增一节，说明如何从 GitHub Actions artifacts 下载 `bench-results-*.json` 文件，以及如何手动对比历史数据

### 已确认满足的成功标准（Phase 55 覆盖）

以下项目经代码审查确认已满足，planner 只需验证，无需改动：
- `src/cli/stats/mod.rs` 中无任何 `warn!` 调用 ✓
- `src/stats/output.rs` 所有函数体 ≤40 行 ✓
- `scripts/collect_bench_results.sh` 存在且可执行（`.rwxr-xr-x`）✓
- `.github/workflows/bench.yml` 已配置 `continue-on-error: true` ✓

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求定义
- `.planning/ROADMAP.md` §"Phase 56: stats 模块清理与 benchmark 稳定化" — 成功标准（3 条）、requirements 映射
- `.planning/REQUIREMENTS.md` §CLEAN-01 — "stats 模块删除遗留 warn! 占位符，stats/output.rs 所有函数不超过 40 行"
- `.planning/REQUIREMENTS.md` §BENCH-01 — "确认 scripts/collect_bench_results.sh 存在，bench.yml 以信息性方式运行"

### 待重构的代码（必读）
- `src/stats/mod.rs` — `scan_files_into_accumulator` 函数（第 38 行起，约 31 行），当前 `log::warn!` 的 parse error 处理是重构对象
- `src/cli/run/mod.rs` — run 命令的文件扫描和 parse error 处理模式，是公共模块的参考实现

### 无需改动的代码（验证对象）
- `src/cli/stats/mod.rs` — 已确认无 `warn!` 占位符
- `src/stats/output.rs` — 已确认所有函数 ≤40 行

### Benchmark 相关
- `benches/BENCHMARKS.md` — 待新增 CI artifact 使用说明节
- `.github/workflows/bench.yml` — CI benchmark workflow（`continue-on-error: true` 已配置）
- `scripts/collect_bench_results.sh` — CI 结果收集脚本（输出 `bench-results-${SHA}.json`）

</canonical_refs>

<code_context>
## Existing Code Insights

### 重构起点
- `src/stats/mod.rs:scan_files_into_accumulator`（约 31 行）— 当前 stats 文件扫描实现，直接使用 `dm_database_parser_sqllog::LogParserBuilder`，parse error 仅 `log::warn!`，无 error log 写入
- `src/cli/run/mod.rs` — run 命令的同等逻辑，完整包含 error log 写入和 `ErrorStats` 计数，是公共模块的行为参考

### 模式参照
- 公共扫描函数签名应能接受一个 callback/accumulator，将记录逐条传出，让调用方（run 或 stats）各自处理记录
- error log 写入通过 `cfg.error` 配置获取路径，与 run 命令的现有模式保持一致

### 集成点
- 新模块（`src/scanner.rs`）需要被 `src/lib.rs` 或 `src/main.rs` 的模块树引入
- `src/stats/mod.rs` 的 `scan_files_into_accumulator` 改为调用新模块
- `src/cli/run/mod.rs` 的文件扫描部分改为调用新模块（行为不变）

</code_context>

<specifics>
## Specific Ideas

- 用户明确要求：新建独立模块，而不是扩展现有 `src/parser.rs`
- parse error 的目标行为：对齐 run 命令，写入 `[error] file` 配置的错误日志文件，不写入 error log 时仍需通过某种方式可观测（保留 warn! 或 info!）
- BENCHMARKS.md 新节应说明：artifact 名称格式（`bench-results-{sha8}.json`）、从 GitHub Actions UI 或 gh CLI 下载方式、JSON 结构说明、手动对比历史数据的方法

</specifics>

<deferred>
## Deferred Ideas

- benchmark CI 门控（自动回归检测）→ 未来 milestone，需要稳定基线
- parse error 影响退出码（退出码 1）→ 用户未要求，保持现状
- crates.io 自动发布 → 未来单独配置（需 `CARGO_REGISTRY_TOKEN` secret）

</deferred>

---

*Phase: 56-stats 模块清理与 benchmark 稳定化*
*Context gathered: 2026-06-02*
