---
phase: "65"
slug: 65-parity
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-04
---

# Phase 65 — Validation Strategy

> Per-phase validation contract for Parallel Verbose Parity（PARALLEL-03/04/05 + IO-01）

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
| 65-01-01 | 01 | 1 | PARALLEL-05 | T-65-01 | verbose=false 时不输出路径（无信息泄露） | integration | `cargo test test_cli_verbose_parallel_prints_processing_lines` | ✅ | ✅ green |
| 65-01-01 | 01 | 1 | PARALLEL-05 | — | verbose=true 多文件 → ≥2 条 "Processing: " 行到 stderr | integration | `cargo test test_cli_verbose_parallel_prints_processing_lines` | ✅ | ✅ green |
| 65-01-02 | 01 | 1 | PARALLEL-03 | — | 并行路径过滤等价顺序路径（运行时验证在 Phase 66） | integration | `cargo test test_parallel_csv_filter_matches_sequential` | ✅ | ✅ green |
| 65-01-02 | 01 | 1 | PARALLEL-04 | — | 并行路径 CSV 导出等价顺序路径（运行时验证在 Phase 66） | integration | `cargo test test_parallel_csv_content_matches_sequential` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

Note: test_cli_verbose_parallel_prints_processing_lines was added by validate-phase audit (2026-06-04) to fill PARALLEL-05 gap. PARALLEL-03/04 runtime verification tests were added in Phase 66.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| IO-01: mmap 读取无 BufReader | IO-01 | 架构属性，无法用测试断言 mmap vs BufReader 路径选择 | 阅读 `dm-database-parser-sqllog` crate 依赖，确认 `memmap2::Mmap` 存在；检查 `src/cli/run/parallel.rs` 注释中的 IO-01 分析记录 |

---

## Validation Audit 2026-06-04

| Metric | Count |
|--------|-------|
| Gaps found | 1 |
| Resolved | 1 |
| Escalated | 0 |

**Gap resolved:** PARALLEL-05 — 新增 `test_cli_verbose_parallel_prints_processing_lines`（tests/integration.rs:1048）验证多文件 verbose 并行路径逐文件输出。

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-06-04
