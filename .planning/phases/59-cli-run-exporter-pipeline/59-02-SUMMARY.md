---
phase: 59-cli-run-exporter-pipeline
plan: "02"
subsystem: cli
tags: [rust, refactor, collector, sqlite-parallel, dead-code-elimination]

requires:
  - phase: 59-01
    provides: processor.rs 函数拆分（与本计划无依赖关系，wave-1 并行执行）

provides:
  - "src/cli/run/collector.rs：pub(super) fn collect_log_file + 私有 fn process_record"
  - "sqlite_parallel.rs 通过 super::collector::collect_log_file 调用共享逻辑，本地副本移除"
  - "mod.rs 注册 mod collector 子模块（按字母序）"

affects:
  - 59-03-PLAN（parallel.rs 接入 collector，D-10 第二步）
  - 59-04-PLAN（CSV 并行路径接入，D-11）

tech-stack:
  added: []
  patterns:
    - "collector.rs 模块：单文件 parse→filter→normalize→收集 Vec 职责边界"
    - "pub(super) 可见性：模块内共享但不对外暴露"
    - "#[allow(dead_code)] 临时标注 + 立即由下一 task 消除的串行提交策略"

key-files:
  created:
    - src/cli/run/collector.rs
  modified:
    - src/cli/run/sqlite_parallel.rs
    - src/cli/run/mod.rs

key-decisions:
  - "D-09：新建 collector.rs 作为单文件记录收集的共享模块，pub(super) 可见性限定在 run 子模块内"
  - "D-10：sqlite_parallel.rs 删除本地 collect_log_file + process_record，改为调用 super::collector"
  - "Task 1 使用临时 #[allow(dead_code)] 通过 clippy，Task 2 立即移除，保证每次提交 clippy 零警告"

patterns-established:
  - "collector 模块：负责单文件 parse→filter→normalize→Vec 收集，sqlite_parallel 负责并行调度+串行写入"
  - "use 区块精简：删除不再使用的 LogParserBuilder、ParamBuffer、Path import，保持 clippy -D warnings 零容忍"

requirements-completed:
  - STRUCT-02

duration: 5min
completed: 2026-06-03
---

# Phase 59 Plan 02: collector.rs 新模块提取消除 STRUCT-02 重复代码 Summary

**从 sqlite_parallel.rs 提取 collect_log_file + process_record 到新的 collector.rs 共享模块，sqlite_parallel.rs 行数从 225 降至 130，满足 STRUCT-02 重复代码消除第一步**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-06-02T23:42:15Z
- **Completed:** 2026-06-02T23:47:00Z
- **Tasks:** 2
- **Files modified:** 3（新建 1，修改 2）

## Accomplishments

- 新建 src/cli/run/collector.rs，包含 `pub(super) fn collect_log_file`（完整单文件解析收集逻辑）与私有 `fn process_record`（PARAMS buffer 感知的记录过滤+normalize 处理）
- 将 sqlite_parallel.rs 本地的两个函数定义（~85 行）替换为一次 `super::collector::collect_log_file(...)` 调用
- 清理 sqlite_parallel.rs use 区块，移除不再引用的 `LogParserBuilder`、`ParamBuffer`、`Path`
- cargo clippy --all-targets -- -D warnings 零警告，全部 638 项测试通过

## Task Commits

每个任务均单独提交：

1. **Task 1: 新建 collector.rs 并迁移函数（D-09）** - `f8865dd` (feat)
2. **Task 2: sqlite_parallel.rs 切换到 super::collector，删除本地副本（D-10）** - `de651fc` (refactor)

**Plan 元数据提交：** (docs: complete plan)

## Files Created/Modified

- `src/cli/run/collector.rs` — 新建：pub(super) collect_log_file + 私有 process_record，完整的单文件解析→过滤→normalize→Vec 收集逻辑
- `src/cli/run/sqlite_parallel.rs` — 删除两个本地函数定义（~85 行），改为调用 super::collector::collect_log_file，清理 use 区块；行数 225 → 130
- `src/cli/run/mod.rs` — 按字母序插入 `mod collector;`（位于 filter_processor 之前）

## Decisions Made

- **串行提交策略**：Task 1 创建 collector.rs 时，sqlite_parallel.rs 尚未接入，collector 函数未被引用会触发 dead_code 警告。采用临时 `#[allow(dead_code)]` 让 Task 1 单独提交通过 clippy，Task 2 立即移除该标注并接入调用，保证每次 git commit 都满足 clippy -D warnings 零警告约束。

## Deviations from Plan

None - 计划执行完全按规格完成。Task 1 添加临时 `#[allow(dead_code)]` 属于计划中预设的"如必须先提交则加 allow"路径，非偏差。

## Issues Encountered

首次提交 Task 1 时，预提交钩子因 dead_code 警告（collector 函数未被引用）而失败。按计划在 collector.rs 加临时 `#[allow(dead_code)]` 后重新提交通过。

## Self-Check: PASSED

- src/cli/run/collector.rs: FOUND
- 59-02-SUMMARY.md: FOUND
- commit f8865dd: FOUND
- commit de651fc: FOUND
- sqlite_parallel.rs 无本地 fn collect_log_file / fn process_record: PASS
- mod.rs 含 mod collector: PASS

## Next Phase Readiness

- collector.rs 模块边界已建立，`pub(super) fn collect_log_file` 签名固定
- Plan 03（parallel.rs 拆分）可读取 collector.rs 并在 D-10 第二步接入 CSV 并行路径
- Plan 04（CSV 并行路径切换到 collect-then-write）可直接调用 collector::collect_log_file

---

*Phase: 59-cli-run-exporter-pipeline*
*Completed: 2026-06-03*
