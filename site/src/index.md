# sqllog2db

[![Crates.io](https://img.shields.io/crates/v/dm-database-sqllog2db?style=flat-square&logo=rust&logoColor=white&label=crates.io&color=d96109)](https://crates.io/crates/dm-database-sqllog2db)
[![CI](https://img.shields.io/github/actions/workflow/status/guangl/sqllog2db/ci.yaml?style=flat-square&logo=github-actions&logoColor=white&label=ci)](https://github.com/guangl/sqllog2db/actions/workflows/ci.yaml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue?style=flat-square)](https://opensource.org/licenses/Apache-2.0)
[![Release](https://img.shields.io/github/v/release/guangl/sqllog2db?style=flat-square&logo=github&logoColor=white&label=release)](https://github.com/guangl/sqllog2db/releases)
[![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-orange?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)

**达梦数据库高性能 SQL 日志解析器**——以恒定内存流式处理数百万条记录，导出为 CSV 或 SQLite，使用内置图表分析查询模式。

---

<!-- 安装 -->

## 安装

```bash
# 从 crates.io 安装（推荐）
cargo install dm-database-sqllog2db

# 或从源码构建
cargo build --release
```

需要 Rust 1.85+。二进制文件大小约 5 MB。

---

<!-- 功能特性 -->

## 功能概览

**解析与导出**——流式处理达梦 SQL 日志（GB18030/GBK），导出为 CSV（16 MB BufWriter + itoa 零分配）或 SQLite（批量事务 + PRAGMA）。单线程，恒定内存。

**过滤与字段控制**——包含/排除过滤器，支持 AND/OR 否决语义。事务级指标和 SQL 内容过滤器。通过 `ordered_indices` 自定义字段投影。

**模板分析与图表**——SQL 指纹归一化，TemplateAggregator 配合 hdrhistogram 统计，CSV+SQLite 双路输出。自动生成四类 SVG 图表。

**配置与性能**——嵌套子表的 TOML 配置（`[filter.include]`、`[template]`、`[charts]`）。管道为空时的零开销快速路径。~520 万条记录/秒的 CSV 吞吐量。

---

<!-- 架构 -->

## 架构

```
SQL Log Files (.log)
      │
      ▼
  SqllogParser          ← 发现文件、逐行迭代
      │
      ▼
dm-database-parser-sqllog  ← 解析每行 → Sqllog 记录
      │
      ▼
   处理管道               ← 空 = 零开销快速路径
   ├─ (空) ────────────────────────────┐
   └─ FilterProcessor ─────────────────►│
                                        │
                                        ▼
                                 ExporterManager
                                 ├─ CSV 导出器  → output.csv
                                 └─ SQLite 导出器 → output.db
```

数据经过四个阶段：**发现** → **解析** → **处理管道**（可选过滤器） → **导出**。当管道为空时，零开销快速路径绕过所有功能逻辑。

---

<!-- 性能 -->

## 性能

| 基准测试 | 记录数/秒 | 数据来源 | 硬件 |
|----------|-----------|----------|------|
| CSV（合成数据） | ~5,200,000 条/秒 | Criterion 基准，50k 条记录 | Apple M 系列 NVMe SSD |
| 真实环境（1.1 GB） | ~1,550,000 条/秒 | 生产 .log 文件，~300 万条 | Apple M 系列 NVMe SSD |

所有基准测试在 Apple Silicon（macOS）配合 NVMe SSD 上运行。流式架构保持内存恒定，不受文件大小影响——100 MB 和 100 GB 日志文件使用相同的峰值内存。

---

<!-- 图表功能 -->

## 图表功能

本工具内置 SQL 模板分析引擎，可自动生成四类 SVG 图表：频率柱状图（Top-N SQL 模板按执行次数排序）、延迟直方图（每类模板的执行时间分布）、趋势折线图（SQL 执行频率随时间变化）和用户饼图（按数据库用户的查询占比）。图表通过 `sqllog2db stats` 命令配合 `--chart` 参数生成，输出到配置的 charts/ 目录。

---

<!-- 演示 -->

## 演示

观看 sqllog2db 实际运行的终端录像：

<script src="https://cdn.jsdelivr.net/npm/asciinema-player@3.8.1/dist/bundle/asciinema-player.min.js"></script>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/asciinema-player@3.8.1/dist/bundle/asciinema-player.css">

<asciinema-player src="asciicast/demo.cast" cols="120" rows="30"></asciinema-player>

录像文件也可下载：[demo.cast](asciicast/demo.cast)

---

## 链接

- [GitHub 仓库](https://github.com/guangl/sqllog2db)
- [crates.io](https://crates.io/crates/dm-database-sqllog2db)
- [变更日志](https://github.com/guangl/sqllog2db/blob/main/CHANGELOG.md)
- [许可证](https://github.com/guangl/sqllog2db/blob/main/LICENSE) — Apache-2.0
- [README](https://github.com/guangl/sqllog2db) — 技术参考与快速入门
- [快速入门指南](https://github.com/guangl/sqllog2db/blob/main/docs/quickstart.md) — 分步使用场景
- [配置参考](https://github.com/guangl/sqllog2db/blob/main/docs/config-reference.md) — 全部配置选项

---

*由 [mdBook](https://rust-lang.github.io/mdBook/) 构建。图表由 plotters 渲染。通过 GitHub Actions 部署。*
