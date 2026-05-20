---
phase: 29-remove-stats-digest
plan: 02
type: execute
subsystem: cli
tags: [digest, fingerprint, normalize-template, serde_json]
requires: [29-01]
provides: [RM-04]
affects: [pipeline, cli]
tech-stack:
  added: []
  patterns: []
key-files:
  created: []
  modified:
    - src/pipeline/normalizer.rs
    - src/pipeline/mod.rs
    - src/cli/mod.rs
    - src/cli/opts.rs
    - src/main.rs
    - src/lang.rs
    - Cargo.toml
    - Cargo.lock
    - tests/integration.rs
  deleted:
    - src/cli/digest.rs
    - src/pipeline/fingerprint.rs
decisions: []
metrics:
  duration: ~15 min
  completed_date: "2026-05-20"
  tasks_completed: 2
  files_modified: 9
  files_deleted: 2
  tests_passed: 334+352+40
---

# Phase 29 Plan 02: 移除 digest 命令、fingerprint.rs 和 serde_json

迁移 `normalize_template()` 从 `fingerprint.rs` 到 `normalizer.rs`，删除整个 digest 子命令模块及其依赖的文件和库。

## Summary

将 fingerprint.rs 中的 SQL 模板归一化函数迁移到 normalizer.rs，同时删除不再需要的 digest 子命令和 serde_json 依赖。迁移涉及简化函数签名（删除 ScanMode 枚举、Fingerprint 分支、keep_literal 参数），保留模板管道功能不受影响。

## Tasks Executed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | 迁移 normalize_template 到 normalizer.rs | `9e86afe` | src/pipeline/normalizer.rs, src/pipeline/mod.rs |
| 2 | 移除 digest 命令、fingerprint.rs 和 serde_json | `eabe4f3` | 11 files (2 deleted, 9 modified) |

## Commits

- `9e86afe`: feat(29-remove-stats-digest): migrate normalize_template from fingerprint.rs to normalizer.rs
- `eabe4f3`: feat(29-remove-stats-digest): remove digest subcommand, fingerprint.rs, and serde_json

## Verification Results

- `cargo build`: 编译成功，无警告
- `cargo test`: 334 lib + 352 integration + 40 doc = 726 tests 全部通过
- `cargo clippy --all-targets -- -D warnings`: 无警告
- `cargo fmt --check`: 格式正确
- `cargo test --lib pipeline::normalizer::tests`: 36 tests全部通过（含 8 个 `#[test]` + 2 个 proptest）

## Deviations from Plan

None - plan executed exactly as written.

## Key Changes

### normalizer.rs（函数迁移与简化）

从 fingerprint.rs 迁移了以下函数到 normalizer.rs，并进行简化：
- `NEEDS_SPECIAL_NORM` - const 常量（原样迁移）
- `normalize_template` - 导出函数（删除 ScanMode 参数）
- `scan_sql_bytes` - 简化版（删除 mode 参数）
- `dispatch_byte` - 简化版（删除 Fingerprint 分支和 mode 参数）
- `handle_quote` - 简化版（始终 keep_literal=true，删除参数）
- 辅助函数全部原样迁移：`handle_line_comment`、`handle_block_comment`、`handle_word`、`try_fold_in_list`、`skip_quoted`、`is_subquery`、`is_keyword`、`is_ident_byte`、`prev_is_ident_byte`

### 删除的文件

- `src/cli/digest.rs` - 完整的 digest 子命令实现（包括 FingerprintAccumulator、DigestEntry、handle_digest 等）
- `src/pipeline/fingerprint.rs` - SQL 指纹/模板归一化模块（含 ~130 行测试）

### 依赖移除

- `serde_json = "1.0.149"` 从 Cargo.toml 中删除（已无使用者）
- `Cargo.lock` 自动更新

## Post-Commit Verification

```bash
# 验证 digest 子命令不再存在
grep -r "pub mod digest" src/cli/mod.rs  # → 无输出
grep -r "Commands::Digest" src/main.rs   # → 无输出
grep -r "zh_digest" src/lang.rs          # → 无输出
grep -r "serde_json" Cargo.toml          # → 无输出（依赖声明已删除）
grep -r "use fingerprint" src/pipeline/mod.rs  # → 无输出
```
