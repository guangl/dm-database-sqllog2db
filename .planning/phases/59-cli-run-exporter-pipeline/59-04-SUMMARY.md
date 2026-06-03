---
phase: 59-cli-run-exporter-pipeline
plan: "04"
subsystem: cli
tags: [rust, rayon, csv, parallel, refactor]

# Dependency graph
requires:
  - phase: 59-02
    provides: collector.rs 模块 pub(super) fn collect_log_file（CSV 并行路径的共享解析入口）
provides:
  - process_csv_parallel 函数体降至 38 行（STRUCT-01 满足）
  - setup_parts_dir / run_parallel_tasks / write_records_to_csv / collect_parallel_results / finalize_concat 五个私有辅助函数
  - CSV 并行路径通过 collector::collect_log_file 与 SQLite 并行路径对称（STRUCT-02 满足）
  - TaskResult 类型别名提升至模块级
affects:
  - phase-59 STRUCT-01 STRUCT-02 完成
  - 后续里程碑如需调整并行策略（流式写入 vs collect Vec）可参考 run_parallel_tasks doc comment

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "collect-then-write 模式：CSV 并行路径先 collect Vec 再写临时 CSV，与 SQLite 并行路径对称"
    - "四阶段骨架：setup_parts_dir + run_parallel_tasks + collect_parallel_results + finalize_concat"

key-files:
  created: []
  modified:
    - src/cli/run/parallel.rs

key-decisions:
  - "finalize_concat 作为第五个辅助函数提取：将拼接+清理逻辑分离，使 process_csv_parallel 主体降至 38 行（满足 <=40 行约束）"
  - "D-11 collect-then-write 内存 trade-off 在 run_parallel_tasks doc comment 中标注，后续 ParallelRunConfig 重构时参考"
  - "Task 1 中间提交使用 #[allow(dead_code)] 通过 pre-commit clippy hook，Task 2 移除"

patterns-established:
  - "辅助函数提取模式：先 Task 1 提取辅助函数（临时 dead_code 允许），再 Task 2 切换调用并移除 dead_code"

requirements-completed:
  - STRUCT-01
  - STRUCT-02

# Metrics
duration: 30min
completed: 2026-06-03
---

# Phase 59 Plan 04: process_csv_parallel 拆分与 collector 接入 Summary

**process_csv_parallel 从 156 行拆分为 38 行骨架 + 五个 <=40 行辅助函数，CSV 并行路径通过 collector::collect_log_file 与 SQLite 路径对称（STRUCT-01/STRUCT-02 全满足）**

## Performance

- **Duration:** 30 min
- **Started:** 2026-06-03T00:00:00Z
- **Completed:** 2026-06-03T00:30:00Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- `process_csv_parallel` 函数体降至 38 行，由四个语义清晰的辅助调用组成（STRUCT-01 满足）
- CSV 并行 lambda 改为调用 `collector::collect_log_file` 收集 Vec 再写临时 CSV，与 SQLite 并行路径对称（STRUCT-02 满足）
- 五个新私有辅助函数均 <=40 行：`setup_parts_dir`（19）、`write_records_to_csv`（15）、`run_parallel_tasks`（39）、`collect_parallel_results`（27）、`finalize_concat`（26）
- `type TaskResult` 从函数体内提升至模块级，供所有辅助函数共享
- 全量 638 个测试通过，clippy 零警告，cargo fmt 通过

## Task Commits

每个任务单独提交：

1. **Task 1: 提取 setup_parts_dir / run_parallel_tasks / write_records_to_csv（D-05/D-06/D-11）** - `4d66150` (refactor)
2. **Task 2: 提取 collect_parallel_results + finalize_concat，改写 process_csv_parallel 骨架（D-07/D-08）** - `e32c3db` (refactor)

## Files Created/Modified

- `src/cli/run/parallel.rs` — 拆分 process_csv_parallel（156 行）为 38 行骨架 + 五个辅助函数；接入 collector::collect_log_file；删除 process_log_file 调用路径

## Decisions Made

- `finalize_concat` 作为额外第五个辅助函数提取，计划原文 Task 2 仅提到 collect_parallel_results，但 process_csv_parallel 主体仍超出 40 行，按计划 D-08 约束中"若超出则再抽出 finalize_concat"指引自动执行
- Task 1 中间提交时三个新函数尚未被 process_csv_parallel 调用，触发 clippy dead_code 错误；按计划 Step 7 说明，临时加 `#[allow(dead_code)]` 属性通过 pre-commit hook，Task 2 提交时完整移除

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] 提取 finalize_concat 辅助函数**

- **Found during:** Task 2（改写 process_csv_parallel 骨架）
- **Issue:** 按 Task 2 计划实施后，process_csv_parallel 主体仍有 49 行（含签名/文档注释），实际函数体 38 行但计划中"若超出 40 行则再抽出 finalize_concat"的条件判断指引需要执行
- **Fix:** 将拼接+清理逻辑（concat_csv_parts + remove_dir_all + 错误清理 + 返回值构建）提取为 `finalize_concat` 函数
- **Files modified:** src/cli/run/parallel.rs
- **Verification:** process_csv_parallel 函数体 38 行，finalize_concat 26 行，测试全通过
- **Committed in:** e32c3db（Task 2 提交）

---

**Total deviations:** 1 auto-fixed（按计划内置条件判断指引执行，非意外）
**Impact on plan:** 完全符合计划意图，process_csv_parallel 函数体 38 行（<=40）。

## Issues Encountered

- 1Password SSH 签名代理在两次 pre-commit hook 运行（cargo test 全套）后偶发 `failed to fill whole buffer`，第二次调用 git commit 时恢复。无需操作，正常现象。

## Next Phase Readiness

- STRUCT-01 与 STRUCT-02 在 parallel.rs 范围内全部满足
- Phase 59 所有四个计划（59-01 至 59-04）全部完成，src/cli/run/ 下所有超 40 行函数均已拆分
- collector.rs、processor.rs、parallel.rs、filter_processor.rs、mod.rs 均满足 40 行函数体约束

---
*Phase: 59-cli-run-exporter-pipeline*
*Completed: 2026-06-03*
