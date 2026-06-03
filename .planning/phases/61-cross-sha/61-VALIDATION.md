---
phase: 61
slug: cross-sha
status: partial
nyquist_compliant: false
wave_0_complete: true
created: 2026-06-03
---

# Phase 61 — Validation Strategy

> Per-phase validation contract for Phase 61: Cross.toml SHA256 digest pinning.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test |
| **Config file** | Cargo.toml |
| **Quick run command** | `cargo test --test cross_config` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~3 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --test cross_config`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** ~3 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 61-01-01 | 01 | 1 | CROSS-01 | T-61-01 / T-61-02 | SHA obtained from live registry (one-time) | manual | — | N/A | ⬜ manual-only |
| 61-01-02 | 01 | 1 | CROSS-01 | T-61-01 / T-61-02 | `@sha256:` present, `:edge` absent, digest is valid 64-hex | integration | `cargo test --test cross_config` | ✅ | ✅ green |
| 61-01-03 | 01 | 1 | CROSS-01 | T-61-03 | cargo quality gates pass (clippy/test/fmt) | suite | `cargo test` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all automated phase requirements.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| SHA256 digest queried from ghcr.io registry | CROSS-01 | One-time live registry query; registry state at query time cannot be replayed | Run: `docker manifest inspect ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge --verbose` and record the amd64 digest when updating the pin |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or justified manual-only
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (task 2+3 automated)
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 3s
- [ ] `nyquist_compliant: true` — blocked by manual-only Task 1 (justified: live registry query)

**Approval:** partial 2026-06-03

---

## Validation Audit 2026-06-03

| Metric | Count |
|--------|-------|
| Gaps found | 2 |
| Resolved | 1 |
| Escalated (manual-only) | 1 |

Tests added: `tests/cross_config.rs` (3 assertions: SHA present, :edge absent, digest valid 64-hex)

## Validation Audit 2026-06-03 (re-audit)

| Metric | Count |
|--------|-------|
| New gaps found | 0 |
| Resolved | 0 |
| Manual-only confirmed | 1 |

Re-audit: `cargo test --test cross_config` 3/3 passed. CROSS-01 coverage confirmed via `tests/cross_config.rs`. Task 1 manual-only justified (one-time live registry query, result permanently fixed in Cross.toml). Status remains partial.
