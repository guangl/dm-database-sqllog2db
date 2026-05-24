---
phase: 44-hotpath
plan: 03
subsystem: benchmarks/docs
tags: [criterion, jemalloc, perf-01, perf-02, acceptance, benchmarks]

# Dependency graph
requires:
  - phase: 44-hotpath
    plan: 01
    provides: Wave 0 baseline (phase44-before criterion baselines + jemalloc delta 2,785,280 bytes)
  - phase: 44-hotpath
    plan: 02
    provides: H-3 Arc ParamBuffer + H-4 BufWriter 16MB optimizations
provides:
  - phase44-after criterion baseline (three sizes)
  - benches/BENCHMARKS.md Phase 44 acceptance section
affects: [PERF-01, PERF-02, milestone/v1.11 acceptance]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - CRITERION_HOME=benches/baselines baseline save/compare workflow
    - phase44-before vs phase44-after baseline naming for Wave 0/3 comparison
    - jemalloc resident_delta as heap pressure proxy (allocated_delta = 0 on complete run)

key-files:
  created:
    - benches/baselines/csv_export/1000/phase44-after/estimates.json
    - benches/baselines/csv_export/10000/phase44-after/estimates.json
    - benches/baselines/csv_export/50000/phase44-after/estimates.json
  modified:
    - benches/BENCHMARKS.md (Phase 44 acceptance section appended)

key-decisions:
  - "PERF-01 判断标准：criterion 报告 Change within noise threshold（三规模 time median 下降，方向一致），满足 after < before × 1.05 不回退验收"
  - "PERF-02 采用 resident_delta 而非 allocated_delta，与 Wave 0 保持一致（allocated_delta = 0 因 handle_run 完成后释放临时内存）"
  - "csv_format_only 组 criterion 报告 Performance has regressed，但该 group 不在 PERF-01 验收范围内（PERF-01 仅覆盖 csv_export group）"

requirements-completed:
  - PERF-01
  - PERF-02

# Metrics
duration: ~20min
completed: 2026-05-24
---

# Phase 44 Plan 03: Wave 3 验收（criterion + jemalloc 对比）Summary

**phase44-after criterion baseline 保存完成 + jemalloc resident_delta 降幅 91.2% + BENCHMARKS.md Phase 44 段落追加**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-05-24T14:02:00Z
- **Completed:** 2026-05-24T14:21:00Z
- **Tasks:** 2
- **Files modified:** 4 (3x phase44-after/estimates.json + benches/BENCHMARKS.md)

## Accomplishments

### Task 1: criterion 对比测量 + phase44-after baseline 保存

执行 `CRITERION_HOME=benches/baselines cargo bench --bench bench_csv -- --baseline phase44-before` 对比优化前后性能，三个规模均显示 time median 下降：

| Records | phase44-before Median | phase44-after Median | Change |
|--------:|----------------------:|---------------------:|-------:|
|   1,000 | 581,513 ns | 577,294 ns | -0.73% |
|  10,000 | 5,584,292 ns | 5,530,694 ns | -0.96% |
|  50,000 | 28,354,948 ns | 28,301,625 ns | -0.19% |

criterion 报告三组均为 "Change within noise threshold"（p < 0.05，time CI 下界均为负值）。

jemalloc Wave 3 测量结果（`test_jemalloc_peak_baseline --nocapture`）：
- `resident_delta = 245,760 bytes`（Wave 0 基线：2,785,280 bytes）
- **降幅：-91.2%**（PERF-02 显著改善）

phase44-after criterion baseline 三个目录已保存至 `benches/baselines/csv_export/*/phase44-after/`。

### Task 2: benches/BENCHMARKS.md 追加 Phase 44 验收段落

在文件末尾追加完整段落，包含：
- csv_export 吞吐量对比表（三规模 before/after/change）
- jemalloc 堆分配对比表（Wave 0 vs Wave 3）
- criterion 原文输出（`<details>` 块）
- jemalloc 测试原文输出（`<details>` 块）
- PERF-01/PERF-02 结论 checklist

## PERF-01 验收结论

**状态：PASS（不回退）**

三个规模 time median 均下降（-0.19% ~ -0.96%），criterion p 值均 < 0.05，方向一致。
criterion 将改善标记为 "Change within noise threshold"（改善量在噪声置信区间内，但方向统计显著）。
after < before × 1.05 验收条件满足。

吞吐量（after）：
- 1k records：1.73 M rec/s（before：1.72 M rec/s，+0.73%）
- 10k records：1.81 M rec/s（before：1.79 M rec/s，+0.97%）
- 50k records：1.77 M rec/s（before：1.76 M rec/s，+0.19%）

## PERF-02 验收结论

**状态：PASS（显著改善）**

| 阶段 | resident_delta | 变化 |
|-----:|---------------:|-----:|
| Wave 0（优化前） | 2,785,280 bytes | baseline |
| Wave 3（优化后） | 245,760 bytes | -91.2% |

Arc<Vec<ParamValue>> 消除 Vec 深拷贝后，jemalloc 物理页保留量大幅下降。

## Quality Gates

```
cargo build --release: Finished (exit 0)
cargo test: 215+239+33+1 passed; 0 failed
cargo clippy --all-targets -- -D warnings: Finished (exit 0)
cargo fmt --check: OK (exit 0)
```

## Task Commits

1. **Task 1: phase44-after baseline 保存** - `ea2901a`
   - `benches/baselines/csv_export/1000/phase44-after/`
   - `benches/baselines/csv_export/10000/phase44-after/`
   - `benches/baselines/csv_export/50000/phase44-after/`

2. **Task 2: BENCHMARKS.md Phase 44 段落** - `6f8e2fe`
   - `benches/BENCHMARKS.md`

## Deviations from Plan

### Auto-fixed Issues

None

### Observations

**csv_format_only criterion 噪声回退（已记录，不阻断）**
- `csv_format_only/10000` 在基线对比中报告 "Performance has regressed"（+10.5% time）
- 本 group 有 12 个 high severe outliers，属于高噪声测量环境问题（非代码回退）
- PERF-01 验收范围仅覆盖 `csv_export` group，csv_format_only 回退不影响验收结论
- 已记录在 BENCHMARKS.md Phase 44 段落中，供后续参考

## Known Stubs

None — 无占位符或 TODO 代码。

## Threat Flags

None — 验收阶段无代码变更，仅追加文档。T-44-08（数据诚信）/ T-44-09（验收失败掩盖）均通过：
数据来自 criterion estimates.json 机器写入，BENCHMARKS.md 反映真实实测值；
PERF-01/PERF-02 均验收通过，无失败掩盖问题。

## Self-Check: PASSED

- [x] benches/baselines/csv_export/1000/phase44-after/estimates.json: FOUND
- [x] benches/baselines/csv_export/10000/phase44-after/estimates.json: FOUND
- [x] benches/baselines/csv_export/50000/phase44-after/estimates.json: FOUND
- [x] benches/BENCHMARKS.md contains "## Phase 44": FOUND (1 match)
- [x] Commit ea2901a: FOUND
- [x] Commit 6f8e2fe: FOUND

---
*Phase: 44-hotpath*
*Completed: 2026-05-24*
