# 研究总结：sqllog2db v1.10 CLI 质量改进

**项目:** sqllog2db v1.10 质量/UX 改进
**领域:** 达梦数据库 SQL 日志处理 CLI 工具（Rust 单线程流式架构）
**研究日期:** 2026-05-21
**置信度:** HIGH（基于完整代码分析 + 生态最佳实践 + 上游 crate 源码验证）

## 执行摘要

sqllog2db 是一个成熟的单线程流式 CLI 工具，当前版本在功能和正确性上已经完善，但在终端用户体验上有显著短板：进度显示依赖基本 `eprintln!`、缺少 stdin 管道输入、错误信息缺少行号和修复建议。v1.10 的目标是在不破坏核心性能的前提下补齐这些 UX 短板。

研究确认了 **v1.10 的核心矛盾**：热循环约 5.2M records/sec 的吞吐量使得任何 per-record 开销都必须极其谨慎。进度条更新、错误统计、per-record 日志都不能直接插入热循环。解决方案是复用已有的 `trailing_zeros() >= 10` 模式（每 1024 条检查一次），把进度更新和中断检测对齐。

**关键建议：** 以"先清理后增强"的策略推进。Phase 1 先做零风险的技术债清理和 --help 文本增强（可并行），Phase 2 做核心错误处理架构变更（fatal/non-fatal 分类），Phase 3 做 stdin 和进度条两个独立功能，Phase 4 做最终打磨。这样每一步都有明确验证点，且风险隔离。

**主要风险：** stdin 支持受上游 `dm-database-parser-sqllog` crate 的 API 限制（只支持 `fs::read` 路径，不支持 `io::Read`），需要通过 `/dev/stdin` 路径映射绕行。事务级过滤与 stdin 模式不兼容（无法 pre-scan），需要在文档和运行时明确警告。

## 主要发现

### 1. 推荐的栈

**唯一新增依赖：** `indicatif 0.18` — Rust CLI 生态的进度条事实标准。MultiProgress 支持多级进度条（文件级 + 总进度），自动检测 TTY，在非终端下自动退化为文本。增加的二进制体积约 120KB。

**不需要的流行 crate：**
- `miette` — 太重，错误上下文 (file:line) 用 thiserror 足够
- `anyhow` / `eyre` — 会丢失变体分发能力（当前用于 exit code）
- `atty` — stdlib 的 `IsTerminal` 自 Rust 1.70 起已稳定，项目 MSRV 1.85
- `termcolor` / `owo-colors` — 只需要 2-3 个 ANSI 码，不值得加依赖
- `tracing` — 在单线程流式架构中 span 开销无收益

### 2. 功能优先级与复杂度评估

**P0（必须，按实现顺序）：**

| # | 功能 | 复杂度 |
|---|------|--------|
| 1 | 技术债清理（FIX-01/02/03）— 删除死代码、拒绝废弃配置段 | LOW |
| 2 | 错误类型细化（ERR-01）— 添加 `line_number`、`suggestion` 字段 | LOW |
| 3 | 非致命错误继续处理（ERR-02）— exporter 错误从 `?` 改为 `match` + continue | MEDIUM |
| 4 | --help 增强（UX-03）— 添加 examples、value_hint | LOW |
| 5 | 更好的错误上下文（UX-04）— 行号、文件路径、修复建议 | MEDIUM |
| 6 | stdin 管道输入（PIPE-01）— `--input -` 支持 | MEDIUM |

**P1（应该）：**

| # | 功能 | 复杂度 |
|---|------|--------|
| 7 | 进度条显示（UX-01）— indicatif MultiProgress，每 1024 条更新 | MEDIUM |
| 8 | 错误统计摘要（UX-02）— 总记录数、错误数、速率、耗时 | LOW |
| 9 | 管道模式自动退化为文本 — is_terminal 检测 | LOW |

**P2（推迟）：** 并行模式不乱刷屏、错误分组汇总、stdout 美化。

### 3. 架构集成点与风险

**四个主要集成点：**

| 功能 | 集成文件 | 风险 |
|------|----------|------|
| 错误类型细化 | `src/error.rs` | LOW — 添加 Option 字段和 `suggestion()` 方法 |
| 非致命错误继续 | `src/cli/run/processor.rs` | LOW — 模式与已有 parse error 处理相同 |
| stdin 管道输入 | `src/cli/run/mod.rs`, `src/config/sqllog.rs` | MEDIUM — 事务级过滤不兼容 |
| 进度条指示 | `src/cli/run/processor.rs` | LOW — 复用 `trailing_zeros() >= 10` 检查点 |

**高风险的集成组合：** stdin + 事务级过滤。stdin 不可 pre-scan，事务级过滤退化到逐条记录匹配，失去"保留整条事务"的语义。需要运行时警告。

### 4. 关键陷阱

| # | 陷阱 | 规避策略 |
|---|------|----------|
| 1 | 错误类型重构时丢失 fatal/non-fatal 界限 | 先设计 `Error::is_fatal()`，再一次性重构热循环。ERR-01 和 ERR-02 必须在同阶段做 |
| 2 | Stdin 与 LogParserBuilder 文件路径假设冲突 | 使用 `/dev/stdin` 路径映射，跳过 pre-scan，限顺序模式。不能直接传 "-" |
| 3 | 进度条更新插入热循环导致 50-100 倍性能下降 | 每 1024 条更新一次，复用 `trailing_zeros() >= 10` 模式。`cargo bench` 验证退化 < 5% |
| 4 | 错误信息过度工程化（数值错误码） | 不引入 E001/E002 编码。用 thiserror Display + `suggestion()` 方法 |
| 5 | 死代码清理遗漏测试引用或配置兼容 | 三步法：`cargo build` -> `cargo test` -> `grep -r` |
| 6 | 热路径中新增操作破坏内联和零成本抽象 | 保证热路径无 `String::new()`、`clone()`、`Mutex::lock()`、`HashMap::insert()` |

## 对路线图的建议

### Phase 1: 基础清理与文本增强（可并行）

| 子任务 | 文件 | 复杂度 |
|--------|------|--------|
| FIX-01/02/03 技术债清理 | `error.rs`, `config.rs` | LOW |
| UX-03 --help 增强 | `cli/opts.rs` | LOW |

**规避陷阱：** Pitfall 5（死代码清理遗漏）

### Phase 2: 错误体系重构（ERR-01 + ERR-02 必须同阶段完成）

| 子任务 | 文件 | 复杂度 |
|--------|------|--------|
| ERR-01 错误类型细化 | `error.rs` | LOW |
| ERR-02 非致命错误继续 | `processor.rs`, `run/mod.rs` | MEDIUM |

**关键验证：** 注入 IO 错误后确认继续处理；磁盘满场景确认正确报错。
**规避陷阱：** Pitfall 1（fatal/non-fatal 界限）、Pitfall 4（错误码过度工程化）、Pitfall 6（热路径零成本破坏）

### Phase 3: 核心新功能（可并行）

| 子任务 | 文件 | 复杂度 |
|--------|------|--------|
| PIPE-01 stdin 输入 | `sqllog.rs`, `run/mod.rs`, `preflight.rs` | MEDIUM |
| UX-01 进度条 | `processor.rs` | MEDIUM |
| UX-04 错误上下文 | `error.rs`, `main.rs` | MEDIUM |

**关键验证：** stdin 模式 `echo "..." | cargo run -- run -c config.toml -- -`；`cargo bench` 退化 < 5%。
**规避陷阱：** Pitfall 2（stdin + 路径冲突）、Pitfall 3（热循环性能退化）

### Phase 4: 最终打磨

| 子任务 | 文件 | 复杂度 |
|--------|------|--------|
| UX-02 统计摘要 | `run/mod.rs` | LOW |

**依赖：** 需要 Phase 2 提供的累计错误计数。

### 阶段排序依据

1. **技术债最先清理：** 零风险变更，如果放到最后可能被忽略
2. **错误体系先于新功能：** ERR-01/02 是错误处理基座，stdin 和进度条都要依赖这个基座
3. **stdin 和进度条可以并行：** 两个功能改动独立文件，没有共享逻辑
4. **统计摘要放最后：** 依赖 ERR-02 提供的累计错误计数

### 开放问题

1. **stdin + 事务级过滤的具体行为？** 推荐方案：发警告 + degrade 到逐条记录匹配，不做硬性拒绝。
2. **`dm-database-parser-sqllog` 上游修改的可行性？** `/dev/stdin` 路径映射工作但依赖 `fs::read` 将整个 stdin 加载到内存。对于 GB 级管道输入，这会打破"恒定内存"的承诺。是否向上游 PR 添加 `from_reader()` API 是 v1.10 之后的事。
3. **进度条风格选择？** MultiProgress（文件级 + 总进度）是理想方案，但增加了实现复杂度。第一版建议用单行 spinner，后续再升级。
4. **配置 `[template]` 段的拒绝时机？** 建议在 `validate()` 方法中检测并返回友好错误信息，而非使用 `#[serde(deny_unknown_fields)]`。

## 置信度评估

| 领域 | 置信度 | 说明 |
|------|--------|------|
| 技术栈 | HIGH | 所有推荐 crate 的 API 和版本已通过 crates.io 和 docs.rs 验证 |
| 功能 | HIGH | 基于完整代码分析 + 通用 CLI 工具最佳实践 |
| 架构 | HIGH | 所有模块代码已逐行阅读，数据流和组件边界已绘制 |
| 陷阱 | HIGH | 基于实际代码模式推导，性能特征来自项目基准测试数据 |

### 已知缺口

- **`indicatif` + log 输出交错问题：** 理论上 `init_logging(log_to_stdout=false)` 已配置日志输出到文件。但如果用户配置了日志到 stderr，进度条和日志输出会交错。
- **`/dev/stdin` 在 macOS 上的实际行为：** 需要在 macOS + Docker 双平台上验证。
- **ANSI 颜色在 Windows 终端的兼容性：** 由于达梦生态主要在 Linux，这个风险可以接受。

---

*研究完成日期: 2026-05-21*
*可用于路线图: 是*
