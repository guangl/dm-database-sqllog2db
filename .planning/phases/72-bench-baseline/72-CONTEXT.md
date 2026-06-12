# Phase 72: 基准体系完善 - Context

**Gathered:** 2026-06-08
**Status:** Ready for planning

<domain>
## Phase Boundary

为 v1.20 里程碑建立完整的性能基准体系：
1. 用 hyperfine 测量当前版本 CLI 冷启动延迟，与 Phase 9（v1.9，~3ms）历史基线比较，结果记录至 BENCHMARKS.md
2. 用 `CRITERION_HOME=benches/baselines cargo bench -- --save-baseline v1.20` 将全部 criterion benchmark 结果存档至 `benches/baselines/`，供 v1.20 后续性能优化阶段（Phase 73–74）作对比基准

本 phase 不引入性能优化，只建立基准和文档。

</domain>

<decisions>
## Implementation Decisions

### Hyperfine 冷启动测量
- **D-01:** 测量命令与 Phase 9 保持一致：`hyperfine --warmup 3 './target/release/sqllog2db --version'` 和 `hyperfine --warmup 3 './target/release/sqllog2db validate -c config.toml'`
- **D-02:** BENCHMARKS.md 新增 "Phase 72 — 基准体系完善（v1.20）" 段落，记录：测量命令、hyperfine 原始输出（详情折叠）、与 Phase 9（v1.9 ~3ms）的对比数值
- **D-03:** 若 hyperfine 未安装（CI 环境），BENCHMARKS.md 使用占位说明，不阻断 CI；hyperfine 安装方式写入文档（`brew install hyperfine`）

### Criterion v1.20 Baseline 存档
- **D-04:** 使用 `CRITERION_HOME=benches/baselines` 环境变量，与现有文档模式（Phase 4/42/44）保持一致；baseline 存档至 `benches/baselines/`，纳入 repo
- **D-05:** 运行全部 4 个 bench files：bench_csv、bench_sqlite、bench_filters、bench_parser，命令形式：
  ```bash
  CRITERION_HOME=benches/baselines cargo bench -- --save-baseline v1.20
  ```
  criterion 会为每个 bench group 在 `benches/baselines/<group>/v1.20/` 下写入 benchmark.json 等文件
- **D-06:** 更新 BENCHMARKS.md 的 "How to compare against this baseline" 段落，添加 v1.20 对比命令示例：
  ```bash
  CRITERION_HOME=benches/baselines cargo bench -- --baseline v1.20
  ```
- **D-07:** baselines/.gitignore 已排除 `**/new/` 和 `**/report/`；确认 v1.20 JSON 数据文件（benchmark.json、estimates.json 等）不在排除范围内，可以正常提交

### BENCHMARKS.md 结构
- **D-08:** 在文件末尾追加新段落，不修改历史段落（Phase 4/5/6/9/10/42/44/56 内容保持不变）
- **D-09:** 新段落包含：hyperfine 冷启动数值（含与 Phase 9 对比）、criterion v1.20 baseline 说明、存档命令

### Claude's Discretion
- hyperfine `--warmup` 次数（3 次为历史惯例，可保持）
- 是否额外测量 `sqllog2db run`（I/O bound，不稳定，可跳过）
- criterion 运行样本数（使用 criterion 默认值）

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Benchmark 基础设施
- `benches/BENCHMARKS.md` — 现有基准文档，Phase 72 在此文件末尾追加新段落
- `benches/baselines/.gitignore` — 确认哪些 baseline 文件会被提交（排除 new/ 和 report/）
- `Cargo.toml` §`[[bench]]` — 4 个已注册 bench targets（bench_csv, bench_sqlite, bench_filters, bench_parser）

### 历史参考
- `benches/BENCHMARKS.md` §Phase 9 — hyperfine 冷启动历史数值（~3ms，Phase 9 / v1.9 时代）
- `benches/BENCHMARKS.md` §"How to compare against this baseline" — CRITERION_HOME 使用模式

### 需求映射
- `.planning/REQUIREMENTS.md` BENCH-01, BENCH-02 — 本 phase 的需求定义

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `benches/bench_csv.rs`, `bench_sqlite.rs`, `bench_filters.rs`, `bench_parser.rs` — 已有 4 个 criterion bench 文件，直接运行 `--save-baseline v1.20` 即可，无需修改
- `benches/baselines/` — 已存在多个历史 baseline（v1.0, phase33, phase4 等），目录结构已建立
- `config.toml` — 项目根目录已有此文件，可直接用于 hyperfine validate 测量

### Established Patterns
- `CRITERION_HOME=benches/baselines` 环境变量模式：已在 BENCHMARKS.md "How to compare" 段落记录，被 Phase 4/42/44 使用
- hyperfine `--warmup 3` 模式：Phase 9 已建立，保持一致
- BENCHMARKS.md 追加新段落模式：Phase 4/5/6/9/10/42/44/56 均是追加，不修改历史

### Integration Points
- BENCHMARKS.md 末尾追加（Phase 72 — 基准体系完善（v1.20））
- `benches/baselines/` 目录新增 v1.20 子目录（由 criterion `--save-baseline v1.20` 自动创建）
- 无代码改动，全部为文档和数据文件

</code_context>

<specifics>
## Specific Ideas

- Phase 9（v1.9）hyperfine 基线：`--version` ~3ms，`validate` ~2.8–3.0ms — 新测量应与此对比
- 存档命令示例（来自 BENCHMARKS.md 现有文档）：
  ```bash
  CRITERION_HOME=benches/baselines cargo bench -- --save-baseline v1.20
  CRITERION_HOME=benches/baselines cargo bench -- --baseline v1.20
  ```
- 构建命令（hyperfine 测量前需先 release build）：`cargo build --release`

</specifics>

<deferred>
## Deferred Ideas

- hyperfine CI 自动化（在 bench.yml 中加入冷启动测量步骤）— 属于 CI/DevOps 范畴，可在后续 phase 单独评估
- hyperfine `--export-json` 输出存档 — 当前手动记录数值即可满足 BENCH-01，自动化存档可留待未来

</deferred>

---

*Phase: 72-bench-baseline*
*Context gathered: 2026-06-08*
