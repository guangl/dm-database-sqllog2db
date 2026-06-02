---
phase: 58
slug: cli-run
status: draft
nyquist_compliant: false
wave_0_complete: true
created: 2026-06-02
---

# Phase 58 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (built-in) |
| **Config file** | `Cargo.toml` |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 58-01-01 | 01 | 1 | CLEAN-02 | — | N/A | e2e | `cargo test test_cli_run_csv_output_header_and_row_count` | ✅ `tests/integration.rs` | ⬜ pending |
| 58-01-02 | 01 | 1 | CLEAN-02 | — | N/A | e2e | `cargo test test_cli_run_sqlite_output_row_count` | ✅ `tests/integration.rs` | ⬜ pending |
| 58-01-03 | 01 | 1 | CLEAN-02 | — | N/A | static | `cargo clippy --all-targets -- -D warnings` | — | ⬜ pending |
| 58-01-04 | 01 | 1 | CLEAN-02 | — | N/A | static | `cargo fmt --check` | — | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. Phase 57 已提供完整 e2e 测试安全网：
- `tests/integration.rs` — `test_cli_run_csv_output_header_and_row_count`、`test_cli_run_sqlite_output_row_count`、`test_cli_init_*`、`test_cli_stats_*` 等

无需新增 Wave 0 测试文件。

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (existing infra sufficient)
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
