---
phase: 69-watch
plan: 01
subsystem: cli
tags: [rust-cli, notify, watch, scaffold, ErrorStats]

# Dependency graph
requires:
  - phase: 65-perf
    provides: handle_run 函数结构（processed_files/total_records 变量）

provides:
  - notify = "6" 依赖已添加至 Cargo.toml
  - ErrorStats.records_exported 字段（usize，Default=0，merge 累计）
  - handle_run 在返回前赋值 run_stats.records_exported = total_records
  - Commands::Watch { config } variant（-c/--config/-e SQLLOG2DB_CONFIG）
  - src/cli/mod.rs pub mod watch 声明
  - src/cli/watch.rs handle_watch 函数签名骨架（Plan 02 填充）
  - src/main.rs Watch dispatch arm（骨架，Plan 02 扩展）

affects: [69-02, 70]

# Tech tracking
tech-stack:
  added: [notify = "6"]
  patterns:
    - "watch 子命令骨架复用 Commands::Validate 风格（-c/--config + long_about/after_help）"
    - "ErrorStats 字段扩展遵循 Default derive 模式，merge() 逐字段累计"

key-files:
  created:
    - src/cli/watch.rs
  modified:
    - Cargo.toml
    - src/error.rs
    - src/cli/run/mod.rs
    - src/cli/opts.rs
    - src/cli/mod.rs
    - src/main.rs

key-decisions:
  - "main.rs 中同步添加 Watch dispatch arm（Rule 3 auto-fix），因 match 必须穷举；Plan 02 填充完整实现"
  - "watch.rs handle_watch 使用 #[allow(clippy::unnecessary_wraps)] 保持 Result<()> 签名以契合 Plan 02 接口"

patterns-established:
  - "新 CLI 子命令流程：opts.rs variant → mod.rs 声明 → watch.rs 骨架 → main.rs dispatch arm"

requirements-completed: [WATCH-01]

# Metrics
duration: 15min
completed: 2026-06-06
---

# Phase 69 Plan 01: Watch Mode Scaffold Summary

**notify="6" 依赖已就位、ErrorStats 新增 records_exported 累计字段、Commands::Watch variant + handle_watch 骨架编译通过，为 Plan 02 watch loop 实现提供全部契约面**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-06-06T02:32:00Z
- **Completed:** 2026-06-06T02:47:00Z
- **Tasks:** 2 (Task 0 auto-approved)
- **Files modified:** 6

## Accomplishments

- Cargo.toml 添加 `notify = "6"`，Cargo.lock 已锁定 notify 6.x 版本
- ErrorStats 新增 `records_exported: usize` 字段（Default=0），merge() 累计，handle_run 赋值
- `Commands::Watch { config }` variant 注册，`sqllog2db watch --help` 可正常输出
- `src/cli/watch.rs` 建立 `handle_watch` 函数签名骨架，供 Plan 02 直接填充函数体

## Task Commits

1. **Task 1: 新增 notify 依赖并扩展 ErrorStats.records_exported** - `0a4c6c5` (feat)
2. **Task 2: 注册 Commands::Watch variant 与 handle_watch 骨架** - `7192009` (feat)

## Files Created/Modified

- `Cargo.toml` - 添加 `notify = "6"` 依赖（位于 ctrlc 与 glob 之间）
- `src/error.rs` - ErrorStats 新增 `records_exported: usize` 字段和 merge 累计行
- `src/cli/run/mod.rs` - handle_run 尾部添加 `run_stats.records_exported = total_records`
- `src/cli/opts.rs` - Commands 枚举末尾追加 Watch variant
- `src/cli/mod.rs` - 追加 `pub mod watch;` 声明
- `src/cli/watch.rs` - 新建，含 handle_watch 函数签名骨架
- `src/main.rs` - Watch dispatch arm（Rule 3 auto-fix，编译必须）

## Decisions Made

- `main.rs` 中添加 Watch dispatch arm：计划禁止修改 main.rs，但 Rust match 必须穷举，不处理新 variant 则编译失败（Rule 3 阻塞性问题）。按 Rule 3 auto-fix，骨架实现与 watch.rs 一致，Plan 02 扩展完整逻辑。
- `handle_watch` 保持 `-> Result<()>` 返回类型并添加 `#[allow(clippy::unnecessary_wraps)]`：保持与 Plan 02 预期接口一致，避免 Plan 02 填充函数体时修改签名。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] 在 main.rs 添加 Watch dispatch arm 以通过编译**
- **Found during:** Task 2（注册 Commands::Watch variant）
- **Issue:** Rust 的 match 语句必须穷举所有 variant；新增 Watch 后不处理则编译报 `E0004: non-exhaustive patterns`
- **Fix:** 在 main.rs 的 match &cli.command 块中添加 Watch dispatch arm（调用 handle_watch 骨架）
- **Files modified:** src/main.rs
- **Verification:** `cargo build --release` 零错误零警告
- **Committed in:** 7192009（Task 2 提交内）

---

**Total deviations:** 1 auto-fixed (Rule 3 blocking)
**Impact on plan:** main.rs 修改为编译必须，不影响 Plan 02 接管。骨架实现与 watch.rs 完全对称。

## Issues Encountered

- clippy `unnecessary_wraps` 警告：handle_watch 骨架只含 `Ok(())`，clippy 提示返回值不必要。通过 `#[allow(clippy::unnecessary_wraps)]` 保留 `Result<()>` 签名（Plan 02 接口契约），clippy 通过。

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 02 可直接编写 watch loop：函数签名已锁定，notify 依赖已就位，main.rs dispatch arm 已存在
- ErrorStats.records_exported 已可用于 watch 模式的累计统计
- `sqllog2db watch --help` 已可用

---
*Phase: 69-watch*
*Completed: 2026-06-06*
