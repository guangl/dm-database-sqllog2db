---
phase: 01-watch
verified: 2026-06-06T00:00:00Z
status: human_needed
score: 4/4 must-haves verified
overrides_applied: 0
human_verification:
  - test: "实际运行 `cargo run -- watch -c config.toml`，等待触发后按 Ctrl+C，执行 `echo $?`"
    expected: "终端输出退出码 130"
    why_human: "SIGINT 信号交互与退出码需在真实终端环境验证，grep 无法检测运行时行为"
---

# Phase 1: watch 功能完善 Verification Report

**Phase Goal:** watch 子命令功能完整：支持 CSV 导出格式增量追加、error log 以追加模式写入不丢失历史错误、Ctrl+C 退出码修正为 130
**Verified:** 2026-06-06
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | watch 触发 CSV exporter 时新增记录追加到现有 CSV 文件，多次触发后文件含累计记录与单一 header | ✓ VERIFIED | `force_append_for_watch_trigger` 在 `trigger_full_file`（L319）和 `build_incremental_cfg`（L542）中注入 `csv_cfg.append=true; csv_cfg.overwrite=false`；`test_watch_csv_append` 单元测试通过 |
| 2 | watch 期间产生的所有 parse error 追加写入 error log，不覆盖前次触发的历史错误 | ✓ VERIFIED | `force_append_for_watch_trigger` 同时设置 `cfg.append_error_log=true`；`write_error_log` 在 `append_error_log=true` 时使用 `OpenOptions::new().create(true).append(true).open()`；`test_watch_error_log_append` 单元测试通过 |
| 3 | watch 进程被 SIGINT/Ctrl+C 中断后 `handle_watch` 返回 `Err(Error::Interrupted)`，main.rs 走 exit(130) 分支 | ✓ VERIFIED | `handle_watch` L74-76：`if interrupted.load(Ordering::Acquire) { return Err(Error::Interrupted); }`，位于 `print_final_summary`（L67）之后；`test_handle_watch_returns_interrupted` 单元测试通过；`main.rs` 已有 `Err(e) if matches!(e, Error::Interrupted) => std::process::exit(130)` 分支 |
| 4 | run 子命令的 `write_error_log` 行为保持覆盖写不变（append_error_log 默认 false） | ✓ VERIFIED | `Config` 使用 `derive(Default)`，`bool` 默认值为 `false`；`write_error_log` else 分支为 `OpenOptions::new().create(true).write(true).truncate(true).open()`；`test_write_error_log_run_still_truncates` 单元测试通过 |

**Score:** 4/4 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/config/mod.rs` | `Config.append_error_log #[serde(skip)] pub bool 字段` | ✓ VERIFIED | L41-43：`#[serde(skip)]` + `pub append_error_log: bool`（SUMMARY 记录了 pub(crate)→pub 的可见性偏差，已自动修正以兼容集成测试结构体字面量语法） |
| `src/cli/run/mod.rs` | `write_error_log` 根据 `cfg.append_error_log` 选择 append 或 truncate | ✓ VERIFIED | L433-444：if/else 双分支实现；`File::create` 旧调用已完全替换（grep 返回 0 行）；函数签名保持不变 |
| `src/cli/watch/mod.rs` | `force_append_for_watch_trigger` 注入 CSV append + `append_error_log`；`handle_watch` 尾部 interrupted 检查 | ✓ VERIFIED | L521-529：辅助函数；L319（trigger_full_file 调用）；L542（build_incremental_cfg 调用）；L74-76（handle_watch 检查） |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `watch/mod.rs::trigger_full_file` | `config/mod.rs::Config.append_error_log` | `force_append_for_watch_trigger(&mut tmp_cfg)` at L319 | ✓ WIRED | `force_append_for_watch_trigger` L528 设置 `cfg.append_error_log = true` |
| `watch/mod.rs::build_incremental_cfg` | `exporter.rs::CsvExporterConfig.append` | `force_append_for_watch_trigger(&mut tmp_cfg)` at L542 | ✓ WIRED | `force_append_for_watch_trigger` L524 设置 `csv_cfg.append = true; csv_cfg.overwrite = false` |
| `run/mod.rs::write_error_log` | `std::fs::OpenOptions` | if/else 双分支 L433-444 | ✓ WIRED | append 分支与 truncate 分支均已实现，通过 `cfg.append_error_log` 路由 |
| `watch/mod.rs::handle_watch` | `error.rs::Error::Interrupted` | `interrupted.load(Ordering::Acquire)` 后 `return Err` at L74-76 | ✓ WIRED | 位于 `print_final_summary` 之后（L67 < L74），符合 D-08 摘要先于退出的要求 |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|--------------------|--------|
| `write_error_log` | `stats.parse_error_records` | `handle_run` 解析阶段收集 | 是（实际解析错误） | ✓ FLOWING |
| `force_append_for_watch_trigger` | `tmp_cfg.exporter.csv.append` | `cfg.clone()` 再注入 | 是（运行时注入） | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `test_watch_csv_append` 通过 | `cargo test --lib test_watch_csv_append` | 1 passed | ✓ PASS |
| `test_watch_error_log_append` 通过 | `cargo test --lib test_watch_error_log_append` | 1 passed | ✓ PASS |
| `test_write_error_log_run_still_truncates` 通过 | `cargo test --lib test_write_error_log_run_still_truncates` | 1 passed | ✓ PASS |
| `test_handle_watch_returns_interrupted` 通过 | `cargo test --lib test_handle_watch_returns_interrupted` | 1 passed（耗时 9.61s） | ✓ PASS |
| 全套库测试无回归 | `cargo test --lib` | 380 passed; 0 failed; 0 ignored | ✓ PASS |
| clippy 质量门禁 | `cargo clippy --all-targets -- -D warnings` | exit 0，无警告 | ✓ PASS |
| 格式检查 | `cargo fmt --check` | exit 0 | ✓ PASS |

### Probe Execution

Step 7c: SKIPPED（本 phase 无 `scripts/*/tests/probe-*.sh`，PLAN 中未声明 probe）

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| WATCH-07 | 01-01-PLAN.md | watch 子命令支持 CSV 导出增量追加 | ✓ SATISFIED | `force_append_for_watch_trigger` 在全量和增量路径均注入 `csv_cfg.append=true`；`test_watch_csv_append` 行为验证通过 |
| WATCH-08 | 01-01-PLAN.md | watch error log 追加写入不覆盖历史 | ✓ SATISFIED | `write_error_log` if/else 双分支；watch 路径注入 `append_error_log=true`；run 路径保持截断；两个测试均通过 |
| WATCH-09 | 01-01-PLAN.md | Ctrl+C 退出码 130 | ✓ SATISFIED（自动部分）| `handle_watch` 在 `print_final_summary` 后检查 `interrupted`，返回 `Err(Error::Interrupted)`；`main.rs` 已有 exit(130) 分支；单元测试通过；实际信号行为需人工验证 |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | — | — | 无 TBD/FIXME/XXX 标记，无 placeholder，无 stub 实现 |

扫描结果：`src/config/mod.rs`、`src/cli/run/mod.rs`、`src/cli/watch/mod.rs`、`src/cli/run/tests.rs` 均无 TBD/FIXME/XXX/placeholder/TODO 标记。

**注意（非阻塞，Code Review CR-01 记录项）：** `trigger_full_file` 调用 `force_append_for_watch_trigger` 会同时设置 `csv_cfg.append=true`，但 SQLite 的 append 标志仅在 `build_incremental_cfg` 中显式设置（增量路径）。PLAN 明确将 SQLite append 限定为增量路径，全量触发 SQLite append 不保护符合设计意图（PLAN 注释："SQLite 仅增量路径需要，与 force_append_for_watch_trigger 抽象边界不重合"）。这是已知的有意设计，不是 Bug，不影响 WATCH-07/08/09 目标。

### Human Verification Required

#### 1. Ctrl+C 退出码 130 实际验证

**Test:** 启动 `cargo run -- watch -c config.toml`（需有有效配置文件），等待监听状态显示后按 Ctrl+C，执行 `echo $?`
**Expected:** 终端显示最终摘要，随后输出退出码 `130`
**Why human:** SIGINT 信号通过终端键盘输入触发，无法通过 grep 或单元测试替代。`test_handle_watch_returns_interrupted` 验证了 `handle_watch` 返回 `Err(Error::Interrupted)`，但 `main.rs` 调用 `std::process::exit(130)` 的完整链路需在真实终端中确认。

### Gaps Summary

无阻塞性 gap。所有 4 个 must-have truths 均已验证，4 个新测试全部通过，三道质量门禁（clippy / fmt / test）全绿。仅剩一项人工验证（Ctrl+C 退出码实际信号行为），属于正常的端到端验收范围。

---

_Verified: 2026-06-06_
_Verifier: Claude (gsd-verifier)_
