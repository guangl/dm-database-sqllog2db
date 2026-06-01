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
inputs = ["sqllogs"]

[exporter.csv]
file = "output/sqllog.csv"
overwrite = true
```

**步骤 3：验证配置**

```bash
sqllog2db validate -c config.toml
```

通过时静默退出（无输出，exit 0）；失败时输出 `[FAIL] <字段>: <原因>` 并以非零码退出。

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
inputs = ["sqllogs"]

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

## 场景三：使用过滤器精确导出

应用记录级和事务级过滤器，仅导出符合条件的 SQL 记录。

**步骤 1：配置过滤器**

编辑 `config.toml`：

```toml
[sqllog]
inputs = ["sqllogs"]

[filter]
enable = true

[filter.include]
statements = ["INS", "UPD", "DEL"]
min_runtime_ms = 100

[filter.exclude]
users = ["SYSDBA", "MONITOR"]

[exporter.csv]
file = "output/filtered.csv"
overwrite = true
```

**步骤 2：验证并运行**

```bash
sqllog2db validate -c config.toml
sqllog2db run -c config.toml
```

**步骤 3：使用 SQL 内容过滤器**

事务级 SQL 内容过滤需要两遍预扫描：

```toml
[filter.sql_filter]
includes = ["FROM ORDERS", "JOIN PAYMENTS"]
excludes = ["pg_catalog", "information_schema"]
min_runtime_ms = 500
```

工具先扫描所有文件收集匹配的事务 ID，再于第二遍中应用全部过滤器。

**通过外部工具分析输出：**

```bash
# 使用 sqlite3 查询 Top 5 最慢的写操作
sqlite3 output/filtered.db "SELECT SQL_TEXT, ELAPSED FROM sqllog_records WHERE STMT_TYPE IN ('INS','UPD','DEL') ORDER BY ELAPSED DESC LIMIT 5;"

# 使用 csvkit 工具
csvstat output/filtered.csv
```

---

## 场景四：SQL 统计分析

使用 `stats` 子命令对 SQL 日志进行统计分析，生成慢 SQL 报告和高频 SQL 报告。

**步骤 1：配置输入和输出目录**

编辑 `config.toml`（复用 `run` 命令的配置格式）：

```toml
[sqllog]
inputs = ["sqllogs"]

[exporter.csv]
file = "output/sqllog.csv"
overwrite = true
```

**步骤 2：运行统计分析**

```bash
sqllog2db stats -c config.toml
```

默认输出 Top 20 条，可用 `--top` 调整：

```bash
sqllog2db stats -c config.toml --top 10
```

**步骤 3：查看输出**

CSV 模式在 `[exporter.csv].file` 的同级目录（本例为 `output/`）生成：

```
output/slow_sql.csv       — Top N 最慢 SQL（字段：sql_text, elapsed_ms, timestamp）
output/frequent_sql.csv   — Top N 高频 SQL（字段：normalized_sql, call_count, avg_elapsed_ms, max_elapsed_ms）
```

示例查看：

```bash
head -5 output/slow_sql.csv
head -5 output/frequent_sql.csv
```

**使用 SQLite 输出：**

```toml
[sqllog]
inputs = ["sqllogs"]

[exporter.sqlite]
database_url = "output/sqllog2db.db"
table_name = "sqllog_records"
overwrite = true
```

SQLite 模式在同一数据库中写入 `slow_sql` 和 `frequent_sql` 表，可直接用 SQL 查询：

```bash
sqlite3 output/sqllog2db.db "SELECT * FROM slow_sql LIMIT 5;"
sqlite3 output/sqllog2db.db "SELECT * FROM frequent_sql ORDER BY call_count DESC LIMIT 5;"
```

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

- 确保处理管道为空（无 `[filter]` 节）以获得最大速度
- CSV 导出比 SQLite 更快（约 520 万条/秒 vs 约 110 万条/秒）
- 使用 NVMe SSD 获得最佳吞吐量
- 对于大数据集，文件 I/O 是主要瓶颈

### 输出中出现解析错误

- 解析错误是非致命的：工具继续处理后续记录
- 错误记录到应用日志中（检查 `[logging]` 配置）
- GB18030/GBK 编码的文件会自动检测和解码
