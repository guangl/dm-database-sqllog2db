# 配置参考

本文档描述 sqllog2db 中所有可用的配置选项。配置文件使用 TOML 格式编写。默认配置由 `sqllog2db init -o config.toml --force` 生成。以下每节记录一个配置块，包含字段、默认值和使用说明。

---

## [sqllog]

指定要处理的输入日志文件。

```toml
[sqllog]
# 达梦 SQL 日志文件路径。可以是单个文件或目录。
path = "sqllogs"
# 可选的文件匹配 glob 模式（如 "*.log"）
# pattern = "*.log"
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `path` | String | *（必填）* | 日志文件路径或包含 `.log` 文件的目录 |
| `pattern` | String | `null` | 当 `path` 为目录时用于筛选文件的 glob 模式 |

**说明：** 自动化脚本中建议使用绝对路径。相对路径从当前工作目录解析。目录扫描是递归的，默认查找所有 `.log` 文件。

---

## [logging]

控制应用日志和错误日志行为。

```toml
[logging]
# 日志级别：trace、debug、info、warn、error
level = "info"
# 可选的解析失败错误日志文件
# file = "error.log"
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `level` | String | `"info"` | 最低日志级别（trace/debug/info/warn/error） |
| `file` | String | `null` | 可选的解析错误输出文件路径 |

**说明：** 生产环境建议设置 `level = "warn"` 以减少输出噪音。解析错误是非致命的，单独记录；单条记录解析失败不会停止整个导出。

---

## [filter.include] 和 [filter.exclude]

记录级过滤：include 规则使用 AND 语义，exclude 规则使用 OR 否决。

```toml
[filter]
# 时间范围过滤（在 [filter] 级，不在 include/exclude 内）
start_ts = "2025-04-15 00:00:00"
end_ts = "2025-04-16 23:59:59"
max_record_limit = 0

[filter.include]
# 每条过滤规则是一组字段-值对（规则内使用 AND）
# 多条规则之间也使用 AND
USERNAME = "APP_USER"
# APPGROUP = "WEB"
# IP_ADDRESS = "192.168.1.100"

[filter.exclude]
# 任意单条 exclude 匹配即丢弃记录（OR 否决）
# STMT_TYPE = "SEL"
# USERNAME = "BATCH_USER"
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `USERNAME` | String | `null` | 按数据库用户名过滤（支持正则） |
| `APPGROUP` | String | `null` | 按应用组名称过滤 |
| `IP_ADDRESS` | String | `null` | 按客户端 IP 地址过滤 |
| `STMT_TYPE` | String | `null` | 按语句类型过滤（INS/UPD/DEL/SEL） |
| `TAG` | String | `null` | 按日志标签过滤 |
| `start_ts` | String | `null` | 过滤此时间戳之后的记录（位于 `[filter]` 级——不在 include/exclude 下） |
| `end_ts` | String | `null` | 过滤此时间戳之前的记录（位于 `[filter]` 级——不在 include/exclude 下） |
| `max_record_limit` | usize | `0` | 最多处理的记录数（0 = 无限制，位于 `[filter]` 级） |

**说明：** include 和 exclude 组合规则为 `(include_AND) AND (NOT exclude_OR)`。同一过滤规则内的所有字段使用 AND 语义。多条 include 规则之间也使用 AND。exclude 使用 OR 否决：任意匹配即丢弃记录。事务级指标和 SQL 内容过滤器使用两遍预扫描检测事务边界。

---

## [exporter.csv]

CSV 导出配置。当 CSV 和 SQLite 同时配置时，CSV 优先级更高。

```toml
[exporter.csv]
# 输出 CSV 文件路径
file = "output/sqllog.csv"
# 覆盖已存在的文件
overwrite = true
# 列分隔符（默认 ","）
# delimiter = ";"
# 可选：按索引选择特定列（从零开始）
# ordered_indices = [0, 1, 3, 5, 7]
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `file` | String | *（必填）* | 输出 CSV 文件路径 |
| `overwrite` | bool | `false` | 覆盖已存在的文件 |
| `delimiter` | char | `,` | 列分隔符 |
| `ordered_indices` | [usize] | `[]`（所有字段） | 字段投影——从零开始的列索引 |

**说明：** CSV 使用 16 MB `BufWriter` + `itoa` 零分配整数格式化，实现约 520 万条记录/秒的吞吐量。`ordered_indices` 用于选择精确的列顺序和子集。为空表示所有字段按默认顺序输出。

---

## [exporter.sqlite]

SQLite 导出配置。当 CSV 和 SQLite 同时配置时，SQLite 优先级较低。

```toml
[exporter.sqlite]
# 输出 SQLite 数据库路径
file = "output/sqllog.db"
# 目标表名
table = "sqllog_records"
# 删除并重建已存在的表
overwrite = true
# 可选：按索引选择特定列（从零开始）
# ordered_indices = [0, 1, 3, 5, 7]
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `file` | String | *（必填）* | 输出 SQLite 数据库路径 |
| `table` | String | `"sqllog_records"` | 目标表名 |
| `overwrite` | bool | `false` | 删除并重建已存在的表 |
| `ordered_indices` | [usize] | `[]`（所有字段） | 字段投影——从零开始的列索引 |

**说明：** 使用批量 INSERT 配合 PRAGMA 优化（synchronous=OFF、mmap_size、cache_size），实现约 110 万条记录/秒的吞吐量。

---

## 附录：配置行为说明

### 导出器优先级

每次运行只有一个导出器处于活动状态。优先级：CSV > SQLite。当 `[exporter.csv]` 和 `[exporter.sqlite]` 都配置时，CSV 优先。移除或注释 CSV 节即可使用 SQLite。

### 处理管道快速路径

当没有启用任何过滤器时，整个处理管道通过单个 `pipeline.is_empty()` 检查绕过。这意味着可选功能在禁用时不会增加任何运行时开销。

### 配置验证

运行前使用 `sqllog2db validate -c config.toml` 检查配置。验证会一次性报告所有错误（而非遇错即停）。常见问题：缺少必填字段、无效路径、TOML 语法错误。

### 命令行子命令

sqllog2db 提供三个子命令：

**`sqllog2db init`** — 生成默认配置文件。支持 `-o` 指定输出路径、`-f` 强制覆盖。

**`sqllog2db validate`** — 校验配置文件。`-c` 指定配置文件路径，一次性报告所有校验错误。

**`sqllog2db run`** — 执行日志导出。`-c` 指定配置文件路径，`-q` 静默模式。

示例见[快速入门指南](quickstart.md)。

### 字段顺序

各 TOML 节中的字段可以按任意顺序排列。配置使用 `serde` 反序列化，与顺序无关。可选节可以完全省略——所有字段采用默认值。

### 环境变量

- `SQLLOG2DB_CONFIG` — 设置默认配置文件路径（可被 `-c` 标志覆盖）
- `NO_COLOR` — 禁用彩色终端输出
- `RUST_LOG` — 使用 `env_logger` 时覆盖日志级别
