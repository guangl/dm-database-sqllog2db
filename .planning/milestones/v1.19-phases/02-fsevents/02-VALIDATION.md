---
phase: 02
slug: fsevents
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-06
updated: 2026-06-07
---

# Phase 02 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + cargo-llvm-cov 0.8.5 |
| **Config file** | none（标准 cargo test） |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo llvm-cov --summary-only` |
| **Estimated runtime** | ~30 seconds (test) / ~60 seconds (llvm-cov) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo llvm-cov --summary-only` （TOTAL Line % ≥ 92%）
- **Before `/gsd:verify-work`:** Full suite green + coverage ≥ 92%
- **Max feedback latency:** ~60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 02-01-01 | 01 | 1 | QUAL-03 | T-02-01 | N/A | integration | `cargo test --test watch_incremental test_watch_07_csv_append` | ❌ Wave 0 | ⬜ pending |
| 02-01-02 | 01 | 1 | QUAL-03 | T-02-01 | N/A | integration | `cargo test --test watch_incremental test_watch_08_error_log_append` | ❌ Wave 0 | ⬜ pending |
| 02-01-03 | 01 | 1 | QUAL-03 | T-02-02 | N/A | integration | `cargo test --test watch_incremental test_watch_09_exit_code_130` | ❌ Wave 0 | ⬜ pending |
| 02-02-01 | 02 | 2 | QUAL-02 | T-02-03 | N/A | unit | `cargo test --lib test_collector_invalid_path_returns_error` | ❌ Wave 0 | ⬜ pending |
| 02-02-02 | 02 | 2 | QUAL-02 | T-02-03 | N/A | unit | `cargo test --lib test_collector_parse_error_accumulation` | ❌ Wave 0 | ⬜ pending |
| 02-02-03 | 02 | 2 | QUAL-02 | T-02-03 | N/A | unit | `cargo test --lib test_collector_not_needed_filtering` | ❌ Wave 0 | ⬜ pending |
| 02-02-04 | 02 | 2 | QUAL-02 | T-02-03 | N/A | unit | `cargo test --lib test_collector_filtered_params_normalize` | ❌ Wave 0 | ⬜ pending |
| 02-02-05 | 02 | 2 | QUAL-02 | — | N/A | coverage | `cargo llvm-cov --summary-only` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

**Plan column reflects the parent PLAN (`02-NN-PLAN.md`).**

---

## Wave 0 Requirements

- [ ] `src/cli/run/tests.rs` — collector.rs 单元测试（4 个：`test_collector_invalid_path_returns_error`、`test_collector_parse_error_accumulation`、`test_collector_not_needed_filtering`、`test_collector_filtered_params_normalize`）
- [ ] `tests/watch_incremental.rs` — WATCH-07/08/09 三个集成测试函数 + `build_csv_config` helper + `INVALID_LOG_LINE` 常量
- [ ] `tests/watch_incremental.rs` — 确认 `handle_watch` 和 `Error` 的 `use` 导入路径

*Wave 0 状态：当前所有测试函数与 helper 均不存在（❌），将在 Wave 1（02-01）与 Wave 2（02-02）执行期间创建。Group 3+4 测试（`test_collector_not_needed_filtering`、`test_collector_filtered_params_normalize`）由 02-02-PLAN.md Task 1 一并创建，依赖 `crate::pipeline::LogProcessor` trait 与本地 inline `AlwaysFail` 处理器，无独立 Wave 0 stub 需求。*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| FSEvents 保留 `#[ignore]` + 书面依据到位 | QUAL-03 | 属文档/决策验证而非代码行为 | 确认 `tests/integration.rs:2917` 仍有 `#[ignore]`；确认 `02-CONTEXT.md` D-01/D-02 决策存在 |
| cargo llvm-cov TOTAL Line % ≥ 92% | QUAL-02 | 需人工读取覆盖率输出并决定补救路径 | 02-02-PLAN.md Task 2 checkpoint，输入 `approved` / `补救：D-05` / `补救：扩展 D-05` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
</content>
</invoke>