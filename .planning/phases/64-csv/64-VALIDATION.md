---
phase: "64"
slug: 64-csv
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-04
---

# Phase 64 — Validation Strategy

> Per-phase validation contract for CSV 并行路径验证（质量门禁 + REQUIREMENTS.md 对齐）

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust standard test harness) |
| **Config file** | Cargo.toml |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 64-01-01 | 01 | 1 | PARALLEL-01 | — | SC1: 多文件 CSV 自动走并行路径 | integration | `cargo test test_handle_run_parallel_csv_multiple_files` | ✅ | ✅ green |
| 64-01-01 | 01 | 1 | PARALLEL-01 | — | SC4: 单文件回退顺序路径 | unit | `cargo test test_parallel_merge_consistent` | ✅ | ✅ green |
| 64-01-02 | 01 | 1 | PARALLEL-02 | — | REQUIREMENTS.md 含 "temp-file" 不含 channel 功能要求 | docs-grep | `grep -c "temp-file" .planning/REQUIREMENTS.md` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| SC2: 无全量内存缓冲（Vec 在 write_records_to_csv 写完后立即 drop） | PARALLEL-02 | 内存释放时机无法用自动化断言捕获 | 阅读 `src/cli/run/parallel.rs` `fn write_records_to_csv` 签名，确认 `rows: Vec<...>` 按值传入，函数返回后 drop |
| SC3: 峰值内存 ≤ 2× 单线程 | PARALLEL-02 | 无内存基准测试要求（ROADMAP 未规定） | 理论分析：rayon work-stealing 保证最多 jobs 个线程并行，jobs=2 时 ≤ 2× 单文件 Vec 大小 |

---

## Validation Audit 2026-06-04

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
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-06-04
