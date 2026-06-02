---
phase: 56
slug: stats-benchmark
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-02
---

# Phase 56 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test && cargo clippy --all-targets -- -D warnings` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo test && cargo clippy --all-targets -- -D warnings`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 56-01-01 | 01 | 1 | CLEAN-01 | — | N/A | grep | `grep -rn "warn!" src/cli/stats/mod.rs && exit 1 || echo "no warn"` | ✅ | ⬜ pending |
| 56-01-02 | 01 | 1 | CLEAN-01 | — | N/A | unit | `cargo test` | ✅ | ⬜ pending |
| 56-02-01 | 02 | 2 | CLEAN-01 | — | N/A | compile | `cargo build` | ✅ | ⬜ pending |
| 56-02-02 | 02 | 2 | CLEAN-01 | — | N/A | unit | `cargo test` | ✅ | ⬜ pending |
| 56-03-01 | 03 | 3 | BENCH-01 | — | N/A | manual | `ls -la scripts/collect_bench_results.sh` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| benches/BENCHMARKS.md 新节内容完整 | BENCH-01 | 文档质量无法自动化 | 阅读 benches/BENCHMARKS.md，确认包含 artifact 下载说明、JSON 结构、手动对比方法 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
