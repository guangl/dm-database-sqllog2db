# sqllog2db

## Current Milestone: v1.20 性能全面提升

**Goal:** 前瞻性探索解析速度、SQLite 写入、内存占用与启动速度四个维度，找出并落地实际收益

**Target features:**
- 解析速度优化（热路径分析，减少分配/拷贝）
- SQLite 导出提速（batch write、WAL 调优、prepared stmt 复用）
- 内存占用控制（大文件批量场景下峰值内存分析与优化）
- 启动速度（冷启动延迟基准测量与优化）

## What This Is

解析达梦（DaMeng）数据库 SQL 日志文件，流式导出到 CSV 或 SQLite 的命令行工具。支持可配置的过滤管道和字段投影，让用户精确控制"导出哪些记录的哪些字段"。支持 stdin 管道输入、实时进度显示与错误诊断、交互式配置向导（`init --interactive`）、以及 `watch` 子命令持续监听目录并增量插入新记录。

## Core Value

用户能够精确指定"导出哪些记录的哪些字段"——过滤逻辑清晰可配置，输出结果完全可控。

## Current State: v1.19 已交付

**Shipped:** 2026-06-07  
**Version:** v1.19 watch完善与文档对齐（Phases 1–3, 71）

**已交付功能：**
- watch CSV 增量追加（WATCH-07）：`force_append_for_watch_trigger` 统一注入，多次触发行数正确累计
- error log 追加写入模式（WATCH-08）：`write_error_log` OpenOptions 双分支，历史错误不丢失
- watch Ctrl+C 退出码 130（WATCH-09）：signal-aware 退出路径，与 run 命令一致
- 行覆盖率 92.06%（QUAL-02）：collector.rs + filter_processor.rs 新增单元测试
- VALIDATION.md 正式落地（QUAL-01）：Phase 67/68/69/70 全部以 `status: complete` 建档
- README 补充 watch/init --interactive/quiet+verbose（DOC-04），--help 示例完善（DOC-05）
- Phase 71 mod.rs 重构：10 个 mod.rs 拆分为 >30 个命名子模块，代码结构大幅改善

## Previous: v1.18 已交付

**Shipped:** 2026-06-06  
**Version:** v1.18 用户体验全面升级（Phases 67–70）

**已交付功能：**
- 进度条升级：`[N/M]` 文件计数器 + ETA + records/sec 实时显示（PROG-01/02）
- 错误诊断：ErrorKind 分类 + error log 含行号与原文前 120 字符 + 摘要 hint（DIAG-01/02/03）
- `init --interactive` 对话式配置向导（INIT-01/02/03）
- `watch` 子命令：notify 监听 + 500ms 防抖 + HumanDuration 状态行 + Ctrl+C 摘要（WATCH-01/02/05/06）
- watch 增量处理：`_watch_offsets` SQLite 辅助表 + Seek 增量读 + 跨重启恢复（WATCH-03/04）

## Previous: v1.17 已交付

**Shipped:** 2026-06-04
**Version:** v1.17 多文件并行提速（Phases 64–66）

**已交付功能：**
- CSV 多文件并行处理（process_csv_parallel，基于 rayon work-stealing，对齐 SQLite 并行路径）
- 单文件 I/O 优化（16KB→4MB BufReader，减少系统调用）
- 3 条兼容性集成测试（COMPAT-01/02/03）：并行路径与顺序路径 CSV 内容一致性验证，及 init 模板格式稳定性验证
- 全量 777 个测试通过，0 回归

## Previous: v1.16 已交付

**Shipped:** 2026-06-03
**Version:** v1.16.0 工程质量深化（Phases 59–63）

**已交付功能：**
- cli/run 结构整理：process_log_file / process_csv_parallel / FilterProcessor 全部拆分，ExportAction 枚举引入
- collector.rs 公共模块提取，sqlite_parallel.rs 与 CSV 并行路径共享 collect_log_file/process_record
- 全代码库 unwrap/expect 审计，生产代码全部标注 infallible 或改为 ? 传播
- Cross.toml SHA256 digest 固定（de04c9cd...），消除 :edge 浮动标签
- README stats 用法示例 + CHANGELOG v1.0.0–v1.15.0 + config 模板 22 字段注释
- 51 项新测试，行覆盖率 90.68% → 91.86%，740 个测试全部通过

## Previous: v1.15 已交付

**Shipped:** 2026-06-02
**Version:** v1.15 工程质量全面提升（Phases 55–58）

**已交付功能：**
- GitHub Actions workflow action 版本修复（@v6/@v7 → @v4，6 处）
- release.yaml artifact 暂存 + 独立 create-release job，消除 4 并行 job 竞争条件
- Cross.toml 新建，aarch64-linux 跨编译 cross-rs edge 镜像配置
- `pub(crate) mod scanner` 公共模块，stats/run 共享同一文件扫描实现
- 5 条 e2e CLI 全链路测试（run CSV/SQLite、init 成功/冲突、stats from>to 拒绝），集成测试总数 69 条
- handle_run（234 行）拆分为 7 个语义清晰私有辅助函数，逻辑语句数 ~37
- BENCHMARKS.md CI Artifact 使用指南

## Previous: v1.14 已交付

**Shipped:** 2026-06-02
**Version:** v1.14 stats 时间段过滤（Phases 53–54）

**已交付功能：**
- `--from`/`--to` CLI 参数时间段过滤
- config.toml `[stats]` 节 `from`/`to` 字段作为默认值
- CLI 参数优先于 config 中的值
- `StatsAccumulator` 在聚合前按时间段跳过不符合的记录

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
- ✓ GitHub Actions CI workflow 修复（@v4 版本锁定，三平台矩阵）— v1.15
- ✓ GitHub Actions CD workflow 修复（artifact 暂存 + 独立 create-release，四平台构建）— v1.15
- ✓ Cross.toml aarch64-linux 跨编译配置 — v1.15
- ✓ `scanner` 公共模块（stats/run 共享文件扫描实现）— v1.15
- ✓ e2e CLI 全链路集成测试（run/init/stats，69 个集成测试）— v1.15
- ✓ cli/run handle_run 拆分（7 个私有辅助函数，逻辑语句数 ~37）— v1.15
- ✓ criterion benchmark 稳定化（non-blocking CI，BENCHMARKS.md 指南）— v1.15
- ✓ 生产代码 unwrap/expect 全部注释说明 infallible 或改为 ? — v1.16（Phase 60）
- ✓ cli/run 函数拆分（process_log_file/process_csv_parallel/FilterProcessor，ExportAction 枚举）— v1.16（Phase 59）
- ✓ collector.rs 公共模块，消除 sqlite_parallel.rs 与 CSV 并行路径重复逻辑 — v1.16（Phase 59）
- ✓ Cross.toml SHA256 digest 固定（de04c9cd...），:edge 浮动标签移除 — v1.16（Phase 61）
- ✓ README stats 用法示例 + CHANGELOG v1.0–v1.15 + config 模板全字段注释 — v1.16（Phase 62）
- ✓ 行覆盖率 91.86% / 函数覆盖率 89.54%（51 项新测试，740 全部通过）— v1.16（Phase 63）

- ✓ CSV watch 支持（`force_append_for_watch_trigger` 追加注入，AppendCsv 路径）— v1.19（Phase 1）
- ✓ watch error log 追加写入模式（OpenOptions 双分支，历史错误不丢失）— v1.19（Phase 1）
- ✓ watch Ctrl+C 退出码修正（130，signal-aware）— v1.19（Phase 1）
- ✓ 测试覆盖率 92.06%（行覆盖率），909 个测试全部通过 — v1.19（Phase 2）
- ✓ macOS FSEvents ignore 测试标注（#[ignore] + 注释说明，文档化平台限制）— v1.19（Phase 2）
- ✓ VALIDATION.md 正式落地（phases 67/68/69/70，status: complete）— v1.19（Phase 3）
- ✓ README 补充 watch/init --interactive/quiet+verbose 完整说明 — v1.19（Phase 3）
- ✓ watch/validate `--help` 各 ≥2 示例（DOC-05）— v1.19（Phase 3）
- ✓ 10 个 mod.rs 拆分为命名子模块（watch/mod.rs 998 行拆为 11 个子文件）— v1.19（Phase 71）

### Recently Validated in v1.18

- ✓ `[N/M]` 文件计数器进度条 + ETA + records/sec — v1.18（Phase 67）
- ✓ ErrorKind 分类 + error log 行号/原文 + 摘要 hint — v1.18（Phase 67）
- ✓ `init --interactive` 对话式配置向导 — v1.18（Phase 68）
- ✓ `watch` 子命令（notify 监听 + 500ms 防抖 + Ctrl+C 摘要）— v1.18（Phase 69）
- ✓ watch 增量处理（_watch_offsets 辅助表 + Seek + 跨重启恢复）— v1.18（Phase 70）

### Recently Validated in v1.17

- ✓ CSV 多文件并行处理（rayon，基于 process_csv_parallel，对齐 SQLite 并行路径）— Phase 64
- ✓ 单文件 I/O 优化（16KB→4MB BufReader，减少系统调用）— Phase 65
- ✓ verbose 透传链（并行路径逐文件 "Processing:" 输出，与顺序路径格式一致）— Phase 65
- ✓ 兼容性验证（COMPAT-01/02/03：并行路径与顺序路径输出一致，init 模板格式稳定）— Phase 66
- ✓ jobs_override: Option<usize> 扩展 handle_run（36 处调用点），强制单核 CI 进入并行路径 — Phase 66.1
- ✓ write_heterogeneous_log helper（trxid/username 两维差异化），验证跨文件聚合正确性 — Phase 66.1

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
- 依赖精简（无 reqwest/rustls/self_update 等重依赖，新增 indicatif + notify）
- 当前代码量：~13,819+ 行 Rust（src + tests），Phase 71 拆分后文件数大幅增加但总行数相近
- 性能基线：~5.2M records/sec（合成 CSV），~1.55M records/sec（1.1GB 真实文件）
- 测试覆盖：909 个测试（lib + integration + jemalloc + bench），全部通过，2 个 ignore（macOS FSEvents 限制）
- 行覆盖率 92.06%（v1.19），函数覆盖率 ~89%
- mod.rs 结构：10 个 mod.rs 已拆分为命名子模块（v1.19 Phase 71），代码结构清晰
- assert_cmd / predicates dev-dependencies，e2e CLI 测试覆盖齐全
- GitHub Actions CI/CD workflow 全面修复，Cross.toml aarch64-linux 跨编译支持（SHA256 固定）

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
| release artifact 暂存 + 独立 create-release job | 消除 4 并行 job 竞争写入 release notes | ✓ Good (v1.15) |
| scanner 公共模块 pub(crate) 可见性 | 与 pub(crate) mod parser 保持一致，stats/run DRY | ✓ Good (v1.15) |
| handle_run 物理行数 override 接受 | cargo fmt 展开所致，逻辑语句数 ~37 满足设计意图 | ✓ Good (v1.15) |
| ExportAction 枚举替代 break 'outer | 消除内联控制流，拆分后语义更清晰 | ✓ Good (v1.16) |
| collector.rs pub(super) 可见性 | 限定在 cli/run 子模块内，不对外暴露 | ✓ Good (v1.16) |
| parallel_collect 行数口径采用函数体（33 行）| cargo fmt 展开含参数行，函数体逻辑满足设计意图 | ✓ Good (v1.16) |
| SHA256 digest 使用宿主机（amd64）平台 | cross-rs 在 amd64 主机运行，应取宿主 digest | ✓ Good (v1.16) |
| watch 仅支持 SQLite 导出 | CSV 增量写入语义复杂（追加 vs 全量重写），延后 | ✓ Good (v1.18) |
| _watch_offsets 用独立 rusqlite::Connection | 避免 SqliteExporter EXCLUSIVE 锁冲突 | ✓ Good (v1.18) |
| handle_run 返回后才 save_offset | 避免 offset 在 exporter 持锁时写入 | ✓ Good (v1.18) |
| watch Ctrl+C 退出码 0（vs run 的 130） | watch 内部处理 interrupted，与 run 不一致；已知 tech debt | ⚠ Revisit (v1.18) |
| watch Ctrl+C 退出码修正为 130（WATCH-09） | `Err(Error::Interrupted)` 从 handle_watch 传播到 main.rs exit(130) | ✓ Good (v1.19) |
| write_error_log OpenOptions 双分支 | append_error_log 字段区分 watch 追加模式和 run 覆盖模式 | ✓ Good (v1.19) |
| force_append_for_watch_trigger 辅助函数 | 统一注入 CSV append + error_log append，消除 trigger_full/incremental 重复 | ✓ Good (v1.19) |
| macOS FSEvents #[ignore] 保留 + 文档化 | 保留测试意图可见性；mock 注入方案引入新依赖且与 notify 深度耦合 | ✓ Good (v1.19) |
| Phase 71 mod.rs 拆分（pub(super) + #[allow]） | WatchLoopState 升级 pub(super) 允许兄弟模块访问；集成测试 pub use 加 #[allow(unused_imports)] | ✓ Good (v1.19) |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-06-07 — started v1.20 milestone 性能全面提升*
