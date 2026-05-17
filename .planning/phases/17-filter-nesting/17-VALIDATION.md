---
phase: 17
slug: filter-nesting
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-17
---

# Phase 17 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + cargo test |
| **Config file** | Cargo.toml（[dev-dependencies] tempfile = "3.27.0"） |
| **Quick run command** | `cargo test --lib features::filters` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** `cargo clippy --all-targets -- -D warnings && cargo test --lib`
- **After every plan wave:** `cargo test`
- **Before `/gsd:verify-work`:** `cargo test` 全绿 + `cargo run -- validate -c config.toml` 通过
- **Max feedback latency:** ~5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 17-01-01 | 01 | 1 | CONFIG-01 | — | N/A | unit | `cargo test --lib features::filters::tests::test_new_nested_format_include` | ❌ W0 | ⬜ pending |
| 17-01-02 | 01 | 1 | CONFIG-02 | — | N/A | unit | `cargo test --lib features::filters::tests::test_new_nested_format_exclude` | ❌ W0 | ⬜ pending |
| 17-01-03 | 01 | 1 | CONFIG-05 | — | N/A | unit | `cargo test --lib features::filters::tests::test_backward_compat_flat_format` | ❌ W0 | ⬜ pending |
| 17-01-04 | 01 | 1 | CONFIG-05 | — | N/A | unit | `cargo test --lib features::filters::tests::test_sql_filters_alias_backward_compat` | ❌ W0 | ⬜ pending |
| 17-02-01 | 02 | 2 | CONFIG-01 | — | N/A | unit | `cargo test --lib` | ✅ | ⬜ pending |
| 17-02-02 | 02 | 2 | CONFIG-05 | — | N/A | unit | `cargo test` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/features/filters.rs` — 新增 `test_backward_compat_flat_format`：旧扁平格式 TOML → parse → FiltersFeature，验证字段值映射到 include/exclude 正确
- [ ] `src/features/filters.rs` — 新增 `test_new_nested_format_include`：新格式 include 子表 TOML → parse → IncludeFilters 各字段正确
- [ ] `src/features/filters.rs` — 新增 `test_new_nested_format_exclude`：新格式 exclude 子表 TOML → parse → ExcludeFilters 各字段正确
- [ ] `src/features/filters.rs` — 新增 `test_sql_filters_alias_backward_compat`：旧 `include_patterns` / `exclude_patterns` 字段名仍可 parse
- [ ] 更新现有 `test_filters_toml_deserialization_with_trxids_and_exec_ids`：将 `filters.meta.trxids` 改为 `filters.include.trxids`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `cargo run -- validate -c config.toml` 旧格式通过 | CONFIG-05 | 需要真实 config 文件运行 | `cargo run -- validate -c config.toml` 输出 "Config validated successfully" |
| `cargo run -- init -o /tmp/test.toml --force` 生成新嵌套格式 | CONFIG-01/02 | 需要 CLI 运行检查模板输出 | `cat /tmp/test.toml` 确认包含 `[features.filter.include]` 子表 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
