---
phase: 48-logging
plan: 01
subsystem: cli
tags: [clap, verbose, quiet, progressbar, stderr, handle_run]

# Dependency graph
requires:
  - phase: 46-errors
    provides: format_error_output 与结构化错误类型基础
  - phase: 47-config
    provides: validate 命令详细输出模式
provides:
  - verbose: bool 布尔标志替代原 u8 Count 标志
  - quiet 长标志 --quiet 支持
  - handle_run 签名扩展含 verbose 参数
  - 顺序路径 verbose 逐文件 Processing 输出
  - quiet 模式 ProgressBar 与完成摘要双重抑制
  - 3 个端到端 CLI 行为回归测试
affects: [48-02, 49-glob]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - verbose bool 从 opts 直传 handle_run（不经中间层转换）
    - show_progress = !quiet && !verbose 双标志互斥控制 ProgressBar
    - run() 返回 Option<(ErrorStats, bool)> 将 quiet 携带到 main() 摘要判断

key-files:
  created: []
  modified:
    - src/cli/opts.rs
    - src/main.rs
    - src/cli/run/mod.rs
    - src/cli/run/tests.rs
    - tests/integration.rs
    - tests/jemalloc_peak.rs
    - benches/bench_csv.rs
    - benches/bench_filters.rs
    - benches/bench_sqlite.rs

key-decisions:
  - "verbose 为纯布尔标志，不再接受 -vv；语义从日志级别控制转为运行时展示控制"
  - "run() 返回 Option<(ErrorStats, bool)> 而非扩展 ErrorStats 字段，实现 quiet 信号向 main() 传递"
  - "并行路径只输出文件数汇总（Processing N files in parallel），顺序路径逐文件输出 Processing: <path>"
  - "为 --quiet 补充 long 形式（Rule 2：原仅有 -q 短标志，--quiet 长标志不可用）"

patterns-established:
  - "全局标志（verbose/quiet）通过参数直传到执行函数，不通过全局状态"
  - "ProgressBar 条件：show_progress = !quiet && !verbose"

requirements-completed: [LOG-01, LOG-02]

# Metrics
duration: 35min
completed: 2026-06-01
---

# Phase 48 Plan 01: 日志级别与运行提示 Summary

**将 -v 重新定位为布尔 --verbose 标志输出逐文件 Processing 行，--quiet 完全抑制 ProgressBar 与完成摘要，35+ 处 handle_run 调用点同步迁移至新 5 参数签名**

## Performance

- **Duration:** 35 min
- **Started:** 2026-06-01T00:47:00Z
- **Completed:** 2026-06-01T01:22:00Z
- **Tasks:** 3
- **Files modified:** 9

## Accomplishments

- verbose 从 u8 Count 改为 bool 布尔标志，移除 debug/trace 日志级别映射
- handle_run 签名扩展 verbose: bool 参数，show_progress 条件改为 `!quiet && !verbose`
- 顺序路径在每文件处理前输出 `Processing: <path>` 到 stderr，并行路径输出文件数汇总
- run() 返回类型扩展为 `Option<(ErrorStats, bool)>`，携带 quiet 信号到 main() 控制摘要输出
- 3 个端到端 CLI 测试覆盖 `-v -q` 互斥、verbose 逐文件输出、quiet 摘要抑制三条核心路径

## Task Commits

1. **Task 1: 改造 opts.rs/main.rs，重塑 -v 语义并清理 debug 映射** - `4f4e132` (feat) — 包含了所有 9 个文件的一次性提交
2. **Task 3: 端到端 CLI 行为验证** - `25b4cc2` (feat)

## Files Created/Modified

- `src/cli/opts.rs` - verbose: u8→bool，添加 long="verbose" 和 conflicts_with；quiet 补充 long="quiet"
- `src/main.rs` - init_simple_logging/apply_verbosity_to_config 参数改为 bool；run() 返回类型扩展
- `src/cli/run/mod.rs` - handle_run 添加 verbose 参数；show_progress 条件；顺序/并行路径 verbose 输出
- `src/cli/run/tests.rs` - 所有 handle_run 调用更新为 5 参数新签名
- `tests/integration.rs` - 17 处 handle_run 调用更新 + 3 个新 CLI 测试
- `tests/jemalloc_peak.rs` - handle_run 调用更新
- `benches/bench_csv.rs` - handle_run 调用更新
- `benches/bench_filters.rs` - handle_run 调用更新
- `benches/bench_sqlite.rs` - handle_run 调用更新

## Decisions Made

- verbose 语义从「调整日志级别」转为「展示运行时进度详情」，-vv 不再有效
- 并行路径输出文件数汇总而非逐文件 Processing 行（并行输出顺序不确定，逐文件输出误导性强）
- run() 返回 Option<(ErrorStats, bool)> 方案比扩展 ErrorStats 字段更简洁，避免语义污染

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] 为 --quiet 补充 long 标志**
- **Found during:** Task 3 (CLI 端到端测试)
- **Issue:** `quiet` 字段仅有 `short = 'q'`，无 `long = "quiet"` 声明。端到端测试使用 `--quiet` 长标志时 clap 报错 "unexpected argument"，导致测试失败。
- **Fix:** 在 `src/cli/opts.rs` 的 quiet 字段属性宏中添加 `long = "quiet"`
- **Files modified:** src/cli/opts.rs
- **Verification:** `cargo test --test integration test_cli_quiet_suppresses_summary` 通过
- **Committed in:** 25b4cc2 (Task 3 commit)

---

**Total deviations:** 1 auto-fixed (Rule 2 - missing critical)
**Impact on plan:** --quiet 长标志缺失导致用户无法通过完整标志名使用该功能，属于正确性缺陷。修复后行为符合计划预期。

## Issues Encountered

pre-commit hook 输出超出 bash 工具响应限制（~36KB），显示 exit code 1，但实际提交均成功（通过 `git log --oneline` 验证）。Task 1 与 Task 2 因编译耦合，合并于同一次提交中包含所有 9 个文件的修改。

## Next Phase Readiness

- Phase 48 Plan 02 可继续实现 LOG-03（运行摘要差异化：默认 vs verbose 的摘要内容区别）
- verbose/quiet 信号路径已全面打通，Plan 02 可在此基础上扩展摘要格式
- handle_run 新签名稳定，后续无需再迁移调用点

## Self-Check: PASSED

- SUMMARY.md 存在于 .planning/phases/48-logging/48-01-SUMMARY.md
- Task 1+2 commit 4f4e132 存在
- Task 3 commit 25b4cc2 存在
- src/cli/opts.rs: `pub(crate) verbose: bool` 已确认
- src/cli/run/mod.rs: `let show_progress = !quiet && !verbose;` 已确认
- tests/integration.rs: 3 个新 CLI 测试函数已确认

---
*Phase: 48-logging*
*Completed: 2026-06-01*
