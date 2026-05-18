---
phase: 19
slug: code-refactor
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-18
---

# Phase 19 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml |
| **Quick run command** | `cargo clippy --all-targets -- -D warnings && cargo test` |
| **Full suite command** | `cargo clippy --all-targets -- -D warnings && cargo test && cargo build --release` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo clippy --all-targets -- -D warnings && cargo test`
- **After every plan wave:** Run `cargo clippy --all-targets -- -D warnings && cargo test && cargo build --release`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 19-01-01 | 01 | 1 | REFACTOR-01 | — | N/A | compile+test | `cargo test` | ✅ | ⬜ pending |
| 19-02-01 | 02 | 1 | REFACTOR-02 | — | N/A | compile+test | `cargo test` | ✅ | ⬜ pending |
| 19-03-01 | 03 | 2 | REFACTOR-03 | — | N/A | compile+test | `cargo test` | ✅ | ⬜ pending |
| 19-04-01 | 04 | 2 | REFACTOR-04 | — | N/A | compile+test | `cargo clippy --all-targets -- -D warnings && cargo test` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. (所有测试在模块内部，无需新建测试基础设施)

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
