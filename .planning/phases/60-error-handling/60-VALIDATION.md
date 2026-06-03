---
phase: 60
slug: error-handling
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-03
audited: 2026-06-03
---

# Phase 60 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (内置) |
| **Config file** | Cargo.toml |
| **Quick run command** | `cargo clippy --all-targets -- -D warnings && cargo test` |
| **Full suite command** | `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo clippy --all-targets -- -D warnings && cargo test`
- **After every plan wave:** Run `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 60-01-01 | 01 | 1 | STRUCT-03 | static | `cargo clippy --all-targets -- -D warnings` | ✅ | ✅ green |
| 60-01-02 | 01 | 1 | STRUCT-03 | smoke | `grep -r 'unwrap()\|expect(' src/ --include="*.rs"` (手动检查注释) | ✅ 手动 | ✅ green (manual) |
| 60-01-03 | 01 | 1 | STRUCT-03 | e2e | `cargo test` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

None — 现有测试基础设施完全覆盖本阶段需求（Phase 57 已补全 e2e 测试）。

*Existing infrastructure covers all phase requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| grep 扫描无未注释 unwrap/expect | STRUCT-03 | 需要人工确认注释内容正确 | `grep -r 'unwrap()\|expect(' src/ --include="*.rs"` 检查每个结果行是否有注释或在测试代码中 |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** 2026-06-03

---

## Validation Audit 2026-06-03

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |
| COVERED | 2 |
| MANUAL | 1 |

All automated commands pass: clippy (exit 0), cargo test (366+335+68 = 769 passed), fmt (exit 0). No new test files required — phase only added comments to production code.
