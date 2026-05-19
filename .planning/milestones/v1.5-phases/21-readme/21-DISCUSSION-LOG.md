# Phase 21: README 全面更新 + 根文档补全 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-18
**Phase:** 21-README 全面更新 + 根文档补全
**Areas discussed:** README 结构重组织, CHANGELOG 补全策略

---

## README 结构重组织

| Option | Description | Selected |
|--------|-------------|----------|
| 按功能模块重排 | 以功能为主线重新组织，当前结构打散融入 | |
| 保留结构 + 增量追加 | 保持当前段落结构，追加 v1.3/v1.4 内容 | |
| 精简 README + 链接 docs/ | README 精简为项目概览+快速开始+功能索引，详细内容移至 docs/ | ✓ |

**User's choice:** 精简 README + 链接 docs/

### README 保留内容

| Option | Description | Selected |
|--------|-------------|----------|
| 最小骨架 | 项目简介+功能概览、安装、3步QuickStart、性能数据+架构图、链接索引 | ✓ |
| 中等保留 | 在最小骨架基础上保留 stats/digest 示例、配置模板、FAQ、故障排查 | |

**User's choice:** 最小骨架

### README 配置示例

| Option | Description | Selected |
|--------|-------------|----------|
| 最小配置片段 | 5-10 行核心配置（sqllog.path + exporter），其余链接到 docs/ | ✓ |
| 完整嵌套配置 | 展示完整 v1.4 嵌套配置模板（约 60 行） | |

**User's choice:** 最小配置片段

### 架构图格式

| Option | Description | Selected |
|--------|-------------|----------|
| Mermaid.js | GitHub 原生支持，干净可维护 | ✓ |
| ASCII 字符画 | 纯文本，任何环境可看 | |
| 两者都要 | README ASCII + Pages Mermaid | |

**User's choice:** Mermaid.js

### 死链处理

| Option | Description | Selected |
|--------|-------------|----------|
| 替换为占位标记 | 保留链接但标注 "Coming in v1.6" | ✓ |
| 直接移除 | 精简 README 不保留死链 | |
| 链接到 Roadmap | 指向 ROADMAP.md 或 Issues | |

**User's choice:** 替换为占位标记

### 功能特性组织

| Option | Description | Selected |
|--------|-------------|----------|
| 按版本分组 | v1.0/v1.1/v1.2/v1.3/v1.4 分组 | |
| 按功能领域分组 | 解析与导出/过滤与字段控制/模板分析与图表/配置与性能 | ✓ |
| 单列表 + 新增高亮 | 保持单列表，NEW 标记新功能 | |

**User's choice:** 按功能领域分组

### 性能数据展示

| Option | Description | Selected |
|--------|-------------|----------|
| 更新为最新数据 | 重新运行 benchmark 获取最新数据 | ✓ |
| 保留现数据 + 加注 | 添加测试环境说明，不重新跑 | |
| 精简为一句摘要 | ~1.55M/s 一句，详细链接 Pages | |

**User's choice:** 更新为最新数据

### 语言风格

| Option | Description | Selected |
|--------|-------------|----------|
| 保持中英混合 | 标题英文、正文中文、代码英文 | |
| 纯英文 README | 全部英文，README.zh-CN 计划 v1.6+ | ✓ |
| 双语并列 | 关键段落中英双语 | |

**User's choice:** 纯英文 README

### QuickStart 命令

| Option | Description | Selected |
|--------|-------------|----------|
| 5 命令全覆盖 | init → validate → run → digest → stats | |
| 3 核心 + 链接 | init → validate → run，digest/stats 提及+链接 | ✓ |

**User's choice:** 3 核心 + 链接

### SVG 图表展示

| Option | Description | Selected |
|--------|-------------|----------|
| 文字描述 + 链接 | 2-3 句话说明，链接 Gallery | |
| 嵌入 1-2 张示例 | 嵌入 PNG 截图，其余链接 Pages | ✓ |

**User's choice:** 嵌入 1-2 张示例

### 链接索引

| Option | Description | Selected |
|--------|-------------|----------|
| 现有 + 标状态 | CHANGELOG/Pages/quickstart/config-reference + Coming v1.6 占位 | ✓ |
| 仅已存在文档 | 只放已存在的链接 | |

**User's choice:** 现有 + 标状态

### 跨 Phase 协调

| Option | Description | Selected |
|--------|-------------|----------|
| 先占位后补全 | Phase 21 写入链接标注 Coming Phase 23 | ✓ |
| Phase 21 同时创建骨架 | 创建 docs/quickstart.md 占位骨架 | |

**User's choice:** 先占位后补全

---

## CHANGELOG 补全策略

| Option | Description | Selected |
|--------|-------------|----------|
| 按里程碑大版本 | v1.0.0/v1.1.0/v1.2.0/v1.3.0/v1.4.0，含需求 ID | |
| 按实际 crate 版本 | 对照 crates.io 实际版本号：v1.0/v1.2/v1.2.1/v1.3/v1.4 | ✓ |
| 只写里程碑摘要 | 5 个版本，每个 3-5 行 Added/Changed/Fixed | |

**User's choice:** 按实际 crate 版本

### CHANGELOG 详细度

| Option | Description | Selected |
|--------|-------------|----------|
| 保持现有详细度 | Added/Changed/Fixed/Performance 分类 | ✓ |
| 精简到 Added 为主 | 大版本只列 Added，小版本列 Fixed | |

**User's choice:** 保持现有详细度

### 历史版本处理

| Option | Description | Selected |
|--------|-------------|----------|
| 保留全部 | 完整保留 0.1.0-0.10.6 | |
| 精简旧版本 | 0.x 折叠为一个摘要段落 | ✓ |

**User's choice:** 精简旧版本

### v1.0 迁移说明

| Option | Description | Selected |
|--------|-------------|----------|
| 需要迁移说明 | 标注 Breaking Changes 和 Migration Guide | ✓ |
| 正常 Added 条目 | 无真正破坏性变更，正常写 | |

**User's choice:** 需要迁移说明

---

## Claude's Discretion

- Mermaid 图的具体节点和布局
- 性能数据的具体数值（需运行 benchmark）
- CHANGELOG 各版本的具体变更条目
- README 中嵌入哪 2 张图表

## Deferred Ideas

- README.zh-CN.md — v1.6+
- CONTRIBUTING.md — v1.6+
- SECURITY.md — v1.6+
- docs/architecture.md — v1.6+
- CHANGELOG 自动化生成（git-cliff）— v1.5 范围外
