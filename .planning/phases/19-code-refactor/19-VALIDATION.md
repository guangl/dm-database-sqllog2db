---
phase: 19
slug: code-refactor
status: approved
nyquist_compliant: true
wave_0_complete: true
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
| 19-01-01 | 01 | 1 | REFACTOR-01 | — | N/A | compile+test | `cargo test --lib pipeline::filters` | ✅ | ✅ green |
| 19-02-01 | 02 | 1 | REFACTOR-01 | — | N/A | compile+test | `cargo test --lib config` | ✅ | ✅ green |
| 19-03-01 | 03 | 2 | REFACTOR-02, REFACTOR-03 | — | N/A | compile+test | `cargo test --lib exporter` | ✅ | ✅ green |
| 19-04-01 | 04 | 2 | REFACTOR-01, REFACTOR-04 | — | N/A | compile+clippy+test | `cargo clippy --all-targets -- -D warnings && cargo test --all-targets` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Audit 2026-05-18

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 1 (stats.rs 未拆分 — D-01 范围外，延后) |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-05-18
