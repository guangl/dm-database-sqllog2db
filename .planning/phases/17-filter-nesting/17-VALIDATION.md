---
phase: 17
slug: filter-nesting
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-17
---

# Phase 17 — Validation Strategy

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

- **After every task commit:** Run `cargo clippy --all-targets -- -D warnings && cargo test --lib`
- **After every plan wave:** Run `cargo test --all-targets`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 3 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 17-01-01 | 01 | 1 | CONFIG-01 | — | N/A | unit | `cargo test --lib pipeline::filters::compiled_tests` | ✅ | ✅ green |
| 17-01-02 | 01 | 1 | CONFIG-02 | — | N/A | unit | `cargo test --lib pipeline::filters::compiled_tests` | ✅ | ✅ green |
| 17-01-03 | 01 | 1 | CONFIG-05 | — | N/A | integration | `cargo test --test integration test_init_generates_new_nested_format` | ✅ | ✅ green |
| 17-01-04 | 01 | 1 | CONFIG-05 | — | N/A | unit | `cargo test --lib config::validate::test_validate_new_nested_format_passes` | ✅ | ✅ green |
| 17-02-01 | 02 | 2 | CONFIG-01 | — | N/A | unit | `cargo test --lib config::validate::test_validate_and_compile_new_format_filter_enabled` | ✅ | ✅ green |
| 17-02-02 | 02 | 2 | CONFIG-05 | — | N/A | integration | `cargo test --test integration` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. Phase 17 delivered:
- 5 parse tests in plan 01 (RED→GREEN TDD cycle)
- `test_validate_new_nested_format_passes` — new [features.filters.include]/[features.filters.exclude] TOML
- `test_validate_old_flat_format_passes` — backward compat flat fields
- `test_init_generates_new_nested_format` — init generates nested format
- `test_validate_and_compile_new_format_filter_*` — new format filters
- All tests migrated through Phase 19 refactoring (filters.rs split), coverage preserved

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `cargo run -- validate -c config.toml` 旧格式通过 | CONFIG-05 | CLI 端到端行为 | `cargo run -- validate -c config.toml` 返回 exit 0 |

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
- [x] Feedback latency < 3s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-05-18
