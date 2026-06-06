---
phase: 69-watch
verified: 2026-06-06T05:00:00Z
status: human_needed
score: 6/7 must-haves verified
overrides_applied: 0
gaps: []
human_verification:
  - test: "确认 watch 运行时状态行的 last 字段随时间更新"
    expected: "触发后状态行显示类似 '3 seconds ago' 的动态时间，而非固定字符串 'just now'"
    why_human: "当前实现 `last: just now` 在每次触发后立即显示正确（刚触发时确实是 just now），但 WATCH-05 要求实时动态时间（上次触发时间）。Plan 02 验收标准要求 HumanDuration(last_trigger_at.elapsed())，但实现使用静态字符串。只有人工运行 watch 并等待数秒后观察状态行是否更新才能验证。"
  - test: "验证 WATCH-02：新建 .log 文件在 2 秒内触发处理"
    expected: "向监听目录写入 .log 文件后，CSV 输出文件被创建，行数 > 1（含 header）"
    why_human: "test_watch_triggers_on_new_log_file 因 macOS FSEvents + cargo test stdin-pipe 问题被标记 #[ignore]，无法自动验证触发行为。需要人工运行：cargo run --release -- watch -c <config.toml>，然后向监听目录写入 .log 文件观察效果。"
---

# Phase 69: Watch 模式核心框架 Verification Report

**Phase Goal:** 用户可通过 `sqllog2db watch -c config.toml` 启动持续监听，目录内新增 .log 文件时自动触发处理，实时显示监听状态，Ctrl+C 优雅退出
**Verified:** 2026-06-06T05:00:00Z
**Status:** human_needed
**Re-verification:** No — 初次核查

## Goal Achievement

### Observable Truths

| #   | Truth                                                                                  | Status          | Evidence                                                                                                |
| --- | -------------------------------------------------------------------------------------- | --------------- | ------------------------------------------------------------------------------------------------------- |
| 1   | `sqllog2db watch -c config.toml` 启动后持续运行，`--help` 可发现（WATCH-01）         | ✓ VERIFIED      | `cargo run --release -- watch --help` 正常输出，含 "TOML configuration file path" 和使用示例          |
| 2   | 监听 Create + .log 扩展名事件触发 handle_run（WATCH-02 代码路径）                    | ✓ VERIFIED      | `src/cli/watch.rs:164` `EventKind::Create(_)` 匹配；`src/cli/watch.rs:173` `.extension()` 过滤        |
| 3   | Modify(Data(Content)) 事件同样触发处理（macOS FSEvents 修复）                         | ✓ VERIFIED      | `src/cli/watch.rs:165-168` 显式匹配 `EventKind::Modify(ModifyKind::Data(DataChange::Content))`        |
| 4   | ErrorStats.merge() 跨触发累计 records_exported（WATCH-05 数据来源）                  | ✓ VERIFIED      | `src/cli/watch.rs:180` `total_stats.merge(&file_stats)` + `src/error.rs:130` `self.records_exported += other.records_exported` |
| 5   | 状态行显示监听路径、触发次数、累计行数（WATCH-05 静态字段）                           | ✓ VERIFIED      | `src/cli/watch.rs:186-189` `"watching {dir} \| triggers: {trigger_count} \| processed: {} rows \| last: just now"` |
| 6   | 状态行 `last` 字段使用动态 HumanDuration（WATCH-05 完整实现、Plan 02 must_have）     | ✗ PARTIAL       | 实现使用字面量 `"just now"` 而非 `HumanDuration(last_trigger_at.elapsed())`；Plan 02 验收标准明确要求 `HumanDuration`，`grep -c 'HumanDuration' src/cli/watch.rs` 输出 0 |
| 7   | Ctrl+C 后 `pb.finish_and_clear()` + `eprintln!` 摘要 + 退出码 0（WATCH-06）         | ✓ VERIFIED      | `src/cli/watch.rs:79-82`；`test_watch_exits_when_interrupted` 通过（result.is_ok()）                  |

**Score:** 6/7 truths verified（Truth 6 为 PARTIAL）

### 关于 Truth 6 的评估

Truth 6 是 Plan 02 `must_haves.truths` 第 4 条（`ProgressBar::new_spinner() + ... tick_chars(...)` 全部满足），以及验收标准 `grep -c 'HumanDuration' src/cli/watch.rs` 输出 ≥ 1 的明确要求。

**实际情况：** 状态行在触发后立即显示 `last: just now`——从用户角度，触发刚完成时这是正确的。但 `"just now"` 是静态字符串，下次触发更新状态行前不会改变（即使已过去 5 分钟仍显示 `just now`）。WATCH-05 要求"实时显示上次触发时间"，意味着值需要随时间变化。

**判定：PARTIAL，列为 human_needed（不阻塞目标，但 WATCH-05 "上次触发时间" 的动态性需人工验证接受程度）。**

### Required Artifacts

| Artifact                   | Expected                       | Status      | Details                                              |
| -------------------------- | ------------------------------ | ----------- | ---------------------------------------------------- |
| `Cargo.toml`               | notify = "6" 依赖              | ✓ VERIFIED  | 第 44 行：`notify = "6"`                             |
| `src/error.rs`             | ErrorStats.records_exported 字段 | ✓ VERIFIED | 第 86 行：`pub records_exported: usize`；merge 累计在第 130 行 |
| `src/cli/run/mod.rs`       | `run_stats.records_exported = total_records` | ✓ VERIFIED | 第 133 行确认                                     |
| `src/cli/opts.rs`          | Commands::Watch variant        | ✓ VERIFIED  | 第 177 行：`Watch { ... }` variant，含 -c/--config  |
| `src/cli/mod.rs`           | `pub mod watch;` 声明          | ✓ VERIFIED  | 第 6 行                                              |
| `src/cli/watch.rs`         | handle_watch 完整实现（≥80行） | ✓ VERIFIED  | 279 行，含 RecommendedWatcher、EventKind::Create、recv_timeout、finish_and_clear |
| `src/main.rs`              | Watch dispatch arm + needs_simple_logging 排除 | ✓ VERIFIED | 第 202-224 行 Watch arm；第 135 行 `Commands::Watch { .. }` 在排除列表 |
| `tests/integration.rs`     | 4 个 watch e2e 测试            | ✓ VERIFIED  | `mod watch_tests` 含 4 个测试函数，1 个 `#[ignore]` |

### Key Link Verification

| From                       | To                              | Via                                   | Status     | Details                                                                           |
| -------------------------- | ------------------------------- | ------------------------------------- | ---------- | --------------------------------------------------------------------------------- |
| `src/main.rs` Watch arm    | `src/cli/watch.rs handle_watch` | `&interrupted` Arc<AtomicBool> 共享    | ✓ WIRED    | `main.rs:222` `cli::watch::handle_watch(&cfg, cli.quiet, cli.verbose, &interrupted)` |
| `watch.rs` watch loop      | `src/cli/run/mod.rs handle_run` | `tmp_cfg.sqllog.inputs = vec![path]`  | ✓ WIRED    | `watch.rs:178` `crate::cli::run::handle_run(&tmp_cfg, quiet, verbose, interrupted, None)` |
| `notify Event`             | `.log` 扩展名过滤               | `EventKind::Create(_)` match arm      | ✓ WIRED    | `watch.rs:164` Create 匹配；`watch.rs:173` `.extension()` 检查                   |
| `handle_run` 返回值        | `ErrorStats.records_exported` 累计 | `total_stats.merge(&file_stats)` | ✓ WIRED    | `watch.rs:180`；`error.rs:130` merge 累计                                        |
| `needs_simple_logging` 排除 | Watch 使用完整 logging stack    | match 排除列表                         | ✓ WIRED    | `main.rs:135` `Commands::Watch { .. }` 包含在排除 match 中                       |

### Data-Flow Trace (Level 4)

| Artifact             | Data Variable        | Source                     | Produces Real Data | Status      |
| -------------------- | -------------------- | -------------------------- | ------------------ | ----------- |
| `watch.rs` 状态行    | `total_stats.records_exported` | `handle_run` 返回的 `ErrorStats` | 是（handle_run 写入 `run_stats.records_exported = total_records`）| ✓ FLOWING |
| `watch.rs` 触发计数  | `trigger_count`      | 每次 `Ok(file_stats)` 时 `+= 1` | 是                  | ✓ FLOWING   |
| `watch.rs` last 字段 | 静态字面量            | 无动态来源                 | 否（固定 "just now"）| ⚠ STATIC   |

### Behavioral Spot-Checks

| Behavior                          | Command                                              | Result                                      | Status  |
| --------------------------------- | ---------------------------------------------------- | ------------------------------------------- | ------- |
| watch --help 可发现               | `cargo run --release -- watch --help`               | 输出帮助，含 "TOML configuration file path" 和示例 | ✓ PASS  |
| 编译无错误无警告                  | `cargo build --release`                             | Finished，零错误零警告                       | ✓ PASS  |
| lib 单元测试通过                  | `cargo test --lib cli::watch::`                     | 5 passed; 0 failed                          | ✓ PASS  |
| 集成测试（watch_tests）           | `cargo test --test integration watch_tests::`       | 3 passed; 0 failed; 1 ignored               | ✓ PASS  |
| clippy 零警告                     | `cargo clippy --all-targets -- -D warnings`         | exit 0                                      | ✓ PASS  |
| cargo fmt --check                 | `cargo fmt --check`                                 | exit 0                                      | ✓ PASS  |
| 全套 cargo test                   | `cargo test`                                        | 843 passed; 0 failed; 2 ignored             | ✓ PASS  |

### Probe Execution

不适用 — Phase 69 无声明式 probe 脚本。

### Requirements Coverage

| Requirement | Source Plan | Description                                          | Status            | Evidence                                                                |
| ----------- | ----------- | ---------------------------------------------------- | ----------------- | ----------------------------------------------------------------------- |
| WATCH-01    | 69-01, 69-02 | `sqllog2db watch -c config.toml` 启动并可发现        | ✓ SATISFIED       | Commands::Watch variant 存在；`--help` 输出正常；`test_watch_help_lists_subcommand` 通过 |
| WATCH-02    | 69-02, 69-03 | 新增 .log 文件自动触发处理                            | ? NEEDS HUMAN     | 代码路径正确（Create + .log 过滤 + handle_run 委托）；但 `test_watch_triggers_on_new_log_file` 被 `#[ignore]`，触发行为无自动验证 |
| WATCH-05    | 69-02, 69-03 | 实时显示监听路径、上次触发时间、累计已处理行数        | ~ PARTIAL         | 路径显示 ✓；累计行数 ✓；触发次数 ✓；"上次触发时间"使用静态 `"just now"` 而非动态 HumanDuration |
| WATCH-06    | 69-02, 69-03 | Ctrl+C 优雅退出，打印最终摘要                        | ✓ SATISFIED       | `test_watch_exits_when_interrupted` 通过；摘要格式正确（`Watch stopped. Triggers: ...`） |

**需注意：** WATCH-03、WATCH-04 明确分配至 Phase 70，不在本 Phase 核查范围内。

### Anti-Patterns Found

| File                  | Line | Pattern                                      | Severity  | Impact                                          |
| --------------------- | ---- | -------------------------------------------- | --------- | ----------------------------------------------- |
| `src/cli/watch.rs`    | 187  | `"last: just now"` 静态字面量                | ⚠ WARNING | WATCH-05 "上次触发时间"为静态值，不随时间更新   |

无 TBD/FIXME/XXX 未引用的 debt marker。

### Human Verification Required

#### 1. Watch 状态行 last 字段动态性验证

**Test:** 启动 `cargo run --release -- watch -c <config.toml>`，向监听目录写入一个 .log 文件触发处理，等待 5-10 秒，观察状态行 `last` 字段是否从 `"just now"` 变化为动态时间（如 `"5 seconds ago"`）
**Expected:** 若 WATCH-05 要求"实时显示上次触发时间"，状态行的 `last` 字段应随时间流逝动态更新（使用 `HumanDuration`）。当前实现在每次触发后重置为 `"just now"` 但之后不再更新。
**Why human:** Plan 02 的 must_have truth 和验收标准都明确要求 `HumanDuration(last_trigger_at.elapsed())`，但这是纯状态行 UI 行为，只有人工运行时才能判断是否可接受。如果产品决策认为"触发后显示 just now 足够"，可以接受；否则需要修复为动态 HumanDuration。

#### 2. WATCH-02 端到端触发验证

**Test:** 使用一个实际的 config.toml 运行 `cargo run --release -- watch -c config.toml`，向配置的 inputs 目录写入一个格式正确的 .log 文件，观察：(a) 触发是否在 2 秒内发生；(b) 状态行是否更新显示触发次数和处理行数；(c) CSV 或 SQLite 导出文件是否被创建/更新
**Expected:** 文件写入后 2 秒内状态行更新，`triggers: 1 | processed: N rows`，输出文件存在且包含数据
**Why human:** `test_watch_triggers_on_new_log_file` 已被 `#[ignore]`（原因：macOS cargo test stdin-pipe 问题导致 handle_run 阻塞），无法自动验证。此测试覆盖 WATCH-02 和 WATCH-05 最核心的行为。

### Gaps Summary

无阻塞性 gaps。Phase 69 的核心架构完整实现：notify watcher、watch loop、Ctrl+C 退出摘要均已到位。

两个 human_needed 项：
1. **WATCH-05 last 字段**：实现使用 `"just now"` 静态字符串而非 Plan 02 要求的 `HumanDuration(last_trigger_at.elapsed())`。功能基本满足（触发后立即显示），但长时运行时值不动态更新。建议：若接受现状，在 VERIFICATION.md 添加 override；若不接受，在 Phase 70 或下一个补丁中替换为 `HumanDuration`。
2. **WATCH-02 主集成测试被 ignore**：`test_watch_triggers_on_new_log_file` 已标记为 Phase 70 待解决，需人工 smoke test 确认触发行为正常。

---

_Verified: 2026-06-06T05:00:00Z_
_Verifier: Claude (gsd-verifier)_
