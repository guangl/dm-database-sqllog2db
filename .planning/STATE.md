---
gsd_state_version: 1.0
milestone: v1.18
milestone_name: 用户体验全面升级
status: roadmap_ready
last_updated: "2026-06-05T00:00:00.000Z"
last_activity: 2026-06-05
progress:
  total_phases: 4
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-05)

**Core value:** 用户能够精确指定"导出哪些记录的哪些字段"——过滤逻辑清晰可配置，输出结果完全可控
**Current focus:** v1.18 用户体验全面升级 — 路线图已创建，准备进入 Phase 67

## Current Position

Phase: 67 (进度/摘要与诊断增强) — Not started
Plan: —
Status: Roadmap ready, awaiting first plan
Last activity: 2026-06-05 — v1.18 roadmap created (Phases 67–70)

## Phase Overview

| Phase | Name | Requirements | Status |
|-------|------|--------------|--------|
| 67 | 进度/摘要与诊断增强 | PROG-01/02/03, DIAG-01/02/03 | Not started |
| 68 | 交互式配置向导 | INIT-01/02/03 | Not started |
| 69 | Watch 模式核心框架 | WATCH-01/02/05/06 | Not started |
| 70 | Watch 增量处理与集成测试 | WATCH-03/04 | Not started |

## Accumulated Context

### Roadmap Evolution

- Phase 66.1 inserted after Phase 66: 修复并行集成测试覆盖（v1.17）
- v1.18 Phases 67–70 derived from 15 requirements across 4 feature areas

### Key Design Decisions (v1.18 Roadmap)

- PROG + DIAG 合并为 Phase 67：两者均是扩展现有输出管道（indicatif + ErrorStats），改动集中在同一层，无独立测试边界
- INIT 向导独立为 Phase 68：stdin/stdout 交互逻辑与常规 CLI 路径不同，测试策略需要模拟 stdin 输入
- watch 拆为两个 phase：Phase 69 建立 notify crate 监听框架（新子命令脚手架 + 显示/退出），Phase 70 做最复杂的增量逻辑（字节偏移持久化 + SQLite 去重）
- watch 仅支持 SQLite 导出（CSV 增量写入语义复杂，已列为 Out of Scope）

### Architecture Notes for Phase 67

- 扩展 `indicatif` ProgressBar template，加入 `{pos}/{len}` 文件计数器和 ETA
- 扩展 `ErrorStats` 结构体：新增 `by_type: HashMap<ErrorKind, u64>` 字段
- error log 行格式：`[ERROR] line {n}: {first_120_chars_of_raw}  reason: {msg}`
- hint 触发逻辑：`encoding_error > threshold` → 输出编码 hint；`field_missing > threshold` → 输出字段 hint

### Architecture Notes for Phase 68

- `init` 子命令新增 `--interactive` bool flag（clap）
- 向导实现在 `src/cli/init.rs` 或新建 `src/cli/init/wizard.rs`
- 每步 `print!` 提示 + `std::io::stdin().read_line()` 读取，Enter 接受默认
- 生成逻辑复用现有 `CONFIG_TEMPLATE_EN` 常量，只替换用户输入的字段值

### Architecture Notes for Phases 69–70

- 新增 `Commands::Watch { config }` clap 变体
- 新建 `src/cli/watch/mod.rs` 模块
- 依赖 `notify` crate（跨平台文件系统事件）
- Phase 70 字节偏移存储：运行时内存 `HashMap<PathBuf, u64>`，进程重启时从 SQLite 辅助表恢复

### Blockers

None

## Session Continuity

Last session: 2026-06-05
Stopped at: Roadmap created, Phase 67 not yet started
Resume file: None

## Operator Next Steps

- Start Phase 67 with `/gsd:plan-phase 67`
