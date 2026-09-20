# 架构说明

sqllog2db 将达梦数据库 SQL 日志流式导出为 Parquet 或 CSV。代码按命令入口、运行编排、记录处理、输出后端划分；统计分析拥有独立的领域模块。

## 项目目录

```text
src/          生产代码：CLI、配置、引擎、处理管道、导出、统计
tests/        所有测试：unit/、python/ 与根目录集成测试
benches/      性能基准与历史基线
docs/         使用说明、架构、验证记录和 mdBook 站点配置
scripts/      内存检查、基准结果收集脚本
.github/      CI、发布与文档站工作流
target/       本地构建产物与文档站输出（忽略提交）
```

源码按职责分层，`src/` 根目录只保留程序入口：

```text
src/
├── application/       运行编排、预检查、扫描
│   └── engine/        单次导出的生命周期与记录处理
├── cli/               命令行入口与交互
├── config/            配置模型、模板和校验
├── domain/            领域模型与处理管道
│   └── pipeline/      过滤、参数归一化、字段投影
├── infrastructure/    输入、输出、错误和日志边界
│   ├── input/
│   │   └── adapters/  按 feature 编译的日志解析器适配器
│   └── output/        CSV、Parquet 和导出器管理
├── stats/             统计聚合与终端展示
├── lib.rs
└── main.rs
```

`tests/unit/` 按同样的 `application/`、`domain/`、`infrastructure/`、`input/`、`cli/`、`config/` 和 `stats/` 层级组织，测试路径与生产代码职责一一对应。

导出器公共接口和统计位于 `infrastructure/output/mod.rs`，后端选择与管理位于 `infrastructure/output/manager.rs`；CSV 生命周期与字段序列化分为两个文件，Parquet 使用单文件实现。应用编排位于 `application/`；字段投影定义集中在 `domain/pipeline/output.rs`。

## 从哪里开始阅读

| 要了解的行为 | 入口 | 职责 |
| --- | --- | --- |
| 命令如何执行 | `src/main.rs` → `src/cli/mod.rs` | 启动运行时、分派命令、决定退出码 |
| 配置如何生效 | `src/cli/runtime.rs`、`src/config/` | 加载配置、应用 CLI 覆盖、校验、初始化日志 |
| 配置如何生成 | `src/cli/init.rs` | 集中完成文件写入、交互问答、TOML 渲染 |
| 一次导出如何运行 | `src/application/engine/mod.rs` | 输入解析 → 事务预扫描 → 执行 → 汇总 |
| 驱动共享什么 | `src/application/engine/context.rs` | 配置、Pipeline、字段投影、参数替换选项和结果类型 |
| 输入如何解析 | `src/infrastructure/input/mod.rs`、`src/infrastructure/input/adapters/` | 格式识别、feature-gated 解析器适配和有界预取 |
| 统一记录模型 | `src/domain/model.rs` | 将不同日志格式映射为处理链共用的 `LogRecord` |
| 单条记录如何处理 | `src/application/engine/record.rs` | 过滤、参数缓存维护、参数回填、导出和错误计数 |
| 参数如何替换 | `src/domain/pipeline/normalizer.rs` | PARAMS 解析、占位符替换、session/statement 关联 |
| 输出如何写入 | `src/infrastructure/output/` | 导出器选择、字段投影、后端生命周期与写入 |
| 如何统计 SQL | `src/stats/runner.rs` | 流式扫描、慢 SQL/高频 SQL 聚合与展示 |
| 错误如何表达 | `src/infrastructure/error.rs` | 结构化错误、严重程度、建议和有界错误统计 |

## 数据流与职责边界

```text
main → cli::run
         ├─ init     → cli/init.rs → 配置文件
         ├─ validate → config → cli/validate
         ├─ stats    → stats/runner → scanner → 聚合结果
         └─ run      → engine::run
                         ├─ prepare：发现文件、事务预扫描、并发预算
                         ├─ context：构建共享运行配置
                         ├─ streaming → adapter → LogRecord → pipeline → exporter
                         └─ report：汇总结果、写错误日志
```

- `main.rs` 只调用 CLI 并应用退出码。命令分派和终端错误格式放在 `cli/`，业务模块无需依赖 CLI。
- `application/engine/mod.rs` 管理一次导出的生命周期；驱动依赖 `context.rs` 中的共享类型，不反向依赖编排实现。
- `infrastructure/input/resolver.rs` 中的 `InputResolver` 负责输入文件发现与排序；`infrastructure/input/mod.rs` 只负责格式识别、适配器选择和有界预取；`infrastructure/input/adapters/` 下的适配器隔离外部解析器依赖；`application/scanner.rs` 提供预扫描和统计使用的扫描接口。
- `domain/model.rs` 定义解析器无关的 `LogRecord`。过滤、归一化、统计和导出只依赖这个模型，因此新增日志格式不会把外部解析器类型传播到核心模块。
- 解析器通过 Cargo feature 控制：`sqllog`、`driver-jdbc`、`driver-dm-provider` 和聚合 feature `all-parsers`。新增格式时增加一个 feature、在 `infrastructure/input/adapters/` 增加适配器，并在 `infrastructure/input/mod.rs` 增加格式选择分支即可。
- `config/` 聚合配置并负责校验，过滤、归一化、字段投影与统计选项由对应功能模块定义。
- `infrastructure/error.rs` 集中定义结构化错误、严重程度和有界错误统计。

## 执行路径

CSV 和 Parquet 统一由 `application/engine/sequential.rs` 按输入顺序逐文件导出，直接写入配置的单个输出文件。普通文件由一个解析线程预读，单个写入器按顺序消费；队列最多两个批次，每批最多 512 条，按约 1 MiB 的记录容量提前发送。超长单条记录可能超过此目标；stdin 等特殊输入保持同步读取。两种格式共用文件进度、记录速率和错误统计；除 `--quiet` 外均启用进度展示，非终端输出由 indicatif 自动隐藏。

输入切块、输出按行拆分和 CSV 专用并发路径已移除。旧 `max_rows_per_file` 配置会报错，需删除该字段。

## 记录处理与参数替换

`Pipeline` 通过 `LogProcessor` 链执行过滤；空管道使用快速路径。事务级过滤由 `application/engine/prepare.rs` 预扫描命中的事务 ID，再将结果交给主处理阶段。

参数替换拆为三个职责：

`domain/pipeline/normalizer.rs` 集中解析 PARAMS、处理占位符并维护 session/statement 参数关联，按配置的标签决定是否回填。

PARAMS 记录即使被过滤，仍可能需要更新参数缓存。回填结果写入调用方复用的 scratch 缓冲。实现位于 `domain::pipeline::normalizer`，并通过兼容导出保留原有 `pipeline::normalizer` 访问方式。

## 导出与内存

`ExporterConfig::active()` 统一选择活跃后端，供预检查、导出创建共用；`ExporterManager` 直接用枚举持有并调用后端；导出器通过 `initialize()`、记录写入方法和 `finalize()` 管理生命周期。后端实现位于 `infrastructure/output/parquet.rs`、`infrastructure/output/csv.rs`。

解析过程逐条读取，不整文件加载。CSV 使用单个 1 MiB 写缓冲，Parquet 直接通过 Arrow StringBuilder 构建字符串列，并按行数与字节数限制 row group 缓冲。但事务 ID 集合、参数关联缓存和统计聚合仍随不同事务、statement 或 SQL 的数量增长，因此不能将所有配置下的总内存描述为绝对恒定。内存验证方法见 [导出内存检查](export-memory-check.md)。

## 错误与退出码

可恢复的解析错误被记录并跳过，运行继续；致命配置、I/O 或导出错误终止处理。`cli/runtime.rs` 统一输出严重程度和修复建议，`application/engine/report.rs` 输出运行统计及配置的错误日志。

| 退出码 | 含义 |
| --- | --- |
| 0 | 成功 |
| 1 | 处理完成，但有非致命错误 |
| 2 | 致命错误或校验失败 |
| 130 | 用户中断 |

## 修改与验证

新增输出后端从 `infrastructure/output/` 开始；修改执行策略从 `application/engine/sequential.rs` 开始；增加参数语法从 `domain/pipeline/normalizer.rs` 开始；新增日志格式从 `Cargo.toml` 的 feature 和 `infrastructure/input/adapters/` 适配器开始。新增命令应让 CLI 负责参数与展示，让对应领域模块负责处理流程。

所有测试实现统一放在根目录 `tests/`：`tests/unit/` 按 `src/` 模块层次组织 Rust 单元测试，根目录的 `.rs` 文件是 Cargo 集成测试，`tests/python/` 是检查脚本的测试。生产模块仅通过 `#[cfg(test)]` 与 `#[path]` 引用单元测试文件，保留对私有实现的测试能力。测试辅助函数也放在测试目录；性能基准仍在 `benches/`。本地检查：

```bash
cargo fmt --all -- --check
cargo test --locked
python3 -B -m unittest discover -s tests/python -p 'test_*.py' -v
cargo clippy --locked --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
```

文档站直接通过 `docs/book.toml` 构建本目录内容：`mdbook build docs`，输出到 `target/book/`。无需单独维护站点包装目录。

CLI 使用平台默认分配器。
