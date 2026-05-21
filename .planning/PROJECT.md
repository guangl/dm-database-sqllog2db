# sqllog2db

## What This Is

解析达梦（DaMeng）数据库 SQL 日志文件，流式导出到 CSV 或 SQLite 的命令行工具。支持可配置的过滤管道和字段投影，让用户精确控制"导出哪些记录的哪些字段"。

## Core Value

用户能够精确指定"导出哪些记录的哪些字段"——过滤逻辑清晰可配置，输出结果完全可控。

## Current Milestone: v1.10 质量加固与体验优化

**Goal:** 修复审计遗留问题，完善核心验证，重构错误处理，提升 CLI 用户体验

**Target features:**
- 审计遗留修复 — 清理 v1.7 审计发现的 3 项技术债
- Phase 33 核心验证 — CSV/SQLite 导出、Pipeline 过滤器、参数归一化、并行 CSV、全链路验证
- 错误处理重构 — 错误类型细分，非致命错误时继续处理
- Unix 管道输入 — 支持 stdin 作为输入源
- CLI 体验优化 — 进度显示、更友好的输出、更好的 --help、更清晰的错误信息

## Requirements

### Validated

- ✓ CSV 导出 — v1.0
- ✓ SQLite 导出 — v1.0
- ✓ Pipeline 过滤器（include/exclude/indicators/sql） — v1.0–v1.2
- ✓ 参数归一化 — v1.3
- ✓ 并行 CSV 处理 — v1.1
- ✓ 项目精简（移除图表/自更新/stats/digest/模板分析/断点续传/补全） — v1.7

### Active

- [ ] **FIX-01**: 清理 normalize_template 死代码（~135 行）
- [ ] **FIX-02**: [template] 配置段应显式拒绝而非静默接受
- [ ] **FIX-03**: 清理 FileError::ReadFailed 遗留 TODO
- [ ] **VER-01**: CSV 导出端到端验证通过
- [ ] **VER-02**: SQLite 导出端到端验证通过
- [ ] **VER-03**: Pipeline 过滤器验证通过
- [ ] **VER-04**: 参数归一化验证通过
- [ ] **VER-05**: 并行 CSV 验证通过
- [ ] **VER-06**: cargo build/test/clippy 全通过
- [ ] **ERR-01**: 错误类型细分（IO/格式/配置/解析），不再笼统报错
- [ ] **ERR-02**: 非致命错误时继续处理而非终止
- [ ] **PIPE-01**: 支持 stdin 管道输入（cat log | sqllog2db run）
- [ ] **UX-01**: 处理进度实时显示
- [ ] **UX-02**: 输出格式更友好（统计摘要、颜色/图标）
- [ ] **UX-03**: --help 文档更清晰完整
- [ ] **UX-04**: 错误信息包含上下文（行号、文件路径、建议修复）

### Out of Scope

| Feature | Reason |
|---------|--------|
| OR 条件组合 | 之前已排除，保持简单过滤模型 |
| 跨字段联合条件 | 之前已排除 |
| 新输出格式（JSON/Parquet） | 保持 CSV/SQLite 双格式，v1.10 聚焦质量 |

## Context

- Rust 项目，单线程流式处理，16MB BufWriter 写入
- 依赖精简（v1.9.0 已砍掉 reqwest/rustls/self_update 等重依赖）
- v1.7 精简了大量功能模块，Phase 33 核心验证被跳过
- v1.7 审计发现 3 项技术债：死代码、静默配置接受、遗留 TODO
- 当前性能基线：~5.2M records/sec（合成 CSV），~1.55M records/sec（1.1GB 真实文件）

## Constraints

- **兼容性**: 不改变现有配置格式（TOML），保持 `init`/`run`/`validate` 三个子命令
- **性能**: 不引入性能退化，保持流式单线程架构
- **依赖**: 不新增重量级依赖，保持精简原则
- **错误处理**: parse error 不致命，写入 error log 后继续

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| 流式单线程架构 | 常数内存，不随文件大小增长 | ✓ Good |
| CSV 优先于 SQLite 导出 | 更通用的交换格式 | ✓ Good |
| 16MB BufWriter + itoa | 零分配 CSV 格式化 | ✓ Good |
| v1.7 移除模板分析/图表等功能 | 精简非核心功能 | ✓ Good |
| 非致命错误继续处理 | 达梦日志量大，单条失败不应中断整个导出 | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-05-21 after v1.10 milestone start*
