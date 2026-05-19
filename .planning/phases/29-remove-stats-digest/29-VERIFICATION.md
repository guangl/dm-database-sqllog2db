---
phase: 29-remove-stats-digest
verified: 2026-05-20T10:00:00Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
---

# Phase 29: Verification Report

**Phase Goal:** 移除 stats 和 digest 两个子命令及其相关依赖和文件
**Verified:** 2026-05-20
**Status:** passed
**Re-verification:** No (initial verification)

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `sqllog2db --help` 不再显示 `stats` 子命令 | ✓ VERIFIED | `cargo run -- --help` 输出仅显示：run, init, validate, show-config, help—无 stats |
| 2 | `sqllog2db --help` 不再显示 `digest` 子命令 | ✓ VERIFIED | 同上输出，无 digest |
| 3 | `src/cli/stats.rs` 和 `src/cli/digest.rs` 文件已移除 | ✓ VERIFIED | `test -f` 均返回文件不存在 |
| 4 | `src/pipeline/fingerprint.rs` 已移除 | ✓ VERIFIED | `test -f` 返回文件不存在 |
| 5 | `serde_json` 依赖从 Cargo.toml 中删除 | ✓ VERIFIED | `grep serde_json Cargo.toml` 无输出 |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | --------- | ------ | ------- |
| `src/cli/stats.rs` | 文件已删除 | ✓ VERIFIED | 文件不存在 |
| `src/cli/digest.rs` | 文件已删除 | ✓ VERIFIED | 文件不存在 |
| `src/pipeline/fingerprint.rs` | 文件已删除 | ✓ VERIFIED | 文件不存在 |
| `src/cli/mod.rs` | 无 `pub mod stats` | ✓ VERIFIED | 无引用 |
| `src/cli/opts.rs` | 无 Stats/Digest variant | ✓ VERIFIED | `grep -n "Stats\|Digest"` 无输出 |
| `src/main.rs` | 无 Stats/Digest match arm | ✓ VERIFIED | `grep -n "Stats\|Digest"` 无输出 |
| `src/lang.rs` | 无 `zh_stats`/`zh_digest` | ✓ VERIFIED | `grep -n "zh_stats\|zh_digest"` 无输出 |
| `src/pipeline/normalizer.rs` | 包含 `pub fn normalize_template` | ✓ VERIFIED | 第 462 行 |
| `src/pipeline/mod.rs` | 从 normalizer 导出 | ✓ VERIFIED | 第 8 行 `pub(crate) use normalizer::normalize_template;` |
| `Cargo.toml` | 无 `serde_json` 依赖声明 | ✓ VERIFIED | `grep serde_json Cargo.toml` 无输出 |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | --- | --- | ------ | ------- |
| `processor.rs:138` | `pipeline/mod.rs` | `crate::pipeline::normalize_template(pm.sql.as_ref())` | ✓ WIRED | 第 138 行调用，路径解析正常 |
| `aggregator.rs` | `pipeline/mod.rs` | 文档注释引用 normalize_template | ✓ DOC-ONLY | 仅文档注释，不影响编译 |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| `normalizer.rs::normalize_template` | sql: &str | via processor.rs from parsed log records | ✓ FLOWING | 36 个测试通过（含 8 个 `#[test]` + 2 个 proptest），processor.rs:138 在生产路径调用 |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| CLI help 无 stats | `cargo run -- --help 2>&1 | grep stats` | 无输出 | ✓ PASS |
| CLI help 无 digest | `cargo run -- --help 2>&1 | grep digest` | 无输出 | ✓ PASS |
| Build 成功 | `cargo build 2>&1` | 成功 | ✓ PASS |
| Clippy 无警告 | `cargo clippy --all-targets -- -D warnings 2>&1` | 成功 | ✓ PASS |
| 所有测试通过 | `cargo test 2>&1` | 334 lib + 40 integration = 374 passed | ✓ PASS |
| normalize_template 测试 | `cargo test --lib pipeline::normalizer::tests 2>&1` | 36 passed | ✓ PASS |
| 格式检查 | `cargo fmt --check 2>&1` | 无输出 | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ---------- | ----------- | ------ | -------- |
| RM-03 | 29-01-PLAN.md | 移除 stats 统计命令 | ✓ SATISFIED | stats.rs 已删除，所有引用已清理，编译无问题 |
| RM-04 | 29-02-PLAN.md | 移除 digest 摘要命令 | ✓ SATISFIED | digest.rs、fingerprint.rs 已删除，serde_json 依赖已移除，normalize_template 正常迁移到 normalizer.rs |

### Anti-Patterns Found

无。所有扫描结果为空：
- 无 `TBD`/`FIXME`/`XXX` 债务标记
- 无 placeholder/coming soon 等存根标记
- 无空实现（`return null` 等）
- 无硬编码空数据

### Human Verification Required

无。所有验证可自动化完成。

---

## Summary

Phase 29 目标完全实现。具体成果：

1. **stats 子命令已移除**：`src/cli/stats.rs` 和所有引用（`cli/mod.rs`、`cli/opts.rs`、`main.rs`、`lang.rs`、`tests/integration.rs`）已清理。
2. **digest 子命令已移除**：`src/cli/digest.rs`、`src/pipeline/fingerprint.rs` 和所有引用已清理。
3. **normalize_template 已迁移**：从 `fingerprint.rs` 迁移到 `normalizer.rs`，简化了 `ScanMode` 枚举和 `keep_literal` 参数。processor.rs 的调用路径 `crate::pipeline::normalize_template` 保持不变。
4. **serde_json 依赖已移除**：从 Cargo.toml 删除。
5. **全链路编译/测试/lint/fmt 验证通过**：
   - `cargo build` 编译成功
   - `cargo clippy --all-targets -- -D warnings` 无警告
   - `cargo test`：334 lib + 40 integration = 374 测试全部通过
   - `cargo fmt --check` 格式正确

---

_Verified: 2026-05-20_
_Verifier: Claude (gsd-verifier)_
