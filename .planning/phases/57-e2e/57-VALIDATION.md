---
phase: 57
slug: e2e
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-02
---

# Phase 57 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in + assert_cmd) |
| **Config file** | none — existing infrastructure |
| **Quick run command** | `cargo test --test integration 2>&1 \| tail -20` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --test integration 2>&1 | tail -20`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | Status |
|---------|------|------|-------------|-----------|-------------------|--------|
| 57-01-01 | 01 | 1 | TEST-03 | unit | `cargo test stats::config::tests` | ⬜ pending |
| 57-01-02 | 01 | 1 | TEST-03 | integration | `cargo test --test integration test_cli_stats_from_greater_than_to` | ⬜ pending |
| 57-01-03 | 01 | 2 | TEST-01 | integration | `cargo test --test integration test_cli_run_csv` | ⬜ pending |
| 57-01-04 | 01 | 2 | TEST-01 | integration | `cargo test --test integration test_cli_run_sqlite` | ⬜ pending |
| 57-01-05 | 01 | 2 | TEST-02 | integration | `cargo test --test integration test_cli_init` | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

- `tests/integration.rs` — 现有文件，追加新测试函数即可
- `assert_cmd`, `predicates`, `tempfile`, `rusqlite` — 已在 Cargo.toml dev-dependencies 中

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
