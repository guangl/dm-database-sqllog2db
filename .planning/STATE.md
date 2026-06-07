---
gsd_state_version: 1.0
milestone: watch完善与文档对齐
milestone_name: milestone
status: ready_to_plan
last_updated: 2026-06-07T03:28:01.082Z
last_activity: 2026-06-07 -- Phase 02 execution started
progress:
  total_phases: 39
  completed_phases: 18
  total_plans: 37
  completed_plans: 57
  percent: 46
stopped_at: Phase 02 complete (2/2) — ready to discuss Phase 35
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-06 after v1.18)

**Core value:** 用户能够精确指定"导出哪些记录的哪些字段"——过滤逻辑清晰可配置，输出结果完全可控
**Current focus:** Phase 35 — cli help

## Current Position

Phase: 35
Plan: Not started
Status: Ready to plan
Last activity: 2026-06-07

```
watch完善与文档对齐 Progress: [░░░░░░░░░░░░░░░░░░░░] 0% (0/3 phases)
Phase 1: watch 功能完善    [ ] Not started
Phase 2: 测试覆盖率与 FSEvents [ ] Not started
Phase 3: 文档与验证对齐   [ ] Not started
```

## Milestone: watch完善与文档对齐

**Goal:** 补完 watch 功能短板（CSV 支持、error log 追加写入、退出码修正），提升文档与测试质量，同步 VALIDATION.md 到 v1.18 实际状态。

**Phases:**

- Phase 1: watch 功能完善 (WATCH-07/08/09)
- Phase 2: 测试覆盖率与 FSEvents (QUAL-02/03)
- Phase 3: 文档与验证对齐 (QUAL-01, DOC-04, DOC-05)

## Deferred Items from v1.18

Items acknowledged at v1.18 milestone close and now scheduled in v1.19:

| Category | Item | v1.19 Phase |
|----------|------|-------------|
| watch | Ctrl+C 退出码 0 vs run 130 | Phase 1 (WATCH-09) |
| watch | write_error_log 覆盖写（只保留最近一次触发的错误） | Phase 1 (WATCH-08) |
| watch | CSV 导出未支持 | Phase 1 (WATCH-07) |
| nyquist | VALIDATION.md 草稿（67/68/69）、70-VALIDATION.md 缺失 | Phase 3 (QUAL-01) |
| test | test_watch_triggers_on_new_log_file #[ignore]（macOS FSEvents 限制） | Phase 2 (QUAL-03) |

## Performance Metrics

- Tests: ~880 total (v1.18 baseline), 2 ignored
- Line coverage: ~91.86% (v1.18 baseline, target 92%+ in v1.19)
- Build: LTO fat + strip + panic=abort

## Accumulated Context

### Key Decisions (v1.19)

- WATCH-07 (CSV watch): 追加写入语义——每次触发向现有 CSV 文件追加，而非全量重写
- WATCH-08 (error log 追加): write_error_log 改为 OpenOptions::append(true)，保留 watch 运行历史
- WATCH-09 (退出码 130): handle_watch 的 Ctrl+C 路径需返回 ExitCode::from(130) 而非 Ok(())
- QUAL-03 (FSEvents): 优先评估 #[cfg(not(target_os = "macos"))] 方案，其次 mock 注入

### Known Constraints

- watch CSV 追加：需要跳过 CSV header 写入（非首次触发时），或通过 append=true config 控制
- error log 追加：需要区分 watch 触发时的追加模式与 run 命令的覆盖模式
- 退出码 130：需要传播 SIGINT 信号穿透 handle_watch 返回路径

## Operator Next Steps

- Run `/gsd:plan-phase 1` to create plans for Phase 1
