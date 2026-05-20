---
phase: 32-cleanup-project-structure
plan: 01
type: execute
subsystem: core-structure
tags: [cleanup, residual-removal, structural-cleanup]
requires: [Phase 28, Phase 29, Phase 30, Phase 31]
provides: [clean-module-decls, clean-pipeline-types, clean-config-struct, clean-cargotoml]
affects: [src/config/mod.rs, src/config/validate.rs, tests/integration.rs, Cargo.toml]
tech-stack:
  added: []
  patterns: []
key-files:
  created: []
  modified:
    - src/config/mod.rs
    - src/config/validate.rs
    - tests/integration.rs
    - Cargo.toml
    - Cargo.lock
decisions: []
metrics:
  duration: "~5 min"
  completed: "2026-05-20"
---

# Phase 32 Plan 01: 模块声明和配置层残留清理

**One-liner:** 清理 Phase 28-31 删除功能后在模块声明、Config 结构体和 Cargo.toml 中遗留的结构性残留，包括 PIPELINE_MIGRATION_HINT 中的 charts 迁移提示和未用的 ryu 依赖。

## Tasks

### Task 1: 删除 lib.rs/main.rs/cli/mod.rs 的 stale mod 声明

**Status:** Completed (no changes needed)

Src/lib.rs、src/main.rs、src/cli/mod.rs 在 Phase 28-31 中已清理完毕，当前无 charts、resume、digest、stats、update 模块声明。

**Proof points:**
- src/lib.rs: 8 行，无 charts/resume
- src/main.rs: 无 charts/resume 模块声明
- src/cli/mod.rs: 6 行，仅含 init/opts/preflight/run/show_config/validate

### Task 2: 删除 pipeline/mod.rs 中已移除模块的类型、函数和测试

**Status:** Completed (no changes needed)

Src/pipeline/mod.rs 在 Phase 28-31 中已清理完毕，当前无 fingerprint、aggregator、template_reporter 模块声明，无 ChartsConfig/TemplateConfig/TemplateReportConfig 类型定义，无 charts/template 相关测试。

### Task 3: 删除 Config 残留字段 + validate_charts() + Cargo.toml 未用依赖 + PIPELINE_MIGRATION_HINT

**Status:** Completed

**Changes made:**
1. **src/config/mod.rs**: 从 `PIPELINE_MIGRATION_HINT` 中删除 `[pipeline.charts] → [charts]` 行
2. **src/config/validate.rs**: 删除 `test_validate_legacy_pipeline_path_rejected` 中检查 charts 迁移提示的断言
3. **tests/integration.rs**: 删除 2 个测试函数中检查 charts 迁移提示的断言
4. **Cargo.toml**: 删除未用的 `ryu = "1"` 依赖
5. **Cargo.lock**: 自动更新（ryu 移除后的 lockfile 变更）

**Verified:**
- `cargo build` 编译成功
- `cargo test` 通过 (285 unit tests + 36 integration tests)
- `cargo clippy --all-targets` 无警告
- `cargo fmt --check` 格式正确

## Verification

- cargo build: 编译成功，无错误
- 所有已删除功能模块的 mod 声明在 lib.rs/main.rs/cli/mod.rs 中已移除 (pre-existing)
- pipeline/mod.rs 无 charts/template/fingerprint/aggregator 残留 (pre-existing)
- Config 结构体无 resume/template/charts 字段 (pre-existing)
- validate.rs 无 validate_charts() 方法 (pre-existing)
- Cargo.toml 无已移除功能的依赖，包括 ryu (已删除)

## Deviations from Plan

### Already Cleaned by Previous Phases

Tasks 1 和 2 的核心清理工作已在 Phase 28-31 中完成。本 Plan 实际修改量小于 Plan 中描述的预期，因为之前的 Phase 已在文件删除过程中同步清理了模块声明和类型定义。

### Integration Tests Update

删除 `PIPELINE_MIGRATION_HINT` 中的 charts 行后，额外的 2 个集成测试也需要更新断言。这些测试不在 Plan 的 Task 3 Cargo.toml/validate.rs 文件列表范围内，但在 tests/integration.rs 中，属于 Rule 3 自动修复 (auto-fix blocking issue — 否则 cargo test 失败)。

**Files:**
- `tests/integration.rs`: 删除 2 处 `err_msg.contains("[pipeline.charts] -> [charts]")` 断言

## Commit

- `d0136e7`: chore(32-01): remove residual charts migration hint and unused ryu dependency

## Self-Check: PASSED

- src/config/mod.rs: PIPELINE_MIGRATION_HINT 无 charts 引用 ✓
- src/config/validate.rs: 无 charts 断言 ✓
- tests/integration.rs: 无 charts 断言 ✓
- Cargo.toml: 无 ryu 依赖 ✓
- `cargo build`: 成功 ✓
- Commit hash: d0136e7 ✓
