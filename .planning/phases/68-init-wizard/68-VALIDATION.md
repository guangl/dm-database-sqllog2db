---
phase: 68
slug: init-wizard
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-06
---

# Phase 68 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml |
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
| 68-01-01 | 01 | 1 | INIT-01 | — | N/A | unit | `cargo test wizard` | ❌ W0 | ⬜ pending |
| 68-01-02 | 01 | 1 | INIT-02 | — | N/A | unit | `cargo test wizard` | ❌ W0 | ⬜ pending |
| 68-01-03 | 01 | 1 | INIT-03 | — | N/A | unit | `cargo test wizard` | ❌ W0 | ⬜ pending |
| 68-02-01 | 02 | 2 | INIT-01 | — | N/A | integration | `cargo test init_interactive` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Unit tests for `run_wizard` (Cursor-based BufRead simulation) covering:
  - csv defaults path (all Enter)
  - custom csv path
  - sqlite path (with table_name)
  - invalid format re-prompt
  - empty inputs accepts default
- [ ] Integration test `test_init_interactive` in `tests/integration.rs` verifying generated config passes `sqllog2db validate`

*Existing cargo test infrastructure covers all phase requirements — no new framework needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Terminal prompt display | INIT-02 | Cannot assert print! output in automated TTY tests | Run `cargo run -- init --interactive`, verify prompts show defaults and examples |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
