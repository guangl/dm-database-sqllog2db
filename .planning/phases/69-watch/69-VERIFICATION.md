---
phase: 69-watch
verified: 2026-06-06T08:00:00Z
status: human_needed
score: 8/8 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 6/7
  gaps_closed:
    - "WATCH-05 状态行 last 字段静态 'just now'：已替换为 HumanDuration(elapsed) 动态格式化 + 200ms 节流刷新"
    - "WATCH-02/CR-01 单文件双重触发（无防抖）：已加入 should_trigger + DEBOUNCE_WINDOW(500ms) HashMap<PathBuf,Instant>"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "验证 WATCH-02：新建 .log 文件在 2 秒内触发处理（端到端）"
    expected: "向监听目录写入格式正确的 .log 文件后，CSV/SQLite 输出文件被创建，行数 > 1（含 header）；状态行更新为 triggers: 1 | processed: N rows"
    why_human: "test_watch_triggers_on_new_log_file 标 #[ignore]（macOS FSEvents + cargo test stdin-pipe 阻塞问题），Phase 70 用 subprocess 模式解决。ROADMAP SC2 要求 2 秒内触发，当前无法自动验证。需人工 smoke test 确认触发行为正常。"
---

# Phase 69: Watch 模式核心框架 Verification Report (Re-verification)

**Phase Goal:** 用户可通过 `sqllog2db watch -c config.toml` 启动持续监听，目录内新增 .log 文件时自动触发处理，实时显示监听状态，Ctrl+C 优雅退出
**Verified:** 2026-06-06T08:00:00Z
**Status:** human_needed
**Re-verification:** Yes — Plan 04 gap closure 后复查（前次 6/7，两个 human gaps 均已代码修复）

## Goal Achievement

### Observable Truths

| #   | Truth                                                                                                                                  | Status     | Evidence                                                                                                                                                                    |
| --- | -------------------------------------------------------------------------------------------------------------------------------------- | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `sqllog2db watch -c config.toml` 启动后持续运行，`--help` 可发现（WATCH-01）                                                           | ✓ VERIFIED | `Commands::Watch` variant 存在于 `src/cli/opts.rs:177`；`test_watch_help_lists_subcommand` 通过                                                                            |
| 2   | 监听 Create(_) + Modify(Data(Content)) + .log 扩展名事件触发 handle_run（WATCH-02 代码路径）                                             | ✓ VERIFIED | `watch.rs:244-248` Create + Modify 双路匹配；`watch.rs:254` `.extension()` 过滤；`should_trigger` 防抖（500ms 窗口）确保单次写入只触发一次                                 |
| 3   | 状态行 last 字段使用 HumanDuration 动态格式化，随时间更新（WATCH-05 Plan 02/04 must_have）                                              | ✓ VERIFIED | `watch.rs:331-335` `render_active_status` 使用 `HumanDuration(elapsed)`；`watch.rs:151-163` `maybe_refresh_status` 在 Timeout 分支每 200ms 节流刷新；`"last: just now"` 静态字面量 = 0 |
| 4   | 防抖：同一路径 500ms 窗口内仅触发一次 handle_run（WATCH-02/CR-01 gap 修复）                                                              | ✓ VERIFIED | `watch.rs:21` `DEBOUNCE_WINDOW = Duration::from_millis(500)`；`watch.rs:312-328` `should_trigger` 逻辑正确；3 个单元测试（test_should_trigger_*）通过                      |
| 5   | ErrorStats.merge() 跨触发累计 records_exported（WATCH-05 数据来源）                                                                     | ✓ VERIFIED | `watch.rs:290` `total_stats.merge(&file_stats)`；`error.rs:130` `self.records_exported += other.records_exported`                                                         |
| 6   | 状态行显示监听路径、触发次数、累计行数（WATCH-05 静态字段）                                                                              | ✓ VERIFIED | `watch.rs:331-335` `"watching {dir} \| triggers: {trigger_count} \| processed: {rows} rows \| last: {}"` 格式；`test_render_active_status_includes_human_duration` 通过  |
| 7   | Ctrl+C 后 `pb.finish_and_clear()` + `eprintln!` 摘要 + 退出码 0（WATCH-06）                                                            | ✓ VERIFIED | `watch.rs:54-60` loop 退出后调用序列；`watch.rs:374-383` `print_final_summary` 含 "Watch stopped. Triggers: ..."；`test_watch_exits_when_interrupted` 通过                |
| 8   | main.rs Watch arm：preflight + logging + ctrlc + handle_watch + needs_simple_logging 排除（WATCH-01）                                   | ✓ VERIFIED | `main.rs:135` 排除列表含 `Commands::Watch { .. }`；`main.rs:202-224` 完整 Watch arm；`cli::watch::handle_watch` 调用正确                                                  |

**Score:** 8/8 truths verified

**WATCH-02 端到端说明：** Truth 2 仅验证代码路径正确。ROADMAP SC2 要求实际触发（新增文件 → 2 秒内处理），`test_watch_triggers_on_new_log_file` 仍标 `#[ignore]`（macOS stdin-pipe 问题），需人工 smoke test 最终确认。

### Required Artifacts

| Artifact                   | Expected                                              | Status      | Details                                                                         |
| -------------------------- | ----------------------------------------------------- | ----------- | ------------------------------------------------------------------------------- |
| `Cargo.toml`               | `notify = "6"` 依赖                                   | ✓ VERIFIED  | 第 44 行：`notify = "6"`                                                        |
| `src/error.rs`             | `ErrorStats.records_exported` 字段 + merge 累计       | ✓ VERIFIED  | 第 86 行字段定义；第 130 行 merge 累计                                          |
| `src/cli/run/mod.rs`       | `run_stats.records_exported = total_records`          | ✓ VERIFIED  | 第 133 行                                                                       |
| `src/cli/opts.rs`          | `Commands::Watch` variant（-c/--config）              | ✓ VERIFIED  | 第 177 行 Watch variant                                                         |
| `src/cli/mod.rs`           | `pub mod watch;` 声明                                 | ✓ VERIFIED  | 第 6 行                                                                         |
| `src/cli/watch.rs`         | handle_watch 完整实现（HumanDuration + 防抖）         | ✓ VERIFIED  | 384 行；含 `DEBOUNCE_WINDOW`、`should_trigger`、`render_active_status`、`HumanDuration` |
| `src/main.rs`              | Watch dispatch arm + needs_simple_logging 排除        | ✓ VERIFIED  | 第 202-224 行 Watch arm；第 135 行排除                                          |
| `tests/integration.rs`     | 4 个 watch e2e 测试（含 1 个 `#[ignore]`）            | ✓ VERIFIED  | `mod watch_tests:2854` 含 4 个测试函数；3 passed, 1 ignored                    |

### Key Link Verification

| From                         | To                               | Via                                      | Status     | Details                                                                              |
| ---------------------------- | -------------------------------- | ---------------------------------------- | ---------- | ------------------------------------------------------------------------------------ |
| `src/main.rs` Watch arm      | `src/cli/watch.rs handle_watch`  | `&interrupted` Arc<AtomicBool> 共享       | ✓ WIRED    | `main.rs:222` `cli::watch::handle_watch(&cfg, cli.quiet, cli.verbose, &interrupted)` |
| `watch.rs` watch loop        | `src/cli/run/mod.rs handle_run`  | `tmp_cfg.sqllog.inputs = vec![path]`     | ✓ WIRED    | `watch.rs:288` `crate::cli::run::handle_run(&tmp_cfg, quiet, verbose, interrupted, None)` |
| `notify Event`               | `.log` 扩展名过滤 + 防抖          | `EventKind::Create(_)` + `should_trigger` | ✓ WIRED   | `watch.rs:244` Create 匹配；`watch.rs:254` extension 检查；`watch.rs:257` should_trigger |
| `WatchLoopState.last_trigger_at` | `render_active_status` HumanDuration | `refresh_active_status` + `maybe_refresh_status` | ✓ WIRED | `watch.rs:159-162` Timeout 分支节流刷新；`watch.rs:353-358` elapsed 动态计算 |
| `handle_run` 返回值          | `ErrorStats.records_exported` 累计 | `total_stats.merge(&file_stats)`        | ✓ WIRED    | `watch.rs:290`；`error.rs:130` merge 累计                                           |

### Data-Flow Trace (Level 4)

| Artifact             | Data Variable              | Source                              | Produces Real Data | Status      |
| -------------------- | -------------------------- | ----------------------------------- | ------------------ | ----------- |
| `watch.rs` 状态行 last 字段 | `last_trigger_at.elapsed()` | `Instant::now()` 在 `process_log_path:292` 设置，`refresh_active_status:357` 读取 elapsed | 是（运行时 monotonic clock）| ✓ FLOWING |
| `watch.rs` 累计行数  | `total_stats.records_exported` | `handle_run` 返回 ErrorStats → merge | 是（来自 handle_run 真实解析计数）| ✓ FLOWING |
| `watch.rs` 触发计数  | `trigger_count`            | 每次 `process_log_path` 成功后 `+= 1`  | 是（防抖后仅计一次）| ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior                       | Command                                                 | Result                                          | Status  |
| ------------------------------ | ------------------------------------------------------- | ----------------------------------------------- | ------- |
| watch --help 可发现            | `cargo run --release -- watch --help`（via assert_cmd） | 含 "TOML configuration file path" 和示例         | ✓ PASS  |
| watch 单元测试 9/9             | `cargo test --lib cli::watch::`                         | 9 passed; 0 failed                              | ✓ PASS  |
| 集成测试 watch_tests 3/3 + 1 ignored | `cargo test --test integration watch_tests::`     | 3 passed; 0 failed; 1 ignored                   | ✓ PASS  |
| 全套 cargo test                | `cargo test`                                            | 0 failed; 2 ignored（852 total passed）          | ✓ PASS  |
| clippy 零警告                  | `cargo clippy --all-targets -- -D warnings`             | exit 0，0 warnings                               | ✓ PASS  |
| cargo fmt --check              | `cargo fmt --check`                                     | exit 0                                          | ✓ PASS  |
| cargo build --release          | `cargo build --release`                                 | 零错误零警告                                     | ✓ PASS  |
| 静态 "just now" 已删除         | `grep -c '"last: just now"' src/cli/watch.rs`           | 0                                               | ✓ PASS  |
| HumanDuration 使用次数         | `grep -c 'HumanDuration' src/cli/watch.rs`              | 7                                               | ✓ PASS  |
| DEBOUNCE_WINDOW 命名常量       | `grep -c 'const DEBOUNCE_WINDOW' src/cli/watch.rs`      | 1                                               | ✓ PASS  |

### Probe Execution

不适用 — Phase 69 无声明式 probe 脚本。

### Requirements Coverage

| Requirement | Source Plan     | Description                                                          | Status        | Evidence                                                                                                                 |
| ----------- | --------------- | -------------------------------------------------------------------- | ------------- | ------------------------------------------------------------------------------------------------------------------------ |
| WATCH-01    | 69-01, 69-02    | `sqllog2db watch -c config.toml` 启动并可发现                        | ✓ SATISFIED   | `Commands::Watch` variant + `test_watch_help_lists_subcommand` 通过                                                      |
| WATCH-02    | 69-02, 69-03, 69-04 | 新增 .log 文件自动触发处理（单次触发，含防抖）                    | ~ PARTIAL     | 代码路径完整 + 防抖 500ms 正确；但 `test_watch_triggers_on_new_log_file` 仍 `#[ignore]`，端到端行为需人工 smoke test 验证 |
| WATCH-05    | 69-02, 69-03, 69-04 | 实时显示监听路径、上次触发时间（动态）、累计已处理行数              | ✓ SATISFIED   | `render_active_status` 使用 `HumanDuration(elapsed)`；`maybe_refresh_status` 200ms 节流刷新；9 个单元测试含 `test_render_active_status_includes_human_duration` 通过 |
| WATCH-06    | 69-02, 69-03    | Ctrl+C 优雅退出，打印最终摘要                                        | ✓ SATISFIED   | `test_watch_exits_when_interrupted` 通过；`print_final_summary` 摘要格式正确                                              |

**WATCH-03、WATCH-04** 明确分配至 Phase 70，不在 Phase 69 核查范围内。

### Anti-Patterns Found

| File                | Line | Pattern              | Severity   | Impact                        |
| ------------------- | ---- | -------------------- | ---------- | ----------------------------- |
| 无                  | —    | —                    | —          | 前次警告 `"last: just now"` 已删除 |

无 TBD/FIXME/XXX/HACK/PLACEHOLDER 未引用的 debt marker。

### Human Verification Required

#### 1. WATCH-02 端到端触发验证（ROADMAP SC2）

**Test:** 使用实际配置文件运行 `cargo run --release -- watch -c config.toml`，向配置的 `inputs` 目录写入一个格式正确的 .log 文件，观察：
1. 触发是否在 2 秒内发生
2. 状态行是否更新为 `triggers: 1 | processed: N rows`
3. CSV 或 SQLite 输出文件是否被创建或更新
4. 同一文件写入后 `triggers` 计数是否只增加 1（防抖验证：Create + Modify 双事件只触发一次）
5. 等待 5-10 秒，观察状态行 `last` 字段是否从 "just now" 变化为 "5s"/"10s" 等动态时间

**Expected:** 文件写入后 2 秒内触发，`triggers: 1`（不是 2），`last` 字段随时间递增更新，输出文件包含数据行。

**Why human:** `test_watch_triggers_on_new_log_file` 被标 `#[ignore]`（macOS FSEvents + `cargo test` stdin-pipe 阻塞，Phase 70 用 subprocess 修复）。ROADMAP SC2 明确要求端到端触发行为，代码路径已验证正确但缺乏自动化覆盖。

### Gaps Summary

**无代码阻塞性 gaps。** Plan 04 修复的两个问题均已在代码层关闭并通过测试验证。

遗留一项 human_needed 项：

1. **WATCH-02 端到端 smoke test（继承自 Phase 69 UAT 设计决策）**：`test_watch_triggers_on_new_log_file` 因 macOS stdin-pipe 问题被标 `#[ignore]`，Phase 70 将通过 subprocess 方式解决。在 Phase 70 修复集成测试前，此项需人工 smoke test 确认。注意这是已知设计决策，不影响 WATCH-02 代码路径的正确性（防抖、过滤、handle_run 委托均已通过 9 个单元测试和 3 个集成测试覆盖）。

---

_Verified: 2026-06-06T08:00:00Z_
_Verifier: Claude (gsd-verifier)_
