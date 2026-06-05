---
plan: 67-01
phase: 67
subsystem: cli/run
status: complete
tags: [progress-bar, indicatif, records-per-sec, eta, tdd]
requirements: [PROG-01, PROG-02]
dependency_graph:
  requires: []
  provides: [make_progress_bar(show_progress, total_files), tick_progress(file_start, file_name)]
  affects: [src/cli/run/mod.rs, src/cli/run/processor.rs]
tech_stack:
  added: []
  patterns: [TDD RED/GREEN, indicatif file-counter progress bar]
key_files:
  created: []
  modified:
    - src/cli/run/mod.rs
    - src/cli/run/processor.rs
    - src/cli/run/tests.rs
decisions:
  - "进度条模板使用 {spinner:.cyan} [{pos}/{len}] {wide_msg} | eta {eta}，由 indicatif 自动渲染 ETA"
  - "tick_progress 不再调用 pb.inc(1024)；文件完成时在 log_file_result 末尾执行 pb.inc(1)"
  - "speed_label 阈值 10_000 rec/s 以上显示 Xk rec/s 格式"
metrics:
  duration: "~30min (estimated)"
  completed: "2026-06-05"
  tasks_completed: 2
  files_modified: 3
---

# Phase 67 Plan 01: 进度条升级 — spinner → 文件计数器 + ETA + records/sec

**One-liner:** 将顺序路径进度条由 spinner 升级为固定长度文件计数器（[N/M]）+ indicatif 自动 ETA + records/sec 速率显示。

## What Was Built

升级了 `make_progress_bar` 和 `tick_progress` 两个函数，实现 PROG-01/PROG-02 需求：

1. **`make_progress_bar(show_progress, total_files)`** — 新签名接收 `total_files: usize`，创建 `ProgressBar::new(total_files as u64)` 而非 spinner；模板改为 `"{spinner:.cyan} [{pos}/{len}] {wide_msg} | eta {eta}"`，让 indicatif 基于 `inc(1)` 速率自动渲染 ETA。

2. **`tick_progress(pb, records_in_file, file_start, file_name, interrupted)`** — 每 1024 条记录计算 elapsed 时间，生成 `rec_per_s` 速率，格式化为 `"Xk rec/s"`（>=10k）或 `"X rec/s"`；调用 `pb.set_message(format!("{file_name} | {speed_label}"))` 更新进度条消息。

3. **`log_file_result` 末尾追加 `pb.inc(1)`** — 每个文件完成时将 `[pos]` 自增，驱动 ETA 计算；不再在 tick 时调用 `pb.inc(1024)`。

4. **单元测试** — `test_progress_bar_template`（验证 `length()==Some(3)`、`position()==0`、模板不 panic）和 `test_progress_bar_disabled`（验证 `false` 返回 `None`）。

## Key Files

### Created
- (none)

### Modified
- `src/cli/run/mod.rs` — `make_progress_bar` 新签名、新模板、调用点更新
- `src/cli/run/processor.rs` — `tick_progress` 新参数 + records/sec 计算、`log_file_result` 追加 `pb.inc(1)`
- `src/cli/run/tests.rs` — 新增 `test_progress_bar_template` 和 `test_progress_bar_disabled`

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| Task 1 (RED) + Task 2 (GREEN) | db845cc | feat(67-01): 升级进度条为文件计数器 + ETA + records/sec |

## Deviations from Plan

None — 计划按原定步骤执行完毕。TDD RED/GREEN 流程：RED（tests.rs 写测试后旧签名编译失败）→ GREEN（mod.rs + processor.rs 实现新签名后所有测试通过）。由于上一个 agent 在 commit 阶段遭遇 1Password SSH signing 超时，代码已完成但需在本 agent 中补充提交。

## Known Stubs

None.

## Threat Flags

None — `file_name` 写入 stderr 与现状一致（T-67-02 已 accept），`tick_progress` 高频格式化有 1024 节奏限制（T-67-01 已 accept）。

## Self-Check: PASSED

- [x] Task 1 (RED): test_progress_bar_template + test_progress_bar_disabled 写入 tests.rs
- [x] Task 2 (GREEN): make_progress_bar 新签名、tick_progress 新参数、pb.inc(1) 在 log_file_result
- [x] Tests pass: `cargo test --lib cli::run::tests` — 9 passed, 0 failed
- [x] Clippy clean: `cargo clippy --all-targets -- -D warnings` — 通过（pre-commit hook 验证）
- [x] Commit db845cc 存在并包含正确变更
