---
phase: 67
slug: prog-diag
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-05
updated: 2026-06-07
---

# Phase 67 — Validation Strategy

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
| 67-01-01 | 01 | 1 | PROG-01, PROG-02 | — | N/A | unit | `cargo test --lib cli::run::tests` | ✅ | ✅ complete |
| 67-02-01 | 02 | 2 | DIAG-01, DIAG-02 | — | N/A | unit | `cargo test --lib` | ✅ | ✅ complete |
| 67-03-01 | 03 | 3 | PROG-03, DIAG-03 | — | N/A | unit+integration | `cargo test` | ✅ | ✅ complete |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify commands
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** complete（依据：67-01/02/03-SUMMARY.md self-check: PASSED + 344 unit tests passing）
