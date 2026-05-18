---
phase: 20
slug: test-coverage
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-18
---

# Phase 20 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test + proptest |
| **Config file** | Cargo.toml (dev-dependencies) |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test && cargo clippy --all-targets -- -D warnings` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo test && cargo clippy --all-targets -- -D warnings`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 20-01-01 | 01 | 1 | TEST-01 | — | N/A | docs | `ls .planning/phases/*/VERIFICATION.md` | ❌ W0 | ⬜ pending |
| 20-02-01 | 02 | 2 | TEST-02 | — | N/A | integration | `cargo test e2e` | ❌ W0 | ⬜ pending |
| 20-03-01 | 03 | 2 | TEST-03 | — | N/A | unit | `cargo test boundary` | ❌ W0 | ⬜ pending |
| 20-04-01 | 04 | 2 | TEST-04 | — | N/A | property | `cargo test proptest` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/integration/` — e2e test fixture directory and test file stub
- [ ] `Cargo.toml` — add `proptest` to `[dev-dependencies]`
- [ ] `tests/fixtures/` — sample `.log` fixture files for e2e test

*Wave 0 must create test infrastructure before functional test code runs.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| VERIFICATION.md completeness review | TEST-01 | Docs quality check | Review each VERIFICATION.md covers all UAT criteria and success criteria for its phase |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
