# 快速入门指南

本指南带你了解 sqllog2db 的常见使用场景。每个场景展示从配置生成到输出验证的完整工作流程。如需最简三命令概览，请参见 [README](../README.md)。

## 环境准备

通过 rustup 安装 Rust：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

安装 sqllog2db：

```bash
cargo install dm-database-sqllog2db
```

验证安装：

```bash
sqllog2db --version
```

备选方案：从源码构建：

```bash
git clone https://github.com/guangl/sqllog2db
cd sqllog2db
cargo build --release
```

二进制文件约 5 MB，静态链接，位于 `target/release/sqllog2db`。

从源码克隆时，`sqllogs/` 目录下包含示例日志文件。生产环境请使用你自己的达梦 SQL 日志。

---

## 场景一：导出 SQL 日志到 CSV

将达梦 SQL 日志导出为 CSV 以供分析或归档。

**步骤 1：生成默认配置**

```bash
sqllog2db init -o config.toml --force
```

预期输出：

```
Config written to config.toml
```

**步骤 2：配置 CSV 导出**

编辑 `config.toml`：

```toml
[sqllog]
path = "sqllogs"

[exporter.csv]
file = "output/sqllog.csv"
overwrite = true
```

**步骤 3：验证配置**

```bash
sqllog2db validate -c config.toml
```

预期输出：

```
Config validation passed
```

**步骤 4：运行导出**

```bash
sqllog2db run -c config.toml
```

预期输出：

```
[INFO] Starting export...
[INFO] Processing: sqllogs/DM_DMSQL_202504_01.log
[INFO] Processed 100000 records...
[INFO] Processed 200000 records...
[INFO] Export complete: 2,372,459 records in 8.87s (267,525 records/sec)
```

**步骤 5：验证输出**

```bash
wc -l output/sqllog.csv
head -5 output/sqllog.csv
```

**故障排查：** 如果看到 "Config validation failed: sqllog.path"，请确保 `path` 指向存在的目录或文件。自动化脚本中建议使用绝对路径。

---

## 场景二：导出到 SQLite 数据库

导出到 SQLite 以便使用 SQL 进行分析。

**步骤 1：配置 SQLite 导出**

编辑 `config.toml`：

```toml
[sqllog]
path = "sqllogs"

[exporter.sqlite]
file = "output/sqllog.db"
table = "sqllog_records"
overwrite = true
```

**步骤 2：验证并运行**

```bash
sqllog2db validate -c config.toml
sqllog2db run -c config.toml
```

**步骤 3：验证并查询**

```bash
# 统计总记录数
sqlite3 output/sqllog.db "SELECT COUNT(*) FROM sqllog_records;"

# 按查询数量查看 Top 5 用户
sqlite3 output/sqllog.db "SELECT USERNAME, COUNT(*) AS cnt FROM sqllog_records GROUP BY USERNAME ORDER BY cnt DESC LIMIT 5;"

# 查看 Top 10 最慢查询
sqlite3 output/sqllog.db "SELECT SQL_TEXT, ELAPSED FROM sqllog_records ORDER BY ELAPSED DESC LIMIT 10;"
```

预期用户排名输出：

```
HIHIS|1748411
BBP|513287
HINIS|83921
BLC|18342
SYSDBA|5234
```

**故障排查：** 如果找不到 `sqlite3`，请通过 `brew install sqlite`（macOS）或系统包管理器安装。

---

## 场景三：按文件统计与慢查询分析

分析导出结果以识别性能模式。

**步骤 1：运行 stats 命令**

```bash
sqllog2db stats output/sqllog.csv --top-slow 10
```

预期按文件统计表：

```
File                                          Lines      Parsed    Errors    Elapsed
sqllogs/DM_DMSQL_202504_01.log                1,523,421  1,523,421  12        3.42s
sqllogs/DM_DMSQL_202504_03.log                1,487,233  1,487,233  8         3.21s
sqllogs/DM_DMSQL_202504_05.log                1,521,876  1,521,876  15        3.35s
```

预期 Top 10 最慢查询：

```
Rank  SQL_TEXT                         ELAPSED(ms)  USERNAME   START_TIME
1     SELECT * FROM ORDERS WHERE ...   12,345       APP_USER   2025-04-15 14:23:01
2     INSERT INTO PAYMENTS ...         8,901        SYS_USER   2025-04-15 14:25:33
```

**步骤 2：按维度分组**

```bash
# 按用户分组
sqllog2db stats output/sqllog.csv --group-by user

# 按应用分组
sqllog2db stats output/sqllog.csv --group-by app

# 按时间范围过滤
sqllog2db stats output/sqllog.csv --from "2025-04-15" --to "2025-04-16"
```

注意：`--group-by` 标志使用小写值（`user`、`app`、`ip`）。这与 `[filter]` 配置节使用大写字段名（`USERNAME`、`APPGROUP`、`IP_ADDRESS`）不同。过滤器字段命名请参见[配置参考](config-reference.md)。

利用此功能识别性能瓶颈、最活跃用户和易出错的日志文件。

---

## 场景四：SQL 模板聚合与图表生成

归一化 SQL 查询以识别结构模式并生成 SVG 图表。

**步骤 1：启用模板分析和图表**

```toml
[sqllog]
path = "sqllogs"

[template]
enable = true
normalize_template = true
aggregator_mode = "hdrhistogram"
latency_buckets = [1, 5, 10, 50, 100, 500, 1000, 5000]

[charts]
output_dir = "charts/"
top_n = 10
frequency_bar = true
latency_hist = true
trend_line = true
user_pie = true

[exporter.csv]
file = "output/sqllog.csv"
overwrite = true
```

**步骤 2：运行带模板聚合的导出**

```bash
sqllog2db run -c config.toml
```

预期额外输出：

```
[INFO] Template aggregation: 245 unique SQL fingerprints
[INFO] Chart generated: charts/top_n_frequency.svg
[INFO] Chart generated: charts/latency_histogram_*.svg
[INFO] Chart generated: charts/frequency_trend.svg
[INFO] Chart generated: charts/user_schema_pie.svg
```

**步骤 3：查看模板摘要**

```bash
# 如果使用 SQLite 输出
sqllog2db digest output/sqllog.db
```

预期模板摘要：

```
Template                                        Count   Avg(ms)   P50(ms)   P95(ms)   P99(ms)
SELECT * FROM HI_BD_TASK_FU WHERE ID_TASK = ?   12,345  342       215       891       2,341
INSERT INTO HI_BD_SIPA_FU_RULE ...              8,901   156       120       445       980
```

**步骤 4：查看输出**

- `output/template_summary.csv` — CSV 摘要（如果使用 CSV 导出器）
- `output/sqllog.db` — SQLite，包含 `sqllog_records` 和 `_templates` 表
- `charts/` — SVG 图表文件（频率柱状图、延迟直方图、趋势折线图、用户饼图）

利用模板聚合理解 SQL 执行模式、识别热点查询并可视化工作负载分布。

---

## 故障排查

### 配置验证失败

- 运行 `sqllog2db validate -c config.toml` 查看具体错误
- 检查 `sqllog.path` 是否存在且可读
- 确保输出目录存在（sqllog2db 不会自动创建中间目录）
- 验证 TOML 语法：节标题使用 `[方括号]`，值使用 `=`

### "未找到 .log 文件"

- 验证 `[sqllog].path` 中的路径是否正确
- 使用绝对路径：`/home/user/logs/` 而非 `../logs/`
- 检查文件是否具有 `.log` 扩展名

### 导出性能较慢

- 确保处理管道为空（无 `[filter]`、`[template]` 或 `[charts]` 节）以获得最大速度
- CSV 导出比 SQLite 更快（约 520 万条/秒 vs 约 110 万条/秒）
- 使用 NVMe SSD 获得最佳吞吐量
- 对于大数据集，文件 I/O 是主要瓶颈

### 输出中出现解析错误

- 解析错误是非致命的：工具继续处理后续记录
- 错误记录到应用日志中（检查 `[logging]` 配置）
- GB18030/GBK 编码的文件会自动检测和解码
- 使用 `sqllog2db stats` 查看每个文件的错误计数

### 模板聚合产生过多模板

- 增加 `[charts]` 中的 `top_n` 以显示更多模板
- 使用 `sqllog2db digest --min-count 100` 过滤罕见模板
- 在模板聚合前添加过滤器以缩小数据范围
