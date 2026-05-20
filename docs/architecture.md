# 架构说明

sqllog2db 是一个解析达梦数据库 SQL 日志并导出为 CSV 或 SQLite 的命令行工具。本文档描述项目的整体架构、数据流、模块划分和关键抽象。面向希望深入理解内部设计的开发者和贡献者。

## 数据流

工具的数据流分为四个阶段：**发现 → 解析 → 管道处理 → 导出**。以下 ASCII art 图展示了完整的数据流路径。

```
SQL 日志文件 (.log)
    ↓ SqllogParser — 文件发现与排序（src/parser.rs）
    ↓ dm-database-parser-sqllog — 逐行解析（外部 crate）
    ↓ Sqllog 记录
    ↓ Pipeline — 可选过滤器/处理器链（src/features/）
    │ ├─ (空) ───────────────────── 零开销快速路径
    │ └─ FilterProcessor ────────── 过滤器/模板处理
    ↓ ExporterManager — 路由到活跃导出器（src/exporter/）
    ▼
CSV 输出（src/exporter/csv.rs）或 SQLite 输出（src/exporter/sqlite.rs）
```

**设计要点：**

- 流式处理：记录逐行读取，内存占用恒定，不受文件大小影响——100 MB 和 100 GB 日志文件消耗相同的峰值内存。
- 当管道（Pipeline）为空（无过滤器、模板或图表）时，热循环通过 `pipeline.is_empty()` 检查跳过所有功能逻辑，实现零开销快速路径。
- stats 和 digest 命令读取已导出的输出文件（CSV 或 SQLite），经过简化的分析路径，不再重新解析原始日志。

## 模块划分

项目按职责分为 5 大核心模块和若干支撑模块。

### 配置层 — `src/config.rs`

**职责：** 加载 TOML 配置文件，验证配置完整性，应用命令行覆盖。

**关键抽象：**
- `Config`：顶层配置结构，包含所有子配置段
- `validate_and_compile()`：验证配置并编译正则表达式

**特性：**
- 嵌套子表支持（v1.4+）：`[filter.include]`、`[filter.exclude]`、`[template]`、`[charts]` 为顶级段
- 命令行覆盖：`--set KEY=VALUE` 无需编辑文件
- 向后兼容：通过 `RawFiltersFeature` 中间结构支持旧版扁平格式

### CLI / 编排层 — `src/cli/`

**职责：** 解析命令行参数，分派子命令，编排整体工作流。

**结构：**
- `run.rs` — `handle_run()`：主编排逻辑（加载配置 → 构建管道 → 预扫描 → 流式导出）
- `stats.rs` — 按文件统计与慢查询分析
- `digest.rs` — SQL 指纹聚合
- `init.rs` — 生成默认配置
- `validate.rs` — 验证配置文件
- `show_config.rs` — 查看当前配置

**模式：** CLI handler 函数以 `handle_` 为前缀（`handle_run`、`handle_stats` 等）。

### Pipeline / 特性层 — `src/features/`

**职责：** 实现可选的记录处理链——包括过滤器、SQL 参数归一化和模板聚合。

**关键抽象：**
- `LogProcessor` trait：可插拔的记录处理接口，定义 `process_with_meta()` 方法
- `Pipeline`：记录处理器的有序链，`is_empty()` 判断是否需要处理
- `FiltersFeature`：包含/排除过滤器（AND/OR 语义）
- `normalize_template`：SQL 指纹归一化（去除注释、折叠 IN-list、关键字大写、合并空白）

**模式：**
- 短路语义：任一 processor 返回跳过信号即停止链
- 零开销快路径：`pipeline.is_empty()` 检查避免热循环中不必要的函数调用

### 导出层 — `src/exporter/`

**职责：** 将已处理的记录写入目标后端（CSV 或 SQLite）。

**关键抽象：**
- `Exporter` trait：导出器接口，三阶段生命周期 `initialize()` → `export_one_preparsed()` → `finalize()`
- `ExporterKind` 枚举：静态分派（`match`）而非动态派发（`Box<dyn Trait>`），利于热路径内联

**实现：**
- `CsvExporter`：16 MB `BufWriter` + `itoa` 零分配整数格式化，~520 万条/秒
- `SqliteExporter`：批量 INSERT + PRAGMA 优化（synchronous=OFF、mmap_size、cache_size），~110 万条/秒

**优先级：** CSV > SQLite。同时配置时仅 CSV 生效。

### 图表层 — `src/charts/`

**职责：** 从模板聚合数据生成四类 SVG 图表。

**图表类型：**
- 频率柱状图（`frequency_bar.rs`）— Top-N SQL 模板按执行次数排序
- 延迟直方图（`latency_hist.rs`）— 每类模板的执行时间分布
- 趋势折线图（`trend_line.rs`）— 模板执行频率随时间变化
- 用户饼图（`user_pie.rs`）— 按数据库用户的查询占比

**技术栈：** plotters 库，SVG-only 后端（不需要系统字体或图片库）。

### 支撑模块

| 模块 | 路径 | 职责 |
|------|------|------|
| 错误处理 | `src/error.rs` | 类型化错误枚举 `Error`、`pub type Result<T>` |
| 解析器 | `src/parser.rs` | 日志文件发现、排序、迭代 |
| 日志 | `src/logging.rs` | 应用日志和错误日志 |
| 工具库 | `src/lib.rs` | 模块注册和公共导出 |

## 关键抽象

### Exporter trait + ExporterKind 枚举

输出后端接口定义。`ExporterKind` 使用静态分派（`match` 语句）替代动态派发（`Box<dyn Trait>`），编译器可以为热路径中每种变体生成优化的内联代码。三阶段生命周期（初始化 → 逐条写入 → 收尾）提供了清晰的资源管理边界。

### LogProcessor trait + Pipeline

可插拔的记录处理链。每个 `LogProcessor` 实现 `process_with_meta()` 方法处理单条日志记录。`Pipeline` 将多个处理器串联为有序链。关键设计：`is_empty()` 允许在配置无过滤/模板需求时完全绕过管道，实现零开销。

### FieldMask

`u16` 位掩码用于字段投影。16 个位对应 15 个输出字段（位 15 保留）。在整个管道中传递，控制每个字段是否被写入。提供 `contains(index)` 和 `set(index)` 方法，编译期内联。

### TemplateAggregator

流式模板统计引擎。累积每个模板的出现次数、首末时间戳和代表性 SQL 示例。使用 `hdrhistogram`（高动态范围直方图）以约 24 KB/模板的紧凑存储保存执行延迟分布，相比原始 `Vec<u64>` 节省约 1600 倍空间。通过 `finalize()` 产出最终统计结果。


## 性能设计

### 流式架构

单线程流式处理是核心性能策略。每条记录被解析后立即传递给管道和导出器，不存入内存。这意味着文件大小的增长不会影响峰值内存——无论 100 MB 还是 100 GB 的日志文件，内存曲线是平直的水平线。

### 零开销快路径

当没有配置任何过滤器、模板或图表时，热循环中的 `pipeline.is_empty()` 检查确保所有功能逻辑被跳过。此时记录不经处理直接流式写入导出器，性能接近裸文件复制的水平。

### CSV 导出优化

- 16 MB `BufWriter`：大幅减少系统调用次数
- `itoa` 零分配整数格式化：整数直接写入缓冲区，无需分配中间字符串
- `memchr` SIMD 加速：字节搜索处理 CSV 转义，比逐字节扫描快数倍

### 编译优化

Release 构建采用激进优化：
- `opt-level = 3`：最高代码优化级别
- `lto = "fat"`：跨 crate 的链接时优化
- `codegen-units = 1`：单一代码生成单元，最大化内联机会
- `panic = "abort"`：移除恐慌展开代码
- `strip = "symbols"`：剥离调试符号

最终二进制文件约 5 MB，无外部运行时依赖。

### 基准性能

| 场景 | 吞吐量 | 说明 |
|------|--------|------|
| CSV（合成数据） | ~520 万条/秒 | criterion 基准，50k 记录 |
| SQLite（合成数据） | ~110 万条/秒 | 批量 + PRAGMA 优化 |
| 真实文件（1.1 GB） | ~155 万条/秒 | ~300 万条记录，NVMe SSD |

## 错误处理策略

### 分层错误类型

顶层 `Error` 枚举通过 `thiserror` 派生宏包含所有子错误变体：

- `Config(ConfigError)` — 配置加载和验证错误
- `File(FileError)` — 文件打开和读写错误
- `Parser(ParserError)` — 日志解析错误
- `Export(ExportError)` — 导出写入错误
- `Io(io::Error)` — 底层 I/O 错误
- `Interrupted` — 用户中断（Ctrl+C）

每个子错误变体包含上下文字段（`path: PathBuf`、`reason: String`），而非裸字符串，便于故障排查。

### 非致命解析错误

解析失败不会中断整个导出过程。无法解析的行被记录到错误日志文件（配置中的 `[error] file`），工具继续处理下一行。在运行结束时汇总各文件的解析成功率。

### 退出码

| 退出码 | 含义 |
|--------|------|
| 0 | 成功 |
| 2 | 配置错误 |
| 3 | 文件/解析错误 |
| 4 | 导出错误 |
| 130 | 用户中断（Ctrl+C） |

---

### 依赖关系

模块间的依赖遵循单向分层原则：

```
src/error.rs ←──────────────── 所有模块（横切关注点）
src/config.rs ←─ src/cli/ ←─ src/features/ ←─ src/exporter/
                                   ↑
                              src/charts/
```

配置层（config）被 CLI 层引用，CLI 层协调 Pipeline 和 Exporter 的具体实例。图表层（charts）作为 Pipeline 的下游消费者，仅在模板聚合完成后被触发。

---

*架构文档在 Phase 25 中编写，反映项目 v1.6 的代码结构。模块级概述，不深入特定 struct 字段或 trait 方法签名。*
