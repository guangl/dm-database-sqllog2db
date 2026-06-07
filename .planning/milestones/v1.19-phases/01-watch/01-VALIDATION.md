---
phase: 1
slug: watch
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-06
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test（内置）|
| **Config file** | Cargo.toml（无独立 test config）|
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 1-01-01 | 01 | 1 | WATCH-07 | — | N/A | integration | `cargo test test_watch_csv_append` | ❌ Wave 0 | ⬜ pending |
| 1-01-02 | 01 | 1 | WATCH-08 | — | N/A | integration | `cargo test test_watch_error_log_append` | ❌ Wave 0 | ⬜ pending |
| 1-01-03 | 01 | 1 | WATCH-08 | — | N/A | unit | `cargo test test_write_error_log_run_still_truncates` | ❌ Wave 0 | ⬜ pending |
| 1-01-04 | 01 | 1 | WATCH-09 | — | N/A | unit | `cargo test test_handle_watch_returns_interrupted` | ❌ Wave 0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `test_watch_csv_append` — 验证 WATCH-07：两次 `trigger_full_file` 调用后 CSV 包含两批记录、仅一个 header
- [ ] `test_watch_error_log_append` — 验证 WATCH-08：两次带解析错误的触发后 error log 包含所有历史错误行
- [ ] `test_write_error_log_run_still_truncates` — 验证 `append_error_log=false`（run 路径）仍覆盖写 error log
- [ ] `test_handle_watch_returns_interrupted` — 验证 `interrupted=true` 时 `handle_watch` 返回 `Err(Error::Interrupted)`

新增测试追加到 `src/cli/watch/mod.rs` 的 `#[cfg(test)]` 块（单元测试）或 `tests/integration.rs`（集成测试）。

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| watch 进程真实 Ctrl+C 退出码 | WATCH-09 | 退出码需在 shell 中验证 `$?` | `cargo run -- watch -c config.toml` → Ctrl+C → `echo $?`，应输出 `130` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
