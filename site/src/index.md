# sqllog2db

[![Crates.io](https://img.shields.io/crates/v/dm-database-sqllog2db?style=flat-square&logo=rust&logoColor=white&label=crates.io&color=d96109)](https://crates.io/crates/dm-database-sqllog2db)
[![CI](https://img.shields.io/github/actions/workflow/status/guangl/sqllog2db/ci.yaml?style=flat-square&logo=github-actions&logoColor=white&label=ci)](https://github.com/guangl/sqllog2db/actions/workflows/ci.yaml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue?style=flat-square)](https://opensource.org/licenses/Apache-2.0)
[![Release](https://img.shields.io/github/v/release/guangl/sqllog2db?style=flat-square&logo=github&logoColor=white&label=release)](https://github.com/guangl/sqllog2db/releases)
[![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-orange?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)

**达梦数据库 SQL 日志高性能解析工具** — 流式处理百万级记录，常量内存占用，支持 CSV/SQLite 导出，内置 SQL 模板分析与图表。

---

## 快速安装

```bash
# 从 crates.io 安装
cargo install dm-database-sqllog2db

# 生成配置并运行
sqllog2db init -o config.toml
sqllog2db run -c config.toml
```

需要 Rust 1.85+。二进制文件约 5 MB。

---

## 文档

- [快速入门](quickstart.md) — 从安装到导出，逐步引导完成常见使用场景
- [配置参考](config-reference.md) — 所有配置项的完整说明
- [架构说明](architecture.md) — 数据流、模块划分和关键设计
- [贡献指南](contributing.md) — 环境搭建、编码规约和 PR 提交流程

---

## 链接

- [GitHub 仓库](https://github.com/guangl/sqllog2db)
- [crates.io](https://crates.io/crates/dm-database-sqllog2db)
- [变更日志](https://github.com/guangl/sqllog2db/blob/main/CHANGELOG.md)
- [许可证](https://github.com/guangl/sqllog2db/blob/main/LICENSE) — Apache-2.0

---

*Built with [mdBook](https://rust-lang.github.io/mdBook/). Deployed via GitHub Actions.*
