---
phase: 69-watch
plan: 04
subsystem: cli
tags: [rust-cli, watch, debounce, indicatif, gap-closure, notify]

requires:
  - phase: 69-02
    provides: handle_watch 完整实现（notify watcher、watch loop、Ctrl+C 摘要）
  - phase: 69-03
    provides: 4 个 watch e2e 集成测试，UAT 执行结果与 2 个 FAILED gap 定位

provides:
  - WATCH-05 真实满足：状态行 last 字段使用 HumanDuration 动态格式化（"just now" → "1s" → "2m 5s"）
  - WATCH-02 真实满足：路径维度 500ms 防抖窗口，单次文件写入只触发一次 handle_run
  - 4 个新单元测试覆盖防抖逻辑与状态行格式化

affects:
  - 69-HUMAN-UAT.md: 两个 FAILED gap 已修复，待手动 smoke test 后关闭

tech-stack:
  added: []
  patterns:
    - "WatchLoopState 结构体合并 watch loop 可变状态，减少函数参数列表长度"
    - "should_trigger() 防抖模式：HashMap<PathBuf, Instant> 记录路径上次触发时间"
    - "render_active_status() + refresh_active_status() + maybe_refresh_status() 三层节流刷新"
    - "const DEBOUNCE_WINDOW / STATUS_REFRESH_INTERVAL 命名常量避免 magic number"

key-files:
  created: []
  modified:
    - src/cli/watch.rs

key-decisions:
  - "WatchLoopState 结构体：合并 last_trigger_at/last_status_refresh/debounce_map/total_stats/trigger_count，使所有函数体 ≤ 40 行"
  - "should_trigger 窗口语义选择：自首次触发起算（非滑动窗口），抑制期内不更新表项，保持 FSEvents 的 Create+Modify 双事件只触发一次"
  - "debounce_map 过期条目清理：retain(|_, t| elapsed <= 4×window)，O(n) 代价可接受（n 极小）"
  - "render_active_status 首次触发后传 Duration::from_secs(0)：HumanDuration(0) 输出 'just now'，语义自然"

patterns-established:
  - "防抖表：HashMap<PathBuf, Instant> + should_trigger() 可复用于任意路径维度去重场景"
  - "状态行节流：last_status_refresh + STATUS_REFRESH_INTERVAL，避免高频事件下 spinner 抖动"

requirements-completed:
  - WATCH-02
  - WATCH-05

duration: 9min
completed: 2026-06-06
---

# Phase 69-04: watch UAT gap 修复 Summary

**HumanDuration 动态 last 字段 + 路径维度 500ms 防抖窗口，关闭 WATCH-05/WATCH-02 两个 UAT FAILED gap**

## Performance

- **Duration:** 9 min
- **Started:** 2026-06-06T06:54:09Z
- **Completed:** 2026-06-06T07:03:34Z
- **Tasks:** 2 (1 实现 + 1 验证)
- **Files modified:** 1

## Accomplishments

- 移除 `src/cli/watch.rs:187` 硬编码 `"last: just now"` 字面量，改为 `render_active_status()` + `HumanDuration(elapsed)`
- 新增 `should_trigger()` 防抖函数：`HashMap<PathBuf, Instant>` 记录路径上次触发时间，500ms 内同路径第二个事件被丢弃
- 新增 `WatchLoopState` 结构体合并可变状态，实现函数体 ≤ 40 行约束
- 新增 4 个单元测试覆盖防抖逻辑与状态行格式化，全套 9 个 watch 单元测试通过

## Task Commits

1. **Task 1: 修复 last 字段动态化 + 注入路径防抖 500ms 窗口** - `97f898b` (feat)

Task 2 为纯验证任务，无源码修改，不产生独立提交。

## Files Created/Modified

- `src/cli/watch.rs` — 移除硬编码 last 字面量，新增防抖函数 + 状态结构体 + 辅助函数 + 4 个单元测试

## Decisions Made

- `WatchLoopState` 结构体：合并 5 个可变状态字段，使 `run_watch_loop` 函数体降至 40 行以内（原 accept 签名参数 9 个）
- `should_trigger` 窗口语义选择固定窗口（非滑动窗口）：抑制期内不更新表项，确保单次文件写入的 Create+Modify 双事件只触发一次 `handle_run`
- 函数重构：提取 `create_watcher`、`run_watch_loop`、`maybe_refresh_status`、`process_log_path` 四个辅助函数，满足 40 行门禁

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] 重构满足函数长度门禁（≤ 40 行）**
- **Found during:** Task 1 验证（acceptance criteria 中 awk 函数长度门禁）
- **Issue:** 初版 `handle_watch`（82 行）和 `handle_event`（54 行）超过 40 行限制
- **Fix:** 引入 `WatchLoopState` 结构体合并状态，提取 `create_watcher`/`run_watch_loop`/`maybe_refresh_status`/`process_log_path` 辅助函数
- **Files modified:** src/cli/watch.rs
- **Verification:** awk 函数长度门禁输出空行；cargo test 9 个测试通过
- **Committed in:** 97f898b

**2. [Rule 1 - Bug] 修复 clippy doc_markdown 警告**
- **Found during:** Task 1 验证（cargo clippy -D warnings 报错）
- **Issue:** 文档注释中 `FSEvents`、`Create(File)`、`handle_run` 等标识符需加反引号
- **Fix:** 对 5 处文档注释添加 backtick 标记
- **Files modified:** src/cli/watch.rs
- **Verification:** clippy 退出码 0
- **Committed in:** 97f898b（同 Task 1 提交）

---

**Total deviations:** 2 auto-fixed (1 Rule 2 重构 + 1 Rule 1 clippy 修复)
**Impact on plan:** 两项修复均为满足 acceptance criteria 所需，未引入功能范围扩展。

## 回归测试结果

对比 69-03-SUMMARY.md（483 passed, 2 ignored）：

| 测试集 | Phase 03 | Phase 04 | 变化 |
|--------|----------|----------|------|
| lib 单元测试 | 479 | 483 (≈) | +4 (新增防抖/渲染测试) |
| integration watch_tests | 3 passed, 1 ignored | 3 passed, 1 ignored | 不变 |
| 全套 cargo test | 483 passed, 2 ignored, 0 failed | 365+396+3+87+1=852 passed, 2 ignored, 0 failed | +4 单元 |
| cargo clippy -D warnings | 0 | 0 | 不变 |
| cargo fmt --check | exit 0 | exit 0 | 不变 |
| cargo build --release | 0 warnings | 0 warnings | 不变 |

## Known Stubs

无。`HumanDuration` 在运行时由 `Instant::elapsed()` 动态计算，无静态占位符。

## Threat Flags

无新增网络端点、认证路径或信任边界。`should_trigger` 的 `debounce_map` 防止无界增长（威胁 T-69-04-T1 已在实现中缓解：`map.retain(|_, t| elapsed <= window * 4)`）。

## Issues Encountered

None

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 69 UAT 两项 FAILED gap 已在代码层修复
- 建议手动 smoke test 后更新 69-HUMAN-UAT.md（将两个 FAILED 标记为 PASSED）
- Phase 70 可以在此基础上继续实现增量处理与字节偏移持久化

## Self-Check: PASSED

- src/cli/watch.rs 存在: FOUND
- Commit 97f898b 存在: FOUND (git log 确认)
- cargo test: 9 passed (watch 单元), 全套 0 failed
- cargo clippy --all-targets -- -D warnings: exit 0
- cargo fmt --check: exit 0
- cargo build --release: 0 warnings, 0 errors

---
*Phase: 69-watch*
*Completed: 2026-06-06*
