# Feature Research: sqllog2db v1.10 CLI Quality Improvements

**Domain:** 达梦数据库 SQL 日志处理 CLI 工具（Rust 实现）
**Researched:** 2026-05-21
**Target Users:** 数据库管理员、使用达梦数据库的开发者
**Confidence:** HIGH（主要基于当前代码分析 + 领域最佳实践）

## Feature Landscape

### Table Stakes (Users Expect These)

用户不会因为这些功能"好评"，但缺失会产生强烈不满。

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| 非致命错误时继续处理 | 达梦日志量可达数 GB，单条解析失败不应中止整个导出流程 | LOW | **已实现。** `processor.rs` 中 parse error 写入日志后继续循环。需要补充的是：错误计数的统计汇总输出（当前仅在 info 日志中有 `{errors_in_file} errors`） |
| `--quiet` 标志 | 所有 CLI 工具的基本礼仪，无冗余输出 | LOW | **已实现。** `-q` 全局标志存在，但验证：`run` 命令中 `quiet` 关闭 `show_progress` 和最终摘要，`init/validate` 子命令未传递 `quiet` |
| `--help` 列出子命令和选项 | CLI 工具的基本可用保障 | LOW | **已实现。** clap derive 自动生成，但内容过简（无 examples，无典型工作流） |
| 错误退出码区分 | 脚本调用时按 exit code 判断错误类型 | LOW | **已实现。** 四种退出码（配置/IO/导出/中断）定义清晰且有测试覆盖 |
| 错误信息包含文件路径 | 多文件处理时知道哪个文件出问题 | MEDIUM | **部分实现。** `processor.rs` 的 warn 中有 `{file_path} \| {e:?}`，但顶层 `Error: {e}` 不是所有变体都携带路径。`FileError::WriteFailed` 带路径，`ParserError::PathNotFound` 带路径，但 `ConfigError::ParseFailed` 的格式化不含路径上下文提示 |
| 输出不污染 stdout | CLI 工具的输出（CSV）不应与日志/进度混在一起 | LOW | **已实现。** 数据输出→文件，状态输出→stderr (`eprintln!`)，日志→文件 |

### Should-Have (Expected for a Polished Tool)

用户不会默认有，但遇上就会觉得"这才是专业工具"。

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| 终端下实时进度显示 | 处理大文件（1GB+）时用户需要知道程序仍在工作 | MEDIUM | **缺失。** 当前用 `eprintln!` 逐文件输出起止行，无进度动画。需要引入 `indicatif` |
| 进度条在管道模式静默 | `sqllog2db run ... 2>&1 \| tee log` 时进度动画不应误渲染 | LOW | 与进度显示配套：检测 stderr 是否为 TTY，非 TTY 时退化为文件级文本状态 |
| stdin 管道输入 | `cat log.log \| sqllog2db run --input -` 是 Unix 用户的直觉 | MEDIUM | **缺失。** 当前 `SqllogParser` 仅支持文件系统路径。需要将 `LogParserBuilder` 接入 `io::stdin().lock()` |
| 错误信息带行号/上下文 | 知道哪一行出错，而不只是"第50000行附近有格式错误" | MEDIUM | **缺失。** 当前 parse error 没有行号记录。需与 `dm-database-parser-sqllog` 配合获取行号 |
| 处理结束的统计摘要 | 快速了解处理结果：总记录数、错误数、耗时、导出大小 | LOW | **已实现。** `handle_run` 最后输出格式化的摘要行 |
| 配置验证的错误提示 | validate 子命令说清楚哪里配置不对，而非简单抛错误 | LOW | **部分实现。** `preflight.rs` 有专门检查，`validate` 返回 `ConfigError` 带详细原因。但缺少"修复建议" |
| `--version` 输出 | 检查当前安装版本 | LOW | **已实现。** clap 的 `version` 属性自动生成 |

### Differentiators (Competitive Advantage)

这些功能使 sqllog2db 在同类工具中脱颖而出。

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| 多级进度（文件级 + 总进度） | `indicatif` 支持 MultiProgress，同时显示当前文件进度条和总体进度条 | MEDIUM | 达梦日志常涉及数百个文件，多级进度让用户知道具体进度和整体进度 |
| 错误按类别分组汇总 | 处理完显示"语法错误 5 条，IO 错误 2 条，跳过文件 1 个"的紧凑摘要 | LOW | 当前 `errors_in_file` 只在 info 日志中，用户要查看日志文件才能看到 |
| stdin + 配置文件同时生效 | 管道输入时仍然应用配置中的 filter/replace 规则 | MEDIUM | 用户无需在管道模式下放弃完整功能集 |
| 帮助文档含真实范例 | --help 中展示 3-4 个达梦场景的具体例子 | LOW | 直接降低新手使用门槛，体现对该领域的理解 |
| 并行模式下不乱刷屏的输出 | 并行 CSV 时多线程 `eprintln!` 不会互相交错，而是各线程独立输出后汇总 | MEDIUM | 当前并行路径中 `process_log_file` 带 `reset_pb=false`，但多个文件的起止信息仍可能交错 |

### Anti-Features (Commonly Requested, Often Problematic)

看起来好但实际有害的功能。

| Anti-Feature | Why Requested | Why Problematic | Alternative |
|--------------|---------------|-----------------|-------------|
| 自动检测 stdin（无 `--input` 标志） | "更简单，用户不用学新参数" | 1) 与 `--config` 中的 `sqllog.path` 冲突：stdin 和配置文件都有输入路径时听谁的？2) 用户在交互式终端忘记传输入参数时，会卡在 stdin 读取。3) Unix 惯例是显式 `-` 表示 stdin | 使用 `sqllog2db run --input -` 显式选择 stdin。配置文件中 `sqllog.path` 在 stdin 模式下忽略（或静默覆盖） |
| 实时输出记录到 stdout（tail -f 模式） | "想实时看到每一条导出的 SQL" | 1) 导出到 CSV 时需要 stdout 是干净的数据渠道。2) 5M records/sec 下实时打印是灾难性瓶颈 | 用进度条显示处理速率。需要"实时查看"的用户可以查错误日志或配置较低日志级别后观察文件 |
| 逐条记录的进度条 | "想看到文件内部的具体进度" | 1) 达梦日志格式不支持预先获取行数，无法计算百分比。2) 逐条更新每行会产生大量终端刷新，~5M/s 速率下不可行 | 文件级进度 + 最终统计摘要。已完成文件数/总文件数的进度条 |
| man page / 完整文档站集成 | "专业工具需要有手册页" | 1) 项目文档已独立部署在 GitHub Pages。2) man page 需要额外构建步骤和 CI 维护。3) 当前社区主要在 README 和配置文件注释中查找用法 | --help 中的 examples 足够覆盖 90% 使用场景 |
| 彩色输出 | "让输出更好看" | 1) 依赖 `ansi_term`/`colored` crate。2) 管道时输出乱码。3) 对效率工具的用户来说"好看"次于"准确"和"快" | 使用符号前缀（`✓` `⚠` `✗`）+ 粗体/普通区分，无需颜色 crate。若用户确实需要颜色，可考虑在 --help 风格的彩色输出中少量使用（clap 已支持） |
| --verbose 输出调试级别的每条记录信息 | "我要追踪每条记录的过滤决策" | 会产生毁灭性的输出量（数百万行），实际不可用 | 用专门的 `--dry-run`（统计模式，不导出）或限定范围的 `--filter-debug` 输出过滤后被丢弃的记录 ID |
| JSON 格式的错误输出 | "方便机器解析" | 当前目标用户是 DBA，不是 CI 系统。JSON 输出增加了一倍的错误处理复杂度 | 结构化但人类可读的文本错误输出。未来需要 machine-readable 输出时再加 `--output-format json` |
| 暂停/恢复进度 | "处理过程中能暂停" | 1) 单线程流式架构不支持暂停后恢复。2) 暂停期间日志文件可能被滚动。3) 用户可以用 `Ctrl+C` 中断后用 `--limit` 跳过已完成部分 | Ctrl+C 中断 + restart 时用 filter 跳过已处理部分。这是 v1.7 移除断点续传模块的原因 |
| 自动打开输出文件 | "处理完直接打开结果" | 1) 破坏 CLI 的 composeability。2) 需要平台相关的打开命令。3) DBA 不需要编辑器打开 CSV | 在摘要中打印输出路径，让用户自己决定 |

## Feature Dependencies

```
[Progress Bar (MultiProgress)]
    └──requires──> [indicatif crate] (~120KB 新增依赖)
    └──requires──> [is-terminal detection]
    └──requires──> [progress 与 log 不交错]
                        └──requires──> [run 模式 logging.init(log_to_stdout=false)]

[Stdin Pipe Input]
    └──requires──> [--input CLI flag on run subcommand]
    └──requires──> [SqllogParser 支持 io::stdin().lock() 路径]
    └──requires──> [LogParserBuilder 的 stdin 适配]
    └──optional──> [Stdin 模式静音进度显示（无文件数可预知）]

[Better Error Messages]
    └──requires──> [行号跟踪从 dm-database-parser-sqllog 传递]
    └──enhances──> [错误按类别分组汇总]
    └──enhances──> [非致命错误继续处理]

[Better --help]
    └──requires──> [clap after_help / examples 属性]
    └──enhances──> [用户 onboarding 体验]

[Error Statistics Summary]
    └──requires──> [processor loop 中持久化错误计数器]
    └──enhances──> [handle_run 最终输出]
```

### Dependency Notes

- **Progress Bar + indicatif**: 这是合理的取舍。当前 0 依赖实现（`eprintln!`）的功能极度简陋，`indicatif` 是 Rust CLI 生态的事实标准，约 120KB 编译产物增量。需要配合 `indicatif-log-bridge` 防止日志输出与进度条渲染交错（当前 `logging::init_logging` 的 `log_to_stdout=false` 参数已在设计中预留了这个需求）。
- **Stdin Path**: `SqllogParser` 的 `log_files()` 方法在 stdin 模式下不应被调用（没有文件列表）。需要新增 `SqllogParser::from_reader()` 或修改 run 命令的分支逻辑，使 stdin 模式跳过文件发现和 preflight 检查。
- **行号传递**: 当前 `dm-database-parser-sqllog` 的 `LogParserBuilder::iter()` 返回的 `Result<Sqllog>` 不包含行号。需要在 parser crate 中新增行号字段或在 wrapper 层包装带行号的迭代器。

## MVP Recommendation for v1.10

### Must Have (P0)

按实现复杂度降序：

| # | Feature | Rationale | Est. Complexity |
|---|---------|-----------|-----------------|
| 1 | 错误类型细分（IO/格式/配置/解析）+ 非致命错误继续 | 审计遗留修复，安全前提 | MEDIUM（已有误差基？） |
| 2 | 技术债清理（FIX-01/02/03） | 审计要求 | LOW（代码清理） |
| 3 | 更好的错误信息上下文——文件路径 + 行号 | 直接影响用户处理问题的效率 | MEDIUM（需 parser crate 配合） |
| 4 | stdin 管道输入 (`--input -`) | 核心功能补齐 | MEDIUM（流式架构已就绪） |
| 5 | --help 增强（examples） | 低成本高回报 | LOW |
| 6 | 核心验证（Phase 33） | 质量保障 | MEDIUM（测试用例） |

### Should Have (P1)

| # | Feature | Rationale | Est. Complexity |
|---|---------|-----------|-----------------|
| 7 | 终端下进度条显示（indicatif） | 用户体验显著提升 | MEDIUM（新增依赖 + MultiProgress 实现） |
| 8 | 错误统计摘要 | 处理完一眼知道结果 | LOW |
| 9 | 进度条在管道模式自动退化为文本 | 兼容性考虑 | LOW（is-terminal 检测） |

### Defer (P2+)

| # | Feature | Why Defer | Complexity |
|---|---------|-----------|------------|
| 10 | 并行模式下不乱刷屏的输出 | 当前并行已有关注（reset_pb=false），偶尔交错不影响正确性 | MEDIUM |
| 11 | 错误按类别分组汇总 | 当前日志系统已记录各类错误，汇总是增值功能 | LOW（但仍需 P0/P1 完成后） |
| 12 | stdout 输出到终端时的格式美化（符号前缀） | 当前 `eprintln!` 已有 `✓` 前缀，够用 | LOW |
| 13 | stdin + config filter 组合测试 | 功能依赖 P0 的 stdin 实现完成后自然覆盖 | LOW（测试用例） |

### Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| 错误类型细分 + continue-on-error | HIGH | LOW | P0 |
| 技术债清理 | MEDIUM | LOW | P0 |
| 更好错误信息（路径 + 行号） | HIGH | MEDIUM | P0 |
| stdin 管道输入 | HIGH | MEDIUM | P0 |
| --help 增强（examples） | MEDIUM | LOW | P0 |
| Phase 33 核心验证 | HIGH | MEDIUM | P0 |
| 进度条（indicatif） | MEDIUM | MEDIUM | P1 |
| 错误统计摘要 | MEDIUM | LOW | P1 |
| 管道模式自动退化为文本 | LOW | LOW | P1 |
| 并行模式不乱刷屏 | LOW | MEDIUM | P2 |
| 错误分组汇总 | LOW | LOW | P2 |
| stdout 输出美化（符号） | LOW | LOW | P2 |

## Competitor Feature Analysis

当前直接竞品较少——达梦 SQL 日志解析是一个小众领域。以下比较基于通用的日志处理 CLI 工具（`grep`, `awk`, `jq`, `pv` 等）的用户期望。

| Feature | grep/awk | pv (pipe viewer) | sqllog2db (当前) | sqllog2db (v1.10 目标) |
|---------|----------|-------------------|------------------|------------------------|
| 进度显示 | 无进度，输出完成后统计行数 | 精确字节进度条 | 逐文件 `[i/N]` 文本 | indicatif MultiProgress + 文件级进度 |
| stdin 支持 | `cat \| grep` 是核心使用场景 | 设计目的就是管道 | 不支持 | `--input -` |
| 错误信息 | `grep: path: No such file or directory` | 极简 | `Error: {e}` 基本完整 | 带行号/路径/建议 |
| 非致命错误处理 | 继续读取下一个文件 | 无此概念 | 已实现 | 保持 + 增强统计 |
| 帮助文档 | 巨量选择（GNU manual） | `pv --help` 清晰列出 flags | 基本 clap 输出 | 添加 examples 和工作流 |
| 统计摘要 | `grep -c` 或 `wc -l` | 结束时显示速率/总量 | `{records} records total` | 增加错误/跳过/耗时 |
| 彩色语法高亮 | grep --color | 无 | 无 | 不追求（anti-feature） |
| 退出码区分 | 0=匹配, 1=不匹配, 2=错误 | 0/1 | 4 种退出码 | 保持 + 确认覆盖 |

## Sources

- 代码分析：sqllog2db 源码（所有模块已阅读）
- 生态参考：`indicatif` crate 文档（crates.io, v0.18.4）
- 生态参考：`is-terminal` crate（crates.io, v0.4.17，clap 的 `is_terminal_polyfill` 已存在于依赖树）
- Unix CLI 设计惯例：stdin `-` 惯例（POSIX 标准，被 tar, cat, git 等广泛采用）
- 错误信息设计参考：Rust CLI error message patterns（thiserror 的最佳实践）
- 项目需求：`.planning/PROJECT.md` 中定义的 v1.10 目标

## 技术方案概要

### 进度条实现

```rust
// 核心选择：indicatif 0.18.x，原因：
// 1. Rust CLI 领域事实标准（161M+ 下载量）
// 2. MultiProgress 支持多级进度条（当前文件 + 总体）
// 3. 自动检测 TTY（on_tty() 方法），管道时静默
// 4. 与 log crate 配合：init_logging(log_to_stdout=false) + indicatif-log-bridge 可选

// 方案：file-level MultiProgress
// - 总进度条（底部）：已完成文件数 / 总文件数，`[████░░░░] 3/5`
// - 当前文件进度条（顶部）：当前文件名 + indeterminate spinner（因行数未知）
// - 完成一个文件后：总进度条前进一步，当前文件进度条替换为完成的文件名

// 管道模式（stderr 非 TTY）：退化为当前行为 `[i/N] file.log`
```

### Stdin Pipe 实现

```rust
// 在 Cli::Commands::Run 中增加：
// #[arg(short = 'i', long = "input")]
// input: Option<String>  // None = 使用配置文件中的 sqllog.path; Some("-") = stdin

// 在 handle_run 中：
// match input {
//     Some("-") => process_stdin(...),   // 跳过 SqllogParser::log_files()
//     Some(path) => override config path, // 临时覆盖 sqllog.path
//     None => use cfg.sqllog.path,        // 默认行为
// }

// process_stdin:
// - LogParserBuilder::from_reader(io::stdin().lock())  // 需要 dm-database-parser-sqllog 支持
// - 跳过 preflight 检查
// - 跳过预扫描（无文件列表）
// - 跳过并行模式
// - 进度条退化为 indeterminate（未知总大小）
```

### 错误信息增强

```
// 当前错误格式：
// Error: Write failed /path/to/file: Permission denied (os error 13)

// 增强后格式（按场景分层）：
// ✗ [EXPORT ERROR] /path/to/output.csv 写入失败 → 权限被拒绝
//   Hint: 检查输出目录的写入权限，当前用户可能没有 /export/ 目录的写权限
//   Fix:   sqllog2db run -c config.toml --set exporter.csv.file=/tmp/output.csv
//
// ✗ [PARSE ERROR] /var/log/dm/sqllog_20250521.log: 第 487 行解析失败
//   Reason: 预期 TIMESTAMP 格式为 YYYY-MM-DD HH:MM:SS.FFF
//   Context: "2025-05-21 10:30:28 这是一行不完整的日志"
//   Hint: 此行不符合达梦 SQL 日志标准格式，可能是跨行截断或日志记录损坏
```

### --help 增强

```
// 当前效果：
// sqllog2db-run
// Run the log export task
// Usage: sqllog2db run [OPTIONS]
//
// Options:
//   -c, --config <CONFIG>  Configuration file path [default: config.toml]

// 增强效果：
// sqllog2db-run
// 解析达梦 SQL 日志并导出到 CSV 或 SQLite
//
// Usage: sqllog2db run [OPTIONS] [--input <PATH>]
//
// Options:
//   -c, --config <CONFIG>  配置文件路径 [default: config.toml] [env: SQLLOG2DB_CONFIG]
//   -i, --input <PATH>     输入源（文件或 "-" 表示 stdin）[default: 从配置文件读取]
//   -q, --quiet            关闭进度显示
//   -v, --verbose...       详细输出（-v debug, -vv trace）
//
// Examples:
//   sqllog2db run -c config.toml                             从目录处理所有 .log 文件
//   cat /var/log/dm/2025-05-21.log | sqllog2db run --input -  管道输入
//   sqllog2db run -c config.toml --set sqllog.path="/tmp/log" 覆盖输入路径
//   sqllog2db run -c config.toml -q                           静默模式（仅错误输出）
```

---

*Feature research for: sqllog2db v1.10 CLI quality improvements*
*Researched: 2026-05-21*
