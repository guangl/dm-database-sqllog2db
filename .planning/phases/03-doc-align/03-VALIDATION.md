---
phase: 03
slug: doc-align
status: draft
nyquist_compliant: false
wave_0_complete: true
created: 2026-06-07
updated: 2026-06-07
---

# Phase 03 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `cargo test` |
| **Config file** | Cargo.toml（无独立 test config）|
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
- **After every plan wave:** Run full suite
- **Before `/gsd:verify-work`:** Full suite green

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 03-01-01 | 01 | 1 | DOC-05 | automated | `cargo clippy --all-targets -- -D warnings && cargo fmt --check` | ✅ | ⬜ pending |
| 03-02-01 | 02 | 2 | DOC-04 | manual | `grep -c "watch" README.md` | ✅ | ⬜ pending |
| 03-03-01 | 03 | 3 | QUAL-01 | manual | `ls .planning/phases/67-prog-diag/67-VALIDATION.md` | ❌ Wave 0 | ⬜ pending |
| 03-03-02 | 03 | 3 | QUAL-01 | manual | `ls .planning/phases/68-init-wizard/68-VALIDATION.md` | ❌ Wave 0 | ⬜ pending |
| 03-03-03 | 03 | 3 | QUAL-01 | manual | `ls .planning/phases/69-watch/69-VALIDATION.md` | ❌ Wave 0 | ⬜ pending |
| 03-03-04 | 03 | 3 | QUAL-01 | manual | `ls .planning/phases/70-watch/70-VALIDATION.md` | ❌ Wave 0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] VALIDATION.md 文件目标（Phase 67/68/69/70），由 Plan 03 创建
- ✅ `src/cli/opts.rs`（已存在，Plan 01 修改）
- ✅ `README.md`（已存在，Plan 02 修改）

*Wave 0 状态：opts.rs 和 README.md 文件已存在，无需预建。4 个 VALIDATION.md 文件由 Plan 03 新建。*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| VALIDATION.md frontmatter 字段正确 | QUAL-01 | 纯文档验证 | 确认 phase/slug/status/nyquist_compliant 字段存在且值合理 |
| README watch 内容完整 | DOC-04 | 内容质量判断 | 确认 watch 用法示例、init --interactive 说明、--quiet/--verbose 说明均存在 |
| --help 示例格式正确 | DOC-05 | 输出格式验证 | 运行 `cargo run -- watch --help` 和 `cargo run -- validate --help` 确认有 2+ 示例 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Wave 0 covers all MISSING references (only docs, no new test stubs needed)
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
