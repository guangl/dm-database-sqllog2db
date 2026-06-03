---
phase: 59-cli-run-exporter-pipeline
plan: "06"
subsystem: cli
tags: [rust, refactor, struct-01, gap-closure]

requires:
  - phase: 59-05
    provides: 59-01~05 完成 normalize_and_export 提取 + collector.rs 新建 + 并行路径重构

provides:
  - "normalize_and_export 函数体降至 39 行（VERIFICATION gap 1 关闭）"
  - "parallel_collect 函数体降至 33 行（VERIFICATION gap 2 关闭）"
  - "私有辅助函数 update_params_buffer_only（processor.rs）"
  - "私有辅助函数 run_parallel_parse + 类型别名 ParseResults（sqlite_parallel.rs）"

affects:
  - STRUCT-01
  - ROADMAP SC-1

tech-stack:
  added: []
  patterns:
    - "提取辅助函数模式：用辅助函数封装单一路径逻辑，降低外层函数体行数"
    - "类型别名 ParseResults 降低 rayon 并行结果复杂类型可读性"

key-files:
  created: []
  modified:
    - src/cli/run/processor.rs
    - src/cli/run/sqlite_parallel.rs

key-decisions:
  - "update_params_buffer_only 使用 let _ = 忽略 compute_normalized 返回值，避免 clippy must_use 警告"
  - "ParseResults 类型别名保留以提升可读性，不内联到签名"
  - "run_parallel_parse 返回 Result<ParseResults>，以 ? 在 parallel_collect 中传播 ThreadPool 构建失败"

patterns-established:
  - "私有辅助函数提取：对单一路径逻辑（!passes 分支、rayon 并行块）提取为小函数"

requirements-completed:
  - STRUCT-01

duration: 15min
completed: 2026-06-03
---

# Phase 59 Plan 06: Gap Closure Summary

**提取 update_params_buffer_only 与 run_parallel_parse 辅助函数，关闭 normalize_and_export（47→39 行）与 parallel_collect（50→33 行）两个 STRUCT-01 缺口**

## Performance

- **Duration:** 15 min
- **Started:** 2026-06-03T02:30:00Z
- **Completed:** 2026-06-03T02:45:00Z
- **Tasks:** 3（其中 Task 3 为纯验证任务，无代码修改）
- **Files modified:** 2

## Accomplishments

- normalize_and_export 函数体从 47 行降至 39 行（VERIFICATION gap 1 关闭）
- parallel_collect 函数体从 50 行降至 33 行（VERIFICATION gap 2 关闭）
- STRUCT-01 / ROADMAP SC-1 完全满足：src/cli/run/ 下所有函数体 ≤40 行（含已文档化豁免）
- 638 项测试全部通过，行为零变化

## Task Commits

1. **Task 1: 提取 update_params_buffer_only** - `93e6aa6` (refactor)
2. **Task 2: 提取 run_parallel_parse** - `c2f7fc9` (refactor)
3. **Task 3: 最终验证**（纯验证，无额外提交）

**Plan metadata:** [见最终提交]

## Files Created/Modified

- `src/cli/run/processor.rs` — 新增私有辅助函数 `update_params_buffer_only`，封装 `!passes` 路径的 `compute_normalized` 调用，使 `normalize_and_export` 函数体从 47 行降至 39 行
- `src/cli/run/sqlite_parallel.rs` — 新增类型别名 `ParseResults` 与私有辅助函数 `run_parallel_parse`，封装 ThreadPool 创建 + `pool.install` 并行块，使 `parallel_collect` 函数体从 50 行降至 33 行

## Decisions Made

- `update_params_buffer_only` 使用 `let _ = crate::pipeline::compute_normalized(...)` 明确忽略返回值，避免 clippy `must_use` 警告（Option<&str> 返回值）
- `ParseResults` 类型别名定义在文件作用域（私有），不导出；降低 `run_parallel_parse` 签名中复杂嵌套类型的可读性负担
- `run_parallel_parse` 以 `Result<ParseResults>` 返回，`parallel_collect` 用 `?` 直接传播 ThreadPoolBuilder 失败，保留原有 `map_err(|e| Error::Io(...))` 语义

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 修复文档注释 clippy::doc_markdown 警告**
- **Found during:** Task 1 验证阶段
- **Issue:** 新增辅助函数文档注释中 `params_buffer` 和 `compute_normalized` 未用反引号包裹，触发 `clippy::doc_markdown` 错误（`-D warnings` 下升级为 error）
- **Fix:** 将两处标识符改为 `` `params_buffer` `` 和 `` `compute_normalized` ``
- **Files modified:** `src/cli/run/processor.rs`
- **Verification:** cargo clippy --all-targets -- -D warnings 零警告
- **Committed in:** `93e6aa6`（Task 1 提交内）

**2. [Rule 3 - Blocking] cargo fmt 自动格式化 run_parallel_parse 调用**
- **Found during:** Task 2 格式检查
- **Issue:** `let results = run_parallel_parse(log_files, pipeline, jobs, do_normalize, placeholder_override, interrupted)?;` 超过行宽，cargo fmt 需要多行展开
- **Fix:** 运行 `cargo fmt` 自动格式化
- **Files modified:** `src/cli/run/sqlite_parallel.rs`
- **Verification:** cargo fmt --check 无差异
- **Committed in:** `c2f7fc9`（Task 2 提交内）

---

**Total deviations:** 2 auto-fixed (1 bug/doc-fix, 1 formatting)
**Impact on plan:** 均为小型自动修复，不影响计划范围和行为正确性。

## Final Verification Results

| 验证项 | 结果 |
|--------|------|
| cargo build | BUILD_OK，零 error，零 warning |
| cargo test | 638 passed, 0 failed（269+300+68+1） |
| cargo clippy --all-targets -- -D warnings | 零警告 |
| cargo fmt --check | 无差异 |
| normalize_and_export 函数体行数 | **39 行** ≤40（gap 1 关闭）|
| parallel_collect 函数体行数 | **33 行** ≤40（gap 2 关闭）|
| src/cli/run/ 其他函数行数 | 均 ≤40（handle_run 101 行 + concat_csv_parts 43 行 已文档化豁免）|

## Issues Encountered

- worktree 初始基于 v1.15（`d0ada30`），未包含 Phase 59 plans 01-05 的重构代码。通过 `git merge main` 将 worktree 同步到最新 HEAD（`ccfea5f`）后继续执行，属正常 worktree 初始化流程。

## Next Phase Readiness

- Phase 59 所有 VERIFICATION gap 全部关闭
- STRUCT-01 完全满足，ROADMAP SC-1 完全满足
- Phase 59 可以进入最终状态标记

## Known Stubs

None — 本计划为纯代码结构整理，无任何 stub 或占位符引入。

## Threat Flags

None — 纯私有辅助函数提取，无新网络边界、无新外部依赖、无公开 API 变更。

---

*Phase: 59-cli-run-exporter-pipeline*
*Completed: 2026-06-03*

## Self-Check: PASSED

- `src/cli/run/processor.rs` — FOUND (contains `fn update_params_buffer_only(`)
- `src/cli/run/sqlite_parallel.rs` — FOUND (contains `fn run_parallel_parse(` + `type ParseResults`)
- commit `93e6aa6` — FOUND
- commit `c2f7fc9` — FOUND
