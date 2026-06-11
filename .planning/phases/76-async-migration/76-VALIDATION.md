---
phase: 76
slug: async-migration
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-11
---

# Phase 76 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test（内置）+ criterion 0.7（bench） |
| **Config file** | Cargo.toml（[[bench]] entries） |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test && cargo clippy --all-targets -- -D warnings && cargo build --release` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo test && cargo clippy --all-targets -- -D warnings && cargo build --release`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 76-01-01 | 01 | 0 | ASYNC-01 | — | N/A | unit+integration | `cargo test` | ✅ | ⬜ pending |
| 76-01-02 | 01 | 0 | ASYNC-01 | — | N/A | lint | `cargo clippy --all-targets -- -D warnings` | ✅ | ⬜ pending |
| 76-01-03 | 01 | 0 | ASYNC-01 | — | N/A | build | `cargo build --release && ls -lh target/release/sqllog2db` | ✅ | ⬜ pending |
| 76-01-04 | 01 | 0 | ASYNC-01 | — | N/A | bench | `cargo bench --bench bench_csv -- csv_export_real` (conditional) | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `cargo test` — 503 tests all green (verified during research)
- [x] `cargo clippy --all-targets -- -D warnings` — 0 warnings (verified during research)
- [x] `cargo build --release` — 3.8MB binary, successful (verified during research)
- [ ] `cargo bench --bench bench_csv -- csv_export_real` — conditional on `sqllogs/` directory existing

*Implementation already complete — Wave 0 is purely verification.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| SC-2: bench 吞吐量不低于 v1.19 基线 | ASYNC-01 | 需要真实 sqllogs/ 日志文件，CI 环境无法提供 | 若有真实日志：`cargo bench --bench bench_csv -- csv_export_real`，对比结果不低于 ~1.55M records/sec |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
