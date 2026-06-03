---
phase: 60-error-handling
plan: "01"
subsystem: error-handling
tags:
  - rust
  - error-handling
  - thiserror
  - refactor
dependency_graph:
  requires: []
  provides:
    - STRUCT-03 Phase 60 成功标准 1（unwrap/expect 全部可解释）
    - STRUCT-03 Phase 60 成功标准 2（map_err 全部审计）
    - 60-AUDIT.md（四条成功标准兜底验证文档）
  affects:
    - src/logging.rs
    - src/cli/run/parallel.rs
tech_stack:
  added: []
  patterns:
    - infallible 注释：在 .unwrap()/.expect() 同行或前置行说明不可失败原因
    - D-01 map_err 保留：携带 path/reason 上下文或 rayon 错误中转模式
key_files:
  created:
    - .planning/phases/60-error-handling/60-AUDIT.md
  modified:
    - src/logging.rs
    - src/cli/run/parallel.rs
decisions:
  - "仅添加注释，不修改任何非注释代码字节（功能行为完全不变）"
  - "normalizer.rs:310 为 #[cfg(test)] 内函数（test_code），normalizer.rs:418 已有 debug_assert + 注释（production_commented），均无需新增注释"
metrics:
  duration: "~10 分钟"
  completed: "2026-06-03"
  tasks_completed: 3
  files_changed: 3
---

# Phase 60 Plan 01: 错误处理路径统一 — infallible 注释 + 全代码库审计

满足 STRUCT-03 成功标准：为生产代码中每个 unwrap/expect 添加 infallible 注释，审计所有 map_err 保留合理性，cargo 全套工具链绿。

## Summary

两处生产代码 infallible 注释已添加（logging.rs:60、parallel.rs:280-281），60-AUDIT.md 交付四条成功标准全勾选，代码行为完全不变。

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | logging.rs:60 write! unwrap 添加 infallible 注释 | c9c7033 | src/logging.rs |
| 2 | parallel.rs:280 expect 添加 infallible 前置注释 | c11af69 | src/cli/run/parallel.rs |
| 3 | 全代码库审计 + 60-AUDIT.md | 426befd | .planning/phases/60-error-handling/60-AUDIT.md |

## Evidence of Changes

### Task 1：logging.rs:60

```
grep -n '\.unwrap(); // infallible: writing to a String never fails' src/logging.rs
60:    .unwrap(); // infallible: writing to a String never fails
```

### Task 2：parallel.rs:280-281

```
grep -n 'infallible: process_csv_parallel is only called when CSV exporter is present' src/cli/run/parallel.rs
280:        // infallible: process_csv_parallel is only called when CSV exporter is present
```

### 60-AUDIT.md 四条成功标准核对结果

- [x] 标准 1：production_uncommented 数量 = 0（4 处生产代码全部已注释）
- [x] 标准 2：src/error.rs 零 diff；replaceable_with_question_mark 数量 = 0
- [x] 标准 3：cargo clippy 退出码 0，无 unwrap_used/expect_used；638 测试全通过
- [x] 标准 4：非注释代码字节零变化，cargo test 100% 通过

## Preservation Decisions (D-01/D-03/D-04)

| 决定 | 目标文件 | git diff 结果 |
|------|---------|--------------|
| D-04：src/error.rs 不修改 | src/error.rs | 零 diff |
| D-03：normalizer.rs:310/418 保留 | src/pipeline/normalizer.rs | 零 diff |
| D-01：rayon map_err 保留 | src/cli/run/sqlite_parallel.rs, prescan.rs | 零 diff |

## Deviations from Plan

无 — 计划完全按预期执行。

## Known Stubs

无。

## Threat Flags

无新增信任边界穿越（本 Plan 为纯注释添加 + 审计报告生成）。

## Self-Check: PASSED

- [x] src/logging.rs 已修改并提交（c9c7033）
- [x] src/cli/run/parallel.rs 已修改并提交（c11af69）
- [x] .planning/phases/60-error-handling/60-AUDIT.md 已创建并提交（426befd）
- [x] cargo fmt/clippy/test 全部通过
- [x] 三个保留判定（D-01/D-03/D-04）零 diff 确认
