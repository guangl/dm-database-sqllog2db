---
phase: 72
slug: bench-baseline
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-08
---

# Phase 72 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo clippy --all-targets -- -D warnings && cargo test` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo clippy --all-targets -- -D warnings && cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** ~30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 72-01-01 | 01 | 1 | BENCH-01 | — | N/A | manual + doc | `hyperfine --warmup 3 './target/release/sqllog2db --version'` → output recorded in BENCHMARKS.md | ✅ | ⬜ pending |
| 72-02-01 | 02 | 1 | BENCH-02 | — | N/A | manual + doc | `CRITERION_HOME=benches/baselines cargo bench -- --save-baseline v1.20` → baselines/ 下出现 v1.20 目录 | ✅ | ⬜ pending |
| 72-02-02 | 02 | 1 | BENCH-02 | — | N/A | automated | `ls benches/baselines/*/v1.20/benchmark.json \| wc -l` → ≥1 | ✅ | ⬜ pending |
| 72-03-01 | both | 2 | BENCH-01/02 | — | N/A | automated | `cargo clippy --all-targets -- -D warnings && cargo test` exits 0 | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

*Existing infrastructure covers all phase requirements.* No new test files needed — phase is documentation + command execution only. `cargo test` 已有 909 个测试覆盖现有功能，本 phase 不引入代码变更。

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| hyperfine 冷启动延迟测量 | BENCH-01 | 依赖本地环境运行时间，不适合 CI 自动化断言 | 1. `cargo build --release`; 2. `hyperfine --warmup 3 './target/release/sqllog2db --version'`; 3. 记录 mean ± σ 到 BENCHMARKS.md |
| criterion v1.20 baseline 存档 | BENCH-02 | 需要实际运行 bench 并提交文件到 repo | 1. `CRITERION_HOME=benches/baselines cargo bench -- --save-baseline v1.20`; 2. 验证 `benches/baselines/csv_export/v1.20/` 等目录存在 JSON 文件; 3. `git add benches/baselines/*/v1.20/ && git commit` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
