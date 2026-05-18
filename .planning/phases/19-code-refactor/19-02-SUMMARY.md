---
phase: 19-code-refactor
plan: 02
subsystem: config
tags:
  - rust
  - refactor
  - module-split
  - visibility
  - config
dependency_graph:
  requires: []
  provides:
    - src/config/validate.rs (Config 验证逻辑独立模块)
    - src/config/apply_one.rs (Config 覆盖逻辑独立模块)
    - src/config/mod.rs (精简后的模块入口)
  affects:
    - src/config/mod.rs
    - src/config/validate.rs
    - src/config/apply_one.rs
tech_stack:
  added: []
  patterns:
    - Rust child module splitting via `mod submodule;` declarations
    - Private module access via `use super::Const` pattern
key_files:
  created:
    - src/config/validate.rs
    - src/config/apply_one.rs
  modified:
    - src/config/mod.rs
decisions:
  - "validate_and_compile 保持 pub（非 pub(crate)），因调用方在 binary crate (main.rs)"
  - "apply_overrides 保持 pub，原因同上"
  - "apply_overrides 测试保留在 mod.rs；apply_one 测试留在 apply_one.rs（私有方法只能在声明模块测试）"
  - "pipeline_deprecated 字段保持 pub（validate.rs 子模块通过 self 访问，但 struct 字段可见性规则要求 pub）"
metrics:
  duration: "~45 分钟（含 context 恢复）"
  completed: "2026-05-18"
  tasks_completed: 3
  files_modified: 3
---

# Phase 19 Plan 02: Split config/mod.rs Into Sub-modules Summary

将 1418 行的 `src/config/mod.rs` 拆分为三个职责独立的文件，消除巨型文件反模式，保持所有 外部 API 路径不变。

## What Was Built

- `src/config/apply_one.rs` — 新建文件，包含 `apply_overrides`/`apply_one` 实现及 18 个 apply_one 私有方法测试
- `src/config/validate.rs` — 新建文件，包含 `validate`/`validate_and_compile`/私有 `validate_*` 方法及 94 个测试
- `src/config/mod.rs` — 精简为 286 行，仅保留 Config struct + from_file + 子模块声明 + re-export + apply_overrides 测试

## Tasks Completed

| Task | Description | Commit | Status |
|------|-------------|--------|--------|
| 1 | 抽取 apply_overrides/apply_one 到 apply_one.rs | bc7aa43 | Done |
| 2 | 抽取 validate/validate_and_compile/validate_* 到 validate.rs | f517529 | Done |
| 3 | 最终验证（mod.rs 已在 Task 2 后满足 ≤300 行，无需修改） | — (no change) | Done |

## Verification Results

- `cargo test`: 442 lib tests + 55 integration tests = 497 tests, 0 failed
- `cargo clippy --all-targets -- -D warnings`: 0 warnings
- `cargo fmt --check`: passes

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] validate_and_compile 误用 pub(crate)**
- **Found during:** Task 2 commit pre-hook (clippy)
- **Issue:** 计划指定 `pub(crate)` 但调用方在 `src/main.rs`（binary crate），导致 lib crate 报 `dead_code`
- **Fix:** 改为 `pub fn validate_and_compile`
- **Files modified:** `src/config/validate.rs`
- **Commit:** f517529

**2. [Rule 1 - Bug] apply_overrides 误用 pub(crate)**
- **Found during:** Task 1 commit pre-hook (clippy)
- **Issue:** 同上，调用方在 binary crate main.rs，pub(crate) 报 dead_code
- **Fix:** 改为 `pub fn apply_overrides`
- **Files modified:** `src/config/apply_one.rs`
- **Commit:** bc7aa43

**3. [Rule 1 - Bug] cargo fmt 模块声明顺序**
- **Found during:** Task 1 & Task 2 commits pre-hook (cargo fmt)
- **Issue:** cargo fmt 要求私有 `mod` 声明排在所有 `pub mod` 声明之后（同 Task 1 遇到的顺序问题重现）
- **Fix:** 将 `mod validate;` 移到 `pub mod sqllog;` 之后
- **Files modified:** `src/config/mod.rs`

### Line Count Deviations (Plan Estimate vs Actual)

| File | Plan Target | Actual | Reason |
|------|-------------|--------|--------|
| src/config/mod.rs | ≤300 | 286 | ✓ 达标 |
| src/config/apply_one.rs | ≤300 | 354 | 计划低估测试密度：18 个测试 × ~9 行 = ~163 行测试代码 |
| src/config/validate.rs | ≤300 | 803 | 计划严重低估：94 个测试函数（平均 6 行/个 = 564 行），实现代码 ~165 行 |

**根因：** 计划的 max_lines: 300 是基于"代码行数"估算，忽略了同文件测试数量。validate 逻辑是 D-10 规范要求"测试与实现共存于同模块"，导致 validate.rs 的测试体积远超预期。

## Known Stubs

None — 所有功能正常连接。

## Threat Flags

None — 此为纯代码重组，无新的网络端点、auth 路径或 schema 变更。

## Self-Check: PASSED

- `src/config/apply_one.rs`: FOUND
- `src/config/validate.rs`: FOUND
- `src/config/mod.rs`: FOUND (286 lines)
- Commit bc7aa43: FOUND (Task 1)
- Commit f517529: FOUND (Task 2)
