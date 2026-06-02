# sqllog2db

## What This Is

解析达梦（DaMeng）数据库 SQL 日志文件，流式导出到 CSV 或 SQLite 的命令行工具。支持可配置的过滤管道和字段投影，让用户精确控制"导出哪些记录的哪些字段"。支持 stdin 管道输入、实时进度显示、错误类型细分（fatal/non-fatal），以及 3 级退出码。

## Core Value

用户能够精确指定"导出哪些记录的哪些字段"——过滤逻辑清晰可配置，输出结果完全可控。

## Current Milestone: v1.15 工程质量全面提升

**Goal:** 补全测试覆盖、清理技术债务、建立 CI/CD 基础设施，为后续功能迭代打好工程基础。

**Target features:**
- e2e CLI 全链路集成测试（覆盖 run/stats/validate/init 命令，含 edge case）
- 代码重构清理（cli/run 模块拆分、stats 模块整理、clippy/技术债务）
- GitHub Actions CI：push/PR 自动运行 test/clippy/fmt
- GitHub Actions CD：打 tag 自动构建多平台二进制并发布到 GitHub Releases
- 性能基准追踪（criterion benchmark 稳定化，CI 可选接入）

## Previous: v1.14 已交付

**Shipped:** 2026-06-02
**Version:** v1.14 stats 时间段过滤（Phases 53–54）

**已交付功能：**
- `--from`/`--to` CLI 参数时间段过滤
- config.toml `[stats]` 节 `from`/`to` 字段作为默认值
- CLI 参数优先于 config 中的值
- `StatsAccumulator` 在聚合前按时间段跳过不符合记录

## Previous: v1.13 已交付

**Shipped:** 2026-06-01  
**Version:** v1.13 SQL 统计分析（Phases 50–52）

**已交付功能：**
- SQL 标准化引擎：将字面量替换为 `?` 占位符，参数不同但模板相同的 SQL 归并为同一组
- `stats` 子命令：`sqllog2db stats -c config.toml [--top N]`
- 慢 SQL TOP-N：按 elapsed 降序，输出 SQL文本 + elapsed + 时间戳
- 高频 SQL TOP-N：标准化分组，输出标准化SQL + 调用次数 + avg/max elapsed
- 复用现有 CSV/SQLite exporter，`--top` 默认 20

## Previous: v1.12 已交付


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
- ✓ `stats` 子命令（慢 SQL TOP + 高频 SQL TOP）— v1.13
- ✓ SQL 标准化（参数替换为占位符）— v1.13
- ✓ `--top N` 参数（默认 20）— v1.13
- ✓ 输出格式复用 config.toml exporter — v1.13
- ✓ `stats --from`/`--to` CLI 参数（时间段过滤）— v1.14
- ✓ config.toml `[stats]` 节 `from`/`to` 字段 — v1.14
- ✓ CLI 参数优先于 config 值 — v1.14
- ✓ `StatsAccumulator` 按时间段跳过不符合记录 — v1.14

### Active

- ✓ e2e CLI 全链路集成测试 — v1.15 (Phase 57)
- ✓ cli/run 模块拆分与代码清理 — v1.15 (Phase 58)
- [ ] stats 模块重构整理 — v1.15
- [ ] GitHub Actions CI（test/clippy/fmt） — v1.15
- [ ] GitHub Actions CD（多平台构建 + GitHub Releases） — v1.15
- [ ] criterion benchmark 稳定化 — v1.15

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
- 测试覆盖：~558 个测试（226 lib + 69 integration + 1 jemalloc + 单元测试），全部通过
- assert_cmd / predicates 加入 dev-dependencies，e2e CLI 测试覆盖大幅提升
- Phase 57 新增：stats --from/--to 跨字段顺序校验，run CSV/SQLite 全链路断言，init 成功/冲突测试

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
*Last updated: 2026-06-02 after Phase 58*
