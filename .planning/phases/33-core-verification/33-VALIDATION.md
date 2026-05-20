---
phase: 33
slug: core-verification
status: ready
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-20
---

# Phase 33 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust built-in) |
| **Config file** | `Cargo.toml` |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo build --release && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 33-01-01 | 01 | 1 | KEEP-06 | — | N/A | build | `cargo check` | ✅ | ⬜ pending |
| 33-01-02 | 01 | 1 | KEEP-06 | — | N/A | build | `cargo build --release` | ✅ | ⬜ pending |
| 33-01-03 | 01 | 1 | KEEP-06 | — | N/A | lint | `cargo clippy --all-targets -- -D warnings` | ✅ | ⬜ pending |
| 33-01-04 | 01 | 1 | KEEP-06 | — | N/A | format | `cargo fmt --check` | ✅ | ⬜ pending |
| 33-02-01 | 02 | 1 | KEEP-01~05 | — | N/A | test | `cargo test` | ✅ | ⬜ pending |
| 33-02-02 | 02 | 1 | KEEP-01~05 | — | N/A | bench | `cargo bench` | ✅ | ⬜ pending |
| 33-03-01 | 03 | 1 | KEEP-01~05 | — | N/A | smoke | `bash smoke_test/run_all.sh` (asset creation) | ❌ W0 | ⬜ pending |
| 33-03-02 | 03 | 1 | KEEP-01~05 | — | N/A | smoke | `bash smoke_test/run_all.sh` (execution) | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `smoke_test/run_all.sh` — 冒烟测试编排脚本 (Plan 3, Task 1)
- [ ] `smoke_test/config_*.toml` — 10 个测试场景配置文件 (Plan 3, Task 1)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| CSV 导出端到端功能 | KEEP-01 | 需要真实 DaMeng 日志文件 | `cargo run -- run -c configs/smoke-csv.toml` 并验证 CSV 输出 |
| SQLite 导出端到端功能 | KEEP-02 | 需要真实 DaMeng 日志文件 | `cargo run -- run -c configs/smoke-sqlite.toml` 并验证 SQLite 输出 |
| 过滤器功能正确性 | KEEP-03 | 需要针对性配置和场景 | 分别对 include/exclude/indicators/sql 四类过滤器运行并验证 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
