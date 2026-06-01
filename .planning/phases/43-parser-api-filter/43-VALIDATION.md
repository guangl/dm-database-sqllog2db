---
phase: 43
slug: parser-api-filter
status: ready
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-24
---

# Phase 43 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | none — existing infrastructure |
| **Quick run command** | `cargo test --lib -- filter` |
| **Full suite command** | `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib -- filter`
- **After every plan wave:** Run `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 43-01-01 | 01 | 1 | PARSER-02 | — | N/A | unit | `cargo test --lib -- filter` | ✅ | ⬜ pending |
| 43-01-02 | 01 | 1 | PARSER-02 | — | N/A | unit | `cargo test --lib -- filter` | ✅ | ⬜ pending |
| 43-02-01 | 02 | 2 | REFACTOR-01 | — | N/A | unit | `cargo test --lib -- filter` | ✅ | ⬜ pending |
| 43-02-02 | 02 | 2 | REFACTOR-01 | — | N/A | unit | `cargo test` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| git diff --stat 验证代码行数减少 | PARSER-02 | 需要人工判断减少量是否合理 | `git diff --stat` 对比重构前后 |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (existing infrastructure sufficient, no Wave 0 tasks needed)
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-05-24
