---
phase: 30
slug: remove-template-analysis
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-20
---

# Phase 30 — Validation Strategy

> 移除模板分析功能，需确认热循环快路径不受影响。

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | None — cargo native |
| **Quick run command** | `cargo build` |
| **Full suite command** | `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo build`
- **After every plan wave:** Run `cargo test && cargo clippy --all-targets -- -D warnings`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 30-01-01 | 01 | 1 | RM-05 | N/A | N/A | build | `cargo build` | ❌ W0 | ⬜ pending |
| 30-02-01 | 02 | 2 | RM-05 | N/A | N/A | build | `cargo build` | ❌ W0 | ⬜ pending |
| 30-03-01 | 03 | 3 | RM-05 | N/A | N/A | build+test | `cargo build && cargo test` | ❌ W0 | ⬜ pending |

---

## Wave 0 Requirements

None — 现有测试基础设施覆盖。

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| 运行不再生成模板报告文件 | RM-05 | 文件系统输出检查 | 用旧 config 运行 `cargo run -- run -c old.toml` 确认无 `*_templates.*` 文件生成 |

---

## Validation Sign-Off

- [ ] All tasks have automated verify
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
