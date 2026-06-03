---
phase: 59
slug: cli-run-exporter-pipeline
status: validated
nyquist_compliant: false
wave_0_complete: true
created: 2026-06-03
audited: 2026-06-03
---

# Phase 59 — cli/run 与 exporter/pipeline 结构整理 Validation Strategy

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (built-in) |
| **Config file** | `Cargo.toml` |
| **Quick run command** | `cargo test --lib --quiet` |
| **Full suite command** | `cargo test --quiet` |
| **Estimated runtime** | ~60 seconds (638 tests) |

---

## Sampling Rate

- **After every task commit:** `cargo test --lib --quiet`
- **After every plan wave:** `cargo test --quiet`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** ~60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Behavior | Test Type | Automated Command | Status |
|---------|------|------|-------------|------------|----------|-----------|-------------------|--------|
| 59-01-T1 | 01 | 1 | STRUCT-01 | T-59-01 | `normalize_and_export` 返回 ExportAction；params_buffer 借用不被破坏 | unit | `cargo test --lib "cli::run::tests::test_normalize_and_export_filtered_params_updates_buffer"` | ✅ green |
| 59-01-T2 | 01 | 1 | STRUCT-01 | T-59-01 | `normalize_and_export` BreakQuota：remaining=Some(0) 时不导出 | unit | `cargo test --lib "cli::run::tests::test_normalize_and_export_quota_hit_returns_break_quota"` | ✅ green |
| 59-01-T3 | 01 | 1 | STRUCT-01 | T-59-02 | process_log_file 顺序导出行为不变（拆分后） | integration | `cargo test --lib "cli::run::tests::test_handle_run_default_config_succeeds"` | ✅ green |
| 59-02-T1 | 02 | 1 | STRUCT-02 | T-59-03 | collector.rs 新建；sqlite_parallel.rs 调用 super::collector | integration | `cargo test --lib "cli::run::tests::test_sqlite_parallel_matches_sequential"` | ✅ green |
| 59-02-T2 | 02 | 1 | STRUCT-02 | T-59-04 | PARAMS buffer 在 collector 中正确更新（normalized_sql 正确） | integration | `cargo test --lib "cli::run::tests::test_sqlite_parallel_matches_sequential"` | ✅ green |
| 59-03-T1 | 03 | 1 | STRUCT-01 | T-59-05 | run_sequential 调用 run_file_loop；行为不变 | integration | `cargo test --lib "cli::run::tests::test_handle_run_default_config_succeeds"` | ✅ green |
| 59-03-T2 | 03 | 1 | STRUCT-01 | T-59-06 | build_include_groups / build_exclude_groups 字段顺序正确 | unit | `cargo test --lib "cli::run::filter_processor::tests"` | ✅ green |
| 59-04-T1 | 04 | 2 | STRUCT-01/02 | — | process_csv_parallel ≤40 行骨架；CSV 并行调用 collector | integration | `cargo test --lib "cli::run::tests::test_parallel_merge_consistent"` | ✅ green |
| 59-05/06 | 05/06 | gap | STRUCT-01 | — | normalize_and_export ≤40 行；parallel_collect ≤40 行 | integration | `cargo test --lib --quiet` | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `run_file_loop` fatal 错误时终止剩余文件处理 | STRUCT-01 | 需要构造 fatal 导出错误（ExporterManager 无 mock 支持） | 手动配置一个权限受限的 CSV 目录，触发写入失败，确认剩余文件未被处理 |

---

## Validation Audit 2026-06-03

| Metric | Count |
|--------|-------|
| Gaps found | 3 |
| Resolved (automated) | 2 |
| Escalated to manual | 1 |

### Gaps Found

| Gap | Status | Resolution |
|-----|--------|------------|
| `normalize_and_export` passes=false buffer-only 路径无单元测试 | RESOLVED | 新增 `test_normalize_and_export_filtered_params_updates_buffer`（tests.rs:261） |
| `normalize_and_export` BreakQuota 路径无单元测试 | RESOLVED | 新增 `test_normalize_and_export_quota_hit_returns_break_quota`（tests.rs:340） |
| `run_file_loop` fatal 错误早退出无直接自动化测试 | MANUAL-ONLY | 需要 ExporterManager mock；标记为手动 |

### Post-Gap Verification

```
cargo test --lib "cli::run::tests::test_normalize_and_export_filtered_params_updates_buffer"
→ ok. 2 passed (with quota test)

cargo clippy --all-targets -- -D warnings
→ 零警告
```

---

## Validation Sign-Off

- [x] All tasks have automated verify or Manual-Only justification
- [x] No 3 consecutive tasks without automated verify
- [x] 2 MISSING gaps resolved with new tests
- [x] No watch-mode flags
- [x] Feedback latency ≤60s
- [ ] `nyquist_compliant: true` — pending: 1 manual-only item (run_file_loop fatal path)

**Approval:** approved 2026-06-03 (partial — 1 manual-only item)
