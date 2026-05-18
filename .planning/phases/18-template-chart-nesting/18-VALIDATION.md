---
phase: 18
slug: template-chart-nesting
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-18
---

# Phase 18 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | none — cargo test auto-discovers |
| **Quick run command** | `cargo test --lib` |
| **Full suite command** | `cargo test --all-targets` |
| **Estimated runtime** | ~3 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib`
- **After every plan wave:** Run `cargo test --all-targets`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 3 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 18-01-01 | 01 | 1 | CONFIG-03 | — | N/A | unit | `cargo test --lib pipeline::test_template_config` | ✅ | ✅ green |
| 18-01-02 | 01 | 1 | CONFIG-03 | — | N/A | unit | `cargo test --lib pipeline::test_output_config` | ✅ | ✅ green |
| 18-01-03 | 01 | 1 | CONFIG-03 | — | N/A | unit | `cargo test --lib config::apply_one::test_apply_one_template` | ✅ | ✅ green |
| 18-01-04 | 01 | 1 | CONFIG-04 | T-18-01 | pipeline_deprecated rejection with 5 migration mappings | unit | `cargo test --lib config::validate::test_validate_template_sqlite_table` | ✅ | ✅ green |
| 18-02-01 | 02 | 2 | CONFIG-03 | — | N/A | unit | `cargo test --lib exporter::csv::tests::test_csv_write_template_stats` | ✅ | ✅ green |
| 18-02-02 | 02 | 2 | CONFIG-03 | T-18-05 | ascii_alphanumeric_or_underscore SQL injection prevention | unit | `cargo test --lib exporter::sqlite::tests::test_sqlite_write_template_stats` | ✅ | ✅ green |
| 18-02-03 | 02 | 2 | CONFIG-03 | — | N/A | unit | `cargo test --lib exporter::tests::test_default_write_template_stats_noop` | ✅ | ✅ green |
| 18-03-01 | 03 | 3 | CONFIG-03 | — | N/A | integration | `cargo test --test integration test_init_generated_zh_template_passes_validate` | ✅ | ✅ green |
| 18-03-02 | 03 | 3 | CONFIG-03 | — | N/A | integration | `cargo test --test integration test_init_generated_en_template_passes_validate` | ✅ | ✅ green |
| 18-03-03 | 03 | 3 | CONFIG-04 | T-18-01 | validate() rejects [pipeline.*] with migration error | integration | `cargo test --test integration test_validate_rejects_legacy_pipeline_template_analysis` | ✅ | ✅ green |
| 18-03-04 | 03 | 3 | CONFIG-04 | T-18-01 | validate() rejects [pipeline.filters] with 5 migration mappings | integration | `cargo test --test integration test_validate_rejects_legacy_pipeline_filters_section` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 3s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-05-18
