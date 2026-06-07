---
phase: 70
slug: watch
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-06
updated: 2026-06-07
---

# Phase 70 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `cargo test` |
| **Config file** | Cargo.toml（无独立 test config）|
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
- **Before `/gsd:verify-work`:** Full suite must be green

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 70-01-01 | 01 | 1 | WATCH-04 | — | N/A | unit | `cargo test --lib` | ✅ | ✅ complete |
| 70-02-01 | 02 | 2 | WATCH-03, WATCH-04 | — | N/A | unit | `cargo test --lib` | ✅ | ✅ complete |
| 70-03-01 | 03 | 3 | WATCH-03, WATCH-04 | — | N/A | integration | `cargo test --test watch_incremental` | ✅ | ✅ complete |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify commands
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** complete（依据：70-01/02/03-SUMMARY.md self-check: PASSED；5 + 4 单元测试 + 4 个集成测试通过；`cargo test --test watch_incremental` 4 passed, 0 failed）
