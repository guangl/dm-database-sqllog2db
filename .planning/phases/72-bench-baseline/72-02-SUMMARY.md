---
phase: 72-bench-baseline
plan: 02
subsystem: testing

# Dependency graph
requires:
  - "72-01: Phase 72 段落骨架与 BENCH-01 hyperfine 基线"
provides:
  - "BENCH-02: criterion v1.20 baseline JSON 已纳入 repo，覆盖 4 个 bench 文件全部合成场景"
  - "How-to-compare 段落追加 v1.20 对比命令示例（D-06）"
  - "Phase 72 段落 Criterion 小节补全 + 结论 checklist 翻转为已完成"
affects: [phase-73, phase-74, phase-75, phase-76]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "CRITERION_HOME=benches/baselines cargo bench --bench <name> -- --save-baseline v1.20（分 bench 文件调用，避免 lib 单元测试截获 --save-baseline 参数）"
    - "--baseline v1.20 对比模式：criterion 自动输出 improved/regressed/no-change 判定"

key-files:
  created:
    - benches/baselines/csv_export/1000/v1.20/ (+ 3 JSON files)
    - benches/baselines/csv_export/10000/v1.20/ (+ 3 JSON files)
    - benches/baselines/csv_export/50000/v1.20/ (+ 3 JSON files)
    - benches/baselines/csv_format_only/10000/v1.20/ (+ 3 JSON files)
    - benches/baselines/filters/exclude_active/v1.20/ (+ 3 JSON files)
    - benches/baselines/filters/exclude_passthrough/v1.20/ (+ 3 JSON files)
    - benches/baselines/filters/indicator_prescan/v1.20/ (+ 3 JSON files)
    - benches/baselines/filters/no_pipeline/v1.20/ (+ 3 JSON files)
    - benches/baselines/filters/pipeline_passthrough/v1.20/ (+ 3 JSON files)
    - benches/baselines/filters/trxid_large/v1.20/ (+ 3 JSON files)
    - benches/baselines/filters/trxid_small/v1.20/ (+ 3 JSON files)
    - benches/baselines/parser_throughput/1000/v1.20/ (+ 3 JSON files)
    - benches/baselines/parser_throughput/10000/v1.20/ (+ 3 JSON files)
    - benches/baselines/parser_throughput/50000/v1.20/ (+ 3 JSON files)
    - benches/baselines/sqlite_export/1000/v1.20/ (+ 3 JSON files)
    - benches/baselines/sqlite_export/10000/v1.20/ (+ 3 JSON files)
    - benches/baselines/sqlite_export/50000/v1.20/ (+ 3 JSON files)
    - benches/baselines/sqlite_single_row/1000/v1.20/ (+ 3 JSON files)
    - benches/baselines/sqlite_single_row/10000/v1.20/ (+ 3 JSON files)
  modified:
    - benches/BENCHMARKS.md (How-to-compare 段落追加 v1.20 命令 + Phase 72 Criterion 小节补全)

key-decisions:
  - "cargo bench -- --save-baseline v1.20 会触发 lib 单元测试并导致 'Unrecognized option: save-baseline' 错误；解决方案是分 bench 文件调用：cargo bench --bench bench_{csv,sqlite,filters,parser} -- --save-baseline v1.20"
  - "csv_export_real 与 sqlite_export_real 自动 skip（sqllogs/ 不在 repo），属于正常行为，不影响 BENCH-02 验收"

requirements-completed: [BENCH-02]

# Metrics
duration: 8min
completed: 2026-06-08
---

# Phase 72 Plan 02: Criterion v1.20 Baseline 存档（BENCH-02）

**criterion v1.20 baseline 存档完成：4 个 bench 文件（bench_csv、bench_sqlite、bench_filters、bench_parser）19 个合成场景 JSON 纳入 repo，BENCHMARKS.md How-to-compare 段落追加 v1.20 对比命令，Phase 72 Criterion 小节补全，BENCH-02 结论 checklist 翻转**

## Performance

- **Duration:** 8 min
- **Started:** 2026-06-08T10:55:56Z
- **Completed:** 2026-06-08T11:03:49Z
- **Tasks:** 2
- **Files modified:** 1 (BENCHMARKS.md)
- **Files created:** 19 directories × 4 JSON files each = 76 new files

## Accomplishments

- 分别运行 4 个 bench 文件的 `--save-baseline v1.20`，生成 19 个 v1.20 目录（19 > 7 最低要求）
- csv_export（3 sizes）、csv_format_only（1 size）、sqlite_export（3 sizes）、sqlite_single_row（2 sizes）、filters（7 场景）、parser_throughput（3 sizes）全部覆盖
- csv_export_real 与 sqlite_export_real 自动 skip（sqllogs/ 不在 repo），正常行为
- `--baseline v1.20` 冒烟测试通过：filters/no_pipeline 对比输出 "No change in performance detected"
- BENCHMARKS.md How-to-compare bash 代码块追加 v1.20 对比命令（D-06）
- Phase 72 Criterion 小节从占位单行扩写为完整内容（引言 + 存档命令块 + 对比命令块 + 结论句）
- BENCH-02 结论 checklist 从 `[ ]` 翻转为 `[x]`
- cargo clippy/test/fmt 三道质量门全部通过

## Task Commits

1. **Task 1: save criterion v1.20 baseline for all 4 bench files** - `8da9f83` (chore)
2. **Task 2: update BENCHMARKS.md with v1.20 baseline commands and Phase 72 Criterion section** - `c5c503b` (docs)

**Plan metadata:** (本 SUMMARY commit)

## Files Created/Modified

- `benches/baselines/*/v1.20/` — 19 个目录，各含 benchmark.json + estimates.json + sample.json + tukey.json（76 文件）
- `benches/BENCHMARKS.md` — How-to-compare 追加 2 行（v1.20 注释 + 命令）；Phase 72 Criterion 小节扩写约 14 行；BENCH-02 checklist 翻转 1 行

## Decisions Made

- **分 bench 文件调用 --save-baseline：** `cargo bench -- --save-baseline v1.20`（无 --bench 过滤）会在运行 bench 二进制之前先运行 lib 单元测试，而 lib 单元测试不认识 `--save-baseline` 选项，导致退出码 101 "Unrecognized option: 'save-baseline'"。解决方案是分别调用 `cargo bench --bench bench_csv -- --save-baseline v1.20` 等 4 条命令，完全等价于联合命令的效果。

## Deviations from Plan

**1. [Rule 1 - Bug] 联合 cargo bench 命令不支持 --save-baseline**

- **Found during:** Task 1，Step A
- **Issue:** `CRITERION_HOME=benches/baselines cargo bench -- --save-baseline v1.20` 退出码 101，lib 单元测试优先执行并拒绝 `--save-baseline` 选项
- **Fix:** 改为分别运行 4 个 bench 文件（`cargo bench --bench bench_csv/sqlite/filters/parser -- --save-baseline v1.20`），最终产物与计划目标完全一致
- **Files modified:** 无（仅调整执行命令，未修改任何源文件）

## Known Stubs

None - 所有 baseline JSON 数据已生成，BENCHMARKS.md 内容完整。

## Threat Flags

None - 本 plan 仅创建 benchmark 数据文件和更新文档，不引入网络端点、认证路径或信任边界变更。

## Self-Check

- [x] benches/baselines/csv_export/1000/v1.20/benchmark.json 存在
- [x] benches/baselines/sqlite_export/1000/v1.20/benchmark.json 存在
- [x] benches/baselines/filters/no_pipeline/v1.20/benchmark.json 存在
- [x] benches/baselines/parser_throughput/1000/v1.20/benchmark.json 存在
- [x] find benches/baselines -type d -name v1.20 | wc -l = 19（>= 7）
- [x] grep -c "--baseline v1.20" benches/BENCHMARKS.md = 2
- [x] grep -c "--save-baseline v1.20" benches/BENCHMARKS.md = 1
- [x] BENCH-02 checklist [x] 已翻转
- [x] BENCH-01 checklist [x] 未被破坏
- [x] Phase 9 段落标题未被修改
- [x] 提交 8da9f83 (Task 1) 存在
- [x] 提交 c5c503b (Task 2) 存在

## Self-Check: PASSED

---
*Phase: 72-bench-baseline*
*Completed: 2026-06-08*
