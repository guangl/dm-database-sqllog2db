# Phase 27: 模板报告独立输出 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-19
**Phase:** 27-模板报告独立输出
**Areas discussed:** 触发方式, 输出格式, 文件命名, SQLite 表结构, 配置模型

---

## 触发方式

| Option | Description | Selected |
|--------|-------------|----------|
| 自动随 run 生成 | 只要配置了模板分析，run 完成后自动生成 | ✓ |
| 新增 CLI 子命令 | 添加 sqllog2db report 子命令 | |
| 两者都支持 | run 自动生成 + 独立 report 子命令 | |
| 你决定 | 由 Claude 选择最合适的方式 | |

**User's choice:** 自动随 run 生成
**Notes:** 与现有 companion CSV 模式一致，减少用户操作

---

## 输出格式

| Option | Description | Selected |
|--------|-------------|----------|
| 复用 TemplateStats | template_key, count, avg/min/max/p50/p95/p99_us, first/last_seen | ✓ |
| 扩展更多统计字段 | 增加 total_us、stddev_us、调用频率等 | |
| 你决定 | 由 Claude 根据实用性设计 | |

**User's choice:** 复用 TemplateStats
**Notes:** 字段已足够覆盖 DBA 常见分析需求，避免过度设计

---

## 文件命名

| Option | Description | Selected |
|--------|-------------|----------|
| 自动派生 | 从输出文件名自动派生：out.csv → out_templates.csv / out_templates.db | ✓ |
| 配置指定 | config.toml 中新增 [templates] 段，允许指定文件名 | |
| 两者都支持 | 配置指定优先，无配置时自动派生 | |

**User's choice:** 自动派生
**Notes:** 简单直观，不需要额外配置

---

## SQLite 表结构

| Option | Description | Selected |
|--------|-------------|----------|
| 单表设计 | 单表 template_stats，以 template_key 为主键 | |
| 多表范式化 | template_keys + template_stats + latency_percentiles，三表范式化 | ✓ |
| 你决定 | 由 Claude 选择最佳设计 | |

**User's choice:** 多表范式化
**Notes:** 三表设计更灵活，方便按模板查询和按百分位分析

---

## 配置模型

| Option | Description | Selected |
|--------|-------------|----------|
| 复用现有配置 | 复用现有 [exporter.csv] 和 [exporter.sqlite] 段 | |
| 新增配置段 | 新增 [templates] 配置段，可选启用/禁用独立报告输出 | ✓ |
| 你决定 | 由 Claude 选择最简洁方案 | |

**User's choice:** 新增配置段
**Notes:** 独立配置段提供清晰的边界，与现有 exporter 配置解耦

---

## Deferred Ideas

None
