# Phase 25: 延后文档补全 - Context

**Gathered:** 2026-05-19
**Status:** Ready for planning

## Phase Boundary

创建三份新的中文文档：CONTRIBUTING.md（贡献指南）、SECURITY.md（安全策略）、docs/architecture.md（架构文档），补全 v1.5 延后的文档需求。

**In scope:** DOC-01, DOC-02, DOC-03
**Out of scope:** 现有代码或架构变更（仅文档）；英文版保留（不创建双语版本）

## Implementation Decisions

### CONTRIBUTING.md
- **D-01:** 标准四段结构 — 开发环境搭建、编码规约、PR 流程、commit 规范
- **D-02:** 全中文撰写，与 Phase 24 中文化策略一致（机翻+人工校对）

### SECURITY.md
- **D-03:** 同时提供 GitHub Security Advisory 和邮箱报告两种方式，以 Advisory 为主
- **D-04:** 全中文撰写

### ARCHITECTURE.md
- **D-05:** 模块级概要深度（2-3 页）— 数据流图、分层架构、主要抽象说明，不深入到具体 struct/trait 级别
- **D-06:** 全中文撰写

### Claude's Discretion
- CONTRIBUTING.md 的具体章节组织和示例代码
- SECURITY.md 的安全邮箱地址和响应时间承诺措辞
- ARCHITECTURE.md 的具体章节划分和图示方式

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求文档
- `.planning/REQUIREMENTS.md` — DOC-01（CONTRIBUTING.md）、DOC-02（SECURITY.md）、DOC-03（docs/architecture.md）的权威描述
- `.planning/ROADMAP.md` — Phase 25 的范围和依赖关系

### 项目规范
- `.planning/PROJECT.md` — Key Decisions 中的架构决策（嵌套配置、模块拆分、validate_and_compile 等）
- `.planning/codebase/CONVENTIONS.md` — 编码规约（命名、格式、lint、import 组织）— CONTRIBUTING.md 的编码规约部分需引用
- `.planning/codebase/STRUCTURE.md` — 目录结构 — ARCHITECTURE.md 的基础参考
- `.planning/codebase/ARCHITECTURE.md` — 现有架构文档 — 参考其深度和结构，但 ARCHITECTURE.md 仅写模块级概要
- `CLAUDE.md` — 项目级开发指令（函数长度 ≤40 行、commits 用 conventional commit 等）

### 待创建文件
- `CONTRIBUTING.md` — 项目根目录
- `SECURITY.md` — 项目根目录
- `docs/architecture.md` — 详细架构文档

## Existing Code Insights

### Reusable Assets
- CONVENTIONS.md 和 STRUCTURE.md 提供了编码规约和目录结构的标准描述，可直接参考
- CLAUDE.md 提供了开发环境搭建步骤（build, test, lint, format）

### Established Patterns
- 全中文文档风格：与 Phase 24 的中文化文档保持一致术语和语气

### Integration Points
- ARCHITECTURE.md 需反映 v1.4 重构后的 5 模块结构（src/config/, src/cli/, src/pipeline/, src/exporter/, src/charts/）
- CONTRIBUTING.md 的 PR 流程与现有 CI gate（cargo clippy -D warnings, cargo fmt --check, cargo test）保持一致

## Specific Ideas

- ARCHITECTURE.md 可使用 ASCII art 数据流图（与 v1.5 决策一致：ASCII art 替代 Mermaid.js）
- CONTRIBUTING.md 可引用 CLAUDE.md 中的 build/test/lint 命令

## Deferred Ideas

None — discussion stayed within phase scope

---

*Phase: 25-延后文档补全*
*Context gathered: 2026-05-19*
