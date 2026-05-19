# Phase 25: 延后文档补全 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-19
**Phase:** 25-延后文档补全
**Areas discussed:** ARCHITECTURE.md 详细程度, SECURITY.md 报告方式, CONTRIBUTING.md 内容范围

---

## ARCHITECTURE.md 详细程度

| Option | Description | Selected |
|--------|-------------|----------|
| 模块级概要 | 模块级：数据流图、分层架构、主要抽象说明（2-3 页） | ✓ |
| 代码级详细 | 代码级：每个模块的关键 struct/trait、调用关系、性能设计要点（5-8 页） | |
| 你决定 | 由 Claude 判断合适深度 | |

**User's choice:** 模块级概要
**Notes:** 2-3 页足够给新贡献者建立全局认知，代码级细节可查看源代码

---

## SECURITY.md 漏洞报告方式

| Option | Description | Selected |
|--------|-------------|----------|
| 邮箱报告 | 创建 SECURITY.md 写明邮箱，漏洞报告通过私密邮件沟通 | |
| GitHub Advisory | 使用 GitHub Security Advisory 功能，在仓库内私密报告和跟踪 | |
| 两者都写 | 两者都写：主推 Advisory，备选邮箱 | ✓ |

**User's choice:** 两者都写
**Notes:** GitHub Advisory 为主流程，邮箱作为备选联系渠道

---

## CONTRIBUTING.md 内容范围

| Option | Description | Selected |
|--------|-------------|----------|
| 标准四段 | 开发环境搭建 + 编码规约 + PR 流程 + commit 规范 | ✓ |
| 完整手册 | 标准四段 + 测试指南 + 发布流程 + 代码审查清单 | |
| 你决定 | 由 Claude 判断合适范围 | |

**User's choice:** 标准四段
**Notes:** 覆盖贡献者最需要的四个环节，避免文档膨胀

---

## Deferred Ideas

None
