---
phase: 20
slug: test-coverage
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-18
---

# Phase 20 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test + proptest 1.6.0 |
| **Config file** | Cargo.toml ([dev-dependencies] proptest = "1.6.0") |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test --all-targets && cargo clippy --all-targets -- -D warnings` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo test --all-targets && cargo clippy --all-targets -- -D warnings`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 20-01-01 | 01 | 1 | TEST-01 | — | N/A | docs | `ls .planning/phases/12-*/VERIFICATION.md .planning/phases/13-*/VERIFICATION.md .planning/phases/14-*/VERIFICATION.md .planning/phases/16-*/VERIFICATION.md` | ✅ | ✅ green |
| 20-02-01 | 02 | 2 | TEST-02 | — | N/A | integration | `cargo test --test integration test_e2e` | ✅ | ✅ green |
| 20-02-02 | 02 | 2 | TEST-03 | — | N/A | integration | `cargo test --test integration test_boundary` | ✅ | ✅ green |
| 20-03-01 | 03 | 2 | TEST-04 | — | N/A | property | `cargo test --lib pipeline::fingerprint::tests` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `tests/integration.rs` — 3 E2E tests (test_e2e_filter_pipeline, test_e2e_template_normalization, test_e2e_field_projection)
- [x] `tests/integration.rs` — 4 boundary tests (test_boundary_empty_log_file, test_boundary_all_filtered, test_boundary_malformed_line, test_boundary_long_sql)
- [x] `Cargo.toml` — proptest = "1.6.0" in [dev-dependencies]
- [x] `src/pipeline/fingerprint.rs` — 2 proptest property tests (prop_normalize_template_is_idempotent, prop_normalize_template_literal_protection)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| VERIFICATION.md 内容完整性审查 | TEST-01 | 文档质量检查 | 人工审查各 VERIFICATION.md 覆盖 UAT 标准与成功标准 |

---

## Validation Audit 2026-05-18

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 10s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-05-18
