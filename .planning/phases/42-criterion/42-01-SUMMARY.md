---
phase: 42-criterion
plan: 01
subsystem: testing
tags: [criterion, benchmark, dm-database-parser-sqllog, parser, throughput]

# Dependency graph
requires: []
provides:
  - bench_parser.rs benchmark 文件，覆盖 BENCH-01 parser 原始解析第四场景
  - benches/baselines/parser_throughput/v1.0/ baseline JSON 存档
  - BENCHMARKS.md Phase 42 段落，三规模 median time 与 throughput 数据
affects:
  - 44-hotpath（parser 性能优化对比基线）
  - 45-ci（CI benchmark 集成四套 bench 文件联动）

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "bench_parser.rs 与 bench_csv.rs 结构对称：synthetic_log 独立函数 + benchmark group 函数 ≤40 行"
    - "CRITERION_HOME=benches/baselines --save-baseline v1.0 存档约定延伸到 parser_throughput group"

key-files:
  created:
    - benches/bench_parser.rs
    - benches/baselines/parser_throughput/1000/v1.0/estimates.json
    - benches/baselines/parser_throughput/10000/v1.0/estimates.json
    - benches/baselines/parser_throughput/50000/v1.0/estimates.json
  modified:
    - Cargo.toml
    - benches/BENCHMARKS.md

key-decisions:
  - "bench_parser.rs 中每次 iter 重建 LogParserBuilder，测全链路（mmap + 解析），排除导出层开销，与 bench_csv 全链路对比有意义"
  - "不新增任何依赖，dm-database-parser-sqllog 和 criterion 均已在 Cargo.lock 锁定"
  - "group 命名使用 parser_throughput（纯小写下划线），与 benches/baselines 目录命名约定对齐"

patterns-established:
  - "bench 函数模式：synthetic_log 独立生成函数 + bench 主函数 ≤40 行 + Throughput::Elements + criterion_group!/criterion_main!"

requirements-completed:
  - BENCH-01

# Metrics
duration: 20min
completed: 2026-05-24
---

# Phase 42 Plan 01: Parser Benchmark Infrastructure Summary

**新增 bench_parser.rs 实现 parser_throughput benchmark group，覆盖 BENCH-01 第四场景，parser 原始解析速度基线约 ~1.97-1.99 M records/s**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-05-24T08:33:12Z
- **Completed:** 2026-05-24T08:53:12Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments

- 新建 `benches/bench_parser.rs`，实现 `parser_throughput` benchmark group（三规模 1000/10000/50000 records）
- Cargo.toml 新增第四个 `[[bench]]` 条目，四套 bench 文件联动 `cargo bench --no-run` 全通过
- 实测采集 parser_throughput v1.0 baseline（100 samples，Apple Silicon），JSON 存档至 `benches/baselines/parser_throughput/`
- `benches/BENCHMARKS.md` 追加 Phase 42 段落，含表格、Criterion 原始输出、结论

## Task Commits

1. **Task 1+2: 新增 bench_parser.rs 并在 Cargo.toml 注册** - `13a6f15` (feat)
2. **Task 3: 采集 baseline 并更新 BENCHMARKS.md** - `4a5a9bf` (feat)

## Files Created/Modified

- `benches/bench_parser.rs` — parser_throughput benchmark，24 行主函数，符合 ≤40 行约束
- `Cargo.toml` — 新增 `[[bench]] name = "bench_parser" harness = false` 条目
- `benches/BENCHMARKS.md` — 追加 Phase 42 段落，含三规模数据表格和 Criterion 原文
- `benches/baselines/parser_throughput/{1000,10000,50000}/v1.0/` — criterion 标准 JSON 存档

## Baseline 数值

| Records | Median time | Throughput |
|--------:|------------:|-----------:|
|   1 000 |   508.62 µs |  1.97 M/s  |
|  10 000 |   5.0294 ms |  1.99 M/s  |
|  50 000 |   25.667 ms |  1.95 M/s  |

## bench_parser.rs 代码统计

- 文件总行数：53 行（含注释和空行）
- `bench_parser_throughput` 函数体：24 行（含函数头尾共 26 行，≤42 行验收通过）
- `synthetic_log` 函数：独立拆分，与 bench_csv.rs 完全一致

## 质量门禁结果

- `cargo bench --bench bench_parser --no-run`：退出码 0，无 warning，无 error
- `cargo bench --no-run`（四套 bench 联动）：退出码 0
- `cargo clippy --all-targets -- -D warnings`：退出码 0
- `cargo fmt --check`：退出码 0
- `cargo test`：33 passed, 0 failed

## Decisions Made

- 每次 iter 重建 LogParserBuilder（测全链路 mmap + 解析），与 bench_csv 全链路数据（~4.7 M/s）对比有解释意义
- group 命名 `parser_throughput`（纯小写下划线）与 baselines 目录约定对齐
- 不新增任何包依赖（BENCH-01 依赖均已在 Cargo.lock 锁定）

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 修复两个 clippy 警告**
- **Found during:** Task 1（提交时 pre-commit hook 触发 clippy）
- **Issue 1:** `doc_markdown`：注释中 `SQLite` 未用反引号包裹
- **Issue 2:** `redundant_closure_for_method_calls`：`filter_map(|r| r.ok())` 应改为 `filter_map(std::result::Result::ok)`
- **Fix:** 在 bench_parser.rs 中修正两处，符合 CLAUDE.md `cargo clippy --all-targets -- -D warnings` 通过要求
- **Files modified:** `benches/bench_parser.rs`
- **Verification:** `cargo clippy --all-targets -- -D warnings` 退出码 0
- **Committed in:** `13a6f15`（含在 Task 1+2 提交中）

---

**Total deviations:** 1 auto-fixed（Rule 1 bug fix）
**Impact on plan:** clippy 修复为 CLAUDE.md 要求，无范围扩张。

## Issues Encountered

None - 两处 clippy 警告在第一次提交尝试时被 pre-commit hook 捕获并立即修复，整体流程顺畅。

## Next Phase Readiness

- BENCH-01 四大 benchmark 场景（CSV/SQLite/filter/parser）全部就绪
- Phase 44（热路径优化）可直接使用 `--baseline v1.0` 对比 parser_throughput 变化
- Phase 45（CI 集成）所需四套 bench 文件均已注册且独立可运行

---
*Phase: 42-criterion*
*Completed: 2026-05-24*

## Self-Check: PASSED

- [x] `benches/bench_parser.rs` 存在
- [x] `Cargo.toml` 包含 `name = "bench_parser"`
- [x] `benches/BENCHMARKS.md` 包含 `## Phase 42 — Parser baseline` 段落
- [x] `benches/baselines/parser_throughput/1000/v1.0/estimates.json` 存在
- [x] commit `13a6f15` 存在（bench_parser.rs + Cargo.toml）
- [x] commit `4a5a9bf` 存在（baseline + BENCHMARKS.md）
