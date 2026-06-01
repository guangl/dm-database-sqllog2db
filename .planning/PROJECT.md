# sqllog2db

## What This Is

解析达梦（DaMeng）数据库 SQL 日志文件，流式导出到 CSV 或 SQLite 的命令行工具。支持可配置的过滤管道和字段投影，让用户精确控制"导出哪些记录的哪些字段"。支持 stdin 管道输入、实时进度显示、错误类型细分（fatal/non-fatal），以及 3 级退出码。

## Core Value

用户能够精确指定"导出哪些记录的哪些字段"——过滤逻辑清晰可配置，输出结果完全可控。

## Latest: v1.12 已交付

**Shipped:** 2026-06-01  
**Version:** v1.12 CLI 体验全面提升（Phases 46–49）

**已交付功能：**
- 错误信息优化：`hint:` 前缀统一格式，`format_error_output` 辅助函数
- 配置文件体验：`validate` 静默通过 / `[FAIL]` 失败输出，`init` 模板全字段注释
- 运行提示/日志级别：`--verbose` 逐文件输出 + `--quiet` 完全抑制，摘要差异化
- 多输入 glob 支持：`inputs: Vec<String>` + `--input` CLI flag，glob 展开，旧 `path` 键检测

**Previous: v1.11 已交付（2026-05-25）** — 性能深化与依赖适配（Phases 41–45）

## Requirements

### Validated

- ✓ CSV 导出 — v1.0
- ✓ SQLite 导出 — v1.0
- ✓ Pipeline 过滤器（include/exclude/indicators/sql） — v1.0–v1.2
- ✓ 参数归一化 — v1.3
- ✓ 并行 CSV 处理 — v1.1
- ✓ 项目精简（移除图表/自更新/stats/digest/模板分析/断点续传/补全） — v1.7
- ✓ 错误类型细分（ConfigError/FileError/ParserError/ExportError） — v1.10
- ✓ 非致命错误继续处理，3 级退出码（0/1/2） — v1.10
- ✓ stdin 管道输入（/dev/stdin 路径映射） — v1.10
- ✓ indicatif 进度显示 + 处理摘要 — v1.10
- ✓ --help 达梦场景示例 — v1.10
- ✓ 全链路验证（487 个测试，clippy/fmt 通过） — v1.10
- ✓ 错误信息结构化 `hint:` 前缀，`format_error_output` 辅助函数 — v1.12
- ✓ `validate` 静默通过/`[FAIL]` 失败输出，`init` 模板全字段注释 — v1.12
- ✓ `--verbose` 逐文件输出 + `--quiet` 完全抑制，摘要差异化 — v1.12
- ✓ `inputs: Vec<String>` 替代 `path: String`，config 和 CLI 均支持 glob 展开 — v1.12

### Active

（待下一里程碑定义）

### Out of Scope

| Feature | Reason |
|---------|--------|
| OR 条件组合 | 之前已排除，保持简单过滤模型 |
| 跨字段联合条件 | 之前已排除 |
| 新输出格式（JSON/Parquet） | 保持 CSV/SQLite 双格式 |
| 上游 parser crate 添加 `from_reader()` API | 超出 sqllog2db 范围 |
| MultiProgress 多级进度条 | 单行进度条已满足需求 |
| 数值错误码系统（E001/E002） | 过度工程化，thiserror Display 足够 |

## Context

- Rust 项目，单线程流式处理，16MB BufWriter 写入
- 依赖精简（无 reqwest/rustls/self_update 等重依赖，仅新增 indicatif）
- 当前代码量：~8,833 行 Rust（src）+ 1,503 行（tests）
- 性能基线：~5.2M records/sec（合成 CSV），~1.55M records/sec（1.1GB 真实文件）
- 测试覆盖：529 个测试（226 lib + 48+ integration + 1 jemalloc），全部通过
- assert_cmd / predicates 加入 dev-dependencies，e2e CLI 测试覆盖大幅提升

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
| 非致命错误继续处理 | 达梦日志量大，单条失败不应中断整个导出 | ✓ Good (v1.10) |
| ErrorStats 结构体（非嵌入 Error 枚举） | 分离统计与错误语义 | ✓ Good (v1.10) |
| 3 级退出码（0/1/2/130） | 替代旧的按错误类型映射方案，更简洁 | ✓ Good (v1.10) |
| stdin 通过 /dev/stdin 路径映射 | 不改变现有 --input 参数结构 | ✓ Good (v1.10) |
| indicatif 取代 eprintln 进度输出 | 非终端自动退化，体验更好 | ✓ Good (v1.10) |
| `hint:` 前缀统一格式（两空格缩进） | 可单元测试，用户易识别 | ✓ Good (v1.12) |
| validate 静默通过策略（D-03） | 仅错误时才输出，减少噪音 | ✓ Good (v1.12) |
| verbose 语义从日志级别→运行展示 | `-vv` 不再有效，语义更清晰 | ✓ Good (v1.12) |
| inputs: Vec<String> 替代 path: String | 支持 glob，旧键检测迁移友好 | ✓ Good (v1.12) |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-06-01 after v1.12 milestone*
