# 配置参考

本文档描述 sqllog2db 中所有可用的配置选项。配置文件使用 TOML 格式编写。默认配置由 `sqllog2db init -o config.toml --force` 生成。以下每节记录一个配置块，包含字段、默认值和使用说明。

---

## [sqllog]

指定要处理的输入日志文件。

```toml
[sqllog]
# 输入列表：目录、单文件或 glob 模式均可，支持多条目
inputs = ["sqllogs"]
# 多条目示例：
# inputs = ["sqllogs/2025-01/*.log", "sqllogs/2025-02/*.log"]
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `inputs` | [String] | *（必填）* | 输入路径数组，支持目录、单文件或 glob 模式（如 `./logs/2025-*.log`） |

**说明：** 自动化脚本中建议使用绝对路径。相对路径从当前工作目录解析。目录条目递归查找 `.log` 文件，结果按路径排序保证确定性顺序。旧版 `path = "..."` 字段已在 v1.12 移除，请迁移为 `inputs = ["..."]`。

---

## [logging]

控制应用日志（程序自身运行日志）的输出路径、级别和滚动保留。

```toml
[logging]
# 应用日志文件路径
file = "logs/sqllog2db.log"
# 日志级别：trace、debug、info、warn、error
level = "info"
# 日志保留天数（1-365）
retention_days = 7
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `file` | String | `"logs/sqllog2db.log"` | 应用日志文件路径（不可为空） |
| `level` | String | `"info"` | 最低日志级别（trace/debug/info/warn/error） |
| `retention_days` | usize | `7` | 滚动日志保留天数，取值 1-365 |

**说明：** 生产环境建议设置 `level = "warn"` 以减少输出噪音。此处的 `file` 是**应用运行日志**，与**解析错误日志**不同——后者由独立的 `[error]` 段配置（见下）。解析错误是非致命的：单条记录解析失败不会停止整个导出。

---

## [error]（可选）

解析错误日志。单独一个文件路径字段，收集解析失败的记录（纯文本行：`file | error | raw | line`）。省略该段则不写错误日志。

```toml
[error]
# 解析错误输出文件路径
file = "export/errors.log"
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `file` | String | *（该段省略时不写错误日志）* | 解析错误输出文件路径 |

---

## [filter]

过滤功能的总开关及其子表。过滤分两类：**记录级**（`[filter.include]` / `[filter.exclude]`，逐条判断）和**事务级**（`[filter.indicators]` / `[filter.sql]`，命中即保留或丢弃整笔事务，需要两遍预扫描）。所有元数据字段均为**精确字符串匹配**（不支持正则），且取值均为**列表**。

```toml
[filter]
# 是否启用过滤管道（false 时下方所有子表均被忽略，走零开销快速路径）
enable = true
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `enable` | bool | `false` | 过滤功能总开关。为 `false` 时所有过滤子表都不生效 |

---

## [filter.include]

记录级包含过滤。**同一字段内的多个值为 OR**（命中任一即可），**不同字段之间为 AND**（每个已配置字段都必须命中，记录才保留）。

```toml
[filter.include]
# users      = ["SYSDBA"]                      # 用户名
# ips        = ["127.0.0.1", "192.168.1.100"]  # 客户端 IP
# sessions   = ["0x7f41435437a8"]              # 会话 ID（十六进制字符串）
# threads    = ["2188515"]                     # 线程 ID
# statements = ["INS", "UPD", "DEL"]           # 语句类型（见下方说明）
# apps       = ["DMSQL"]                        # 应用名
# tags       = ["SEL", "INS"]                  # 日志标签（与 statements 同义）
# start_ts   = "2023-01-01 00:00:00"           # 时间戳闭区间下界
# end_ts     = "2023-01-01 23:59:59"           # 时间戳闭区间上界
# trxids     = ["257809109", "257809110"]      # 事务 ID
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `users` | [String] | `null` | 要保留的用户名列表 |
| `ips` | [String] | `null` | 要保留的客户端 IP 列表 |
| `sessions` | [String] | `null` | 要保留的会话 ID 列表（十六进制字符串） |
| `threads` | [String] | `null` | 要保留的线程 ID 列表 |
| `statements` | [String] | `null` | 要保留的语句类型列表（INS/UPD/DEL/SEL/SET/OTH/ORA），匹配日志方括号标签 |
| `apps` | [String] | `null` | 要保留的应用名列表 |
| `tags` | [String] | `null` | 要保留的日志标签列表，不带方括号（`statements` 的同义字段） |
| `start_ts` | String | `null` | 记录时间戳的闭区间下界（格式 `YYYY-MM-DD HH:MM:SS`） |
| `end_ts` | String | `null` | 记录时间戳的闭区间上界（格式同上） |
| `trxids` | [String] | `null` | 要保留的事务 ID 列表 |

---

## [filter.exclude]

记录级排除过滤，采用 **OR 否决**：任意一个字段的任意一个值命中，即丢弃该记录。字段集合与 include 相同（时间戳与事务 ID 除外）。

```toml
[filter.exclude]
# users      = ["guest", "anon"]         # 用户名
# ips        = ["10.0.0.1"]              # 客户端 IP
# sessions   = ["0x0000000000000000"]    # 会话 ID
# threads    = ["0"]                     # 线程 ID
# statements = ["ORA"]                   # 语句类型（见下方说明）
# apps       = ["monitor", "health"]     # 应用名
# tags       = ["ORA"]                   # 日志标签（与 statements 同义）
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `users` | [String] | `null` | 要排除的用户名列表 |
| `ips` | [String] | `null` | 要排除的客户端 IP 列表 |
| `sessions` | [String] | `null` | 要排除的会话 ID 列表 |
| `threads` | [String] | `null` | 要排除的线程 ID 列表 |
| `statements` | [String] | `null` | 要排除的语句类型列表，匹配日志方括号标签 |
| `apps` | [String] | `null` | 要排除的应用名列表 |
| `tags` | [String] | `null` | 要排除的日志标签列表，不带方括号（`statements` 的同义字段） |

**关于 `statements` 与 `tags`：** DM SQL 日志中的语句类型以方括号标签形式出现（如 `[SEL]`、`[INS]`、`[ORA]`），解析后存于记录的 `tag` 字段，取值**不含方括号**（`[ORA]` → `"ORA"`）。`statements` 与 `tags` 匹配的是**同一个字段**，互为同义——填 `["ORA"]` 即可，`["[ORA]"]` 匹配不到。日志里的 `stmt:` 句柄指针（如 `0x7fa38c03a480`）不是语句类型，无法也无需按它过滤。

---

## [filter.indicators]

事务级指标过滤（需要两遍预扫描）：命中任一条件即**保留整笔事务**的所有记录。

```toml
[filter.indicators]
# exec_ids = [257809109, 257809110]   # 执行 ID 列表
# min_runtime_ms = 1000               # 最小执行时长（毫秒）
# min_row_count = 100                 # 最小影响行数
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `exec_ids` | [i64] | `null` | 任一记录的 `exec_id` 命中则保留整笔事务 |
| `min_runtime_ms` | u32 | `null` | 任一语句执行时长（毫秒）≥ 此阈值则保留整笔事务 |
| `min_row_count` | u32 | `null` | 任一语句影响行数 ≥ 此阈值则保留整笔事务 |

---

## [filter.sql]

事务级 SQL 内容过滤（需要两遍预扫描）：按 SQL 文本的**字面量子串**匹配（`str::contains`，不支持正则）。

```toml
[filter.sql]
# includes = ["FROM USER_TABLES", "DELETE FROM"]   # 命中任一子串则保留整笔事务
# excludes = ["SELECT 1", "DUAL"]                  # 命中任一子串则丢弃整笔事务
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `includes` | [String] | `null` | 任一 SQL 文本包含所列任一子串则保留整笔事务（旧字段名 `include_patterns` 仍兼容） |
| `excludes` | [String] | `null` | 任一 SQL 文本包含所列任一子串则丢弃整笔事务（旧字段名 `exclude_patterns` 仍兼容） |

**说明：** 记录级组合规则为 `(include 各字段 AND) AND (NOT exclude 任一命中)`。事务级过滤器（`indicators`、`sql`）在预扫描阶段收集命中的事务 ID，正式扫描时保留这些事务的全部记录。启用任一事务级过滤器都会触发预扫描，对大文件有额外一遍 I/O 成本。旧版扁平字段（如 `usernames`、`client_ips`、`exclude_usernames`）仍向后兼容，但建议迁移到上述子表写法。

---

## [exporter.csv]

CSV 导出配置。当 CSV 和 SQLite 同时配置时，CSV 优先级更高。

```toml
[exporter.csv]
# 输出 CSV 文件路径
file = "outputs/sqllog.csv"
# 写入前删除并重建文件
overwrite = true
# 追加到已有文件而非覆盖
append = false
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `file` | String | *（必填）* | 输出 CSV 文件路径（不可为空） |
| `overwrite` | bool | `true` | 写入前删除并重建文件 |
| `append` | bool | `false` | 追加到已有文件而非覆盖 |
| `include_performance_metrics` | bool | `true` | 是否输出 `exec_time_ms`/`row_count`/`exec_id` 三列；为 `false` 时跳过性能指标解析并省略这三列 |

**说明：** CSV 使用 16 MB `BufWriter` + `itoa` 零分配整数格式化，实现约 520 万条记录/秒的吞吐量。`overwrite` 与 `append` 不能同时为 `false`（否则会静默截断已有文件），验证阶段会报错。列的投影与顺序由独立的 `[output]` 段控制（见下）。

---

## [exporter.sqlite]

SQLite 导出配置。当 CSV 和 SQLite 同时配置时，SQLite 优先级较低。

```toml
[exporter.sqlite]
# 输出 SQLite 数据库文件路径
database_url = "export/sqllog2db.db"
# 目标表名（仅限 ASCII 标识符：^[A-Za-z_][A-Za-z0-9_]*$）
table_name = "sqllog_records"
# 写入前删除并重建该表
overwrite = true
# 追加行到已有表而非覆盖
append = false
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `database_url` | String | *（必填）* | 输出 SQLite 数据库文件路径（不可为空） |
| `table_name` | String | `"sqllog_records"` | 目标表名，须匹配 `^[A-Za-z_][A-Za-z0-9_]*$` |
| `overwrite` | bool | `true` | 写入前删除并重建该表 |
| `append` | bool | `false` | 追加行到已有表而非覆盖 |
| `batch_size` | usize | `10000` | 单个事务内的 INSERT 批大小（须 > 0） |
| `multi_row_batch_size` | usize | `64` | 多行 INSERT 每条语句的行数，取值 1-64（15 列 × 64 = 960 < SQLite 变量上限 999） |

**说明：** 使用批量 INSERT 配合 PRAGMA 优化（synchronous=OFF、mmap_size、cache_size），实现约 110 万条记录/秒的吞吐量。列的投影与顺序由独立的 `[output]` 段控制（见下）。

---

## [output]（可选）

字段投影：选择导出哪些列以及列的顺序，对 CSV 和 SQLite 同时生效。省略该段则输出全部 15 个字段的默认顺序。

```toml
[output]
# 仅导出这些字段，并按此处顺序排列
fields = ["ts", "username", "sql", "exec_time_ms"]
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `fields` | [String] | `null`（全部字段） | 要导出的字段名列表，按列表顺序输出；为空或省略则输出全部 15 列 |

**可用字段名：** `ts`、`ep`、`sess_id`、`thrd_id`、`username`、`trx_id`、`statement`、`appname`、`client_ip`、`tag`、`sql`、`exec_time_ms`、`row_count`、`exec_id`、`normalized_sql`。字段名在 `validate` 阶段校验，无效名称会导致校验失败。

---

## 附录：配置行为说明

### 导出器优先级

每次运行只有一个导出器处于活动状态。优先级：CSV > SQLite。当 `[exporter.csv]` 和 `[exporter.sqlite]` 都配置时，CSV 优先。移除或注释 CSV 节即可使用 SQLite。

### 处理管道快速路径

当没有启用任何过滤器时，整个处理管道通过单个 `pipeline.is_empty()` 检查绕过。这意味着可选功能在禁用时不会增加任何运行时开销。

### 配置验证

运行前使用 `sqllog2db validate -c config.toml` 检查配置。验证会一次性报告所有错误（而非遇错即停）。常见问题：缺少必填字段、无效路径、TOML 语法错误。

### 命令行子命令

sqllog2db 提供五个子命令：

**`sqllog2db init`** — 生成默认配置文件。支持 `-o` 指定输出路径、`--force` 强制覆盖。

**`sqllog2db validate`** — 校验配置文件。`-c` 指定配置文件路径，通过时静默退出（exit 0），失败时输出 `[FAIL] <字段>: <原因>` 并以非零码退出。

**`sqllog2db run`** — 执行日志导出。`-c` 指定配置文件路径，`-v` 详细模式，`-q` 静默模式。

**`sqllog2db stats`** — 统计分析。流式扫描日志文件，聚合慢 SQL 和高频 SQL。
- `-c` 指定配置文件路径（复用 `[sqllog]` 输入配置和 `[exporter]` 输出目录）
- `--top N`（默认 20）：每张表输出 Top N 条记录
- CSV 模式：在 `[exporter.csv].file` 同级目录输出 `slow_sql.csv` 和 `frequent_sql.csv`，并在终端打印 Top N 表格
- SQLite 模式：在配置的数据库中写入 `slow_sql` 和 `frequent_sql` 表

**`sqllog2db watch`** — 监听模式。持续监视 `[sqllog].inputs` 配置的目录，出现新的 `.log` 文件时自动触发处理。`-c` 指定配置文件路径，`-q` 静默模式（适合 cron/后台运行），按 Ctrl+C 停止。

示例见[快速入门指南](quickstart.md)。

### 字段顺序

各 TOML 节中的字段可以按任意顺序排列。配置使用 `serde` 反序列化，与顺序无关。可选节可以完全省略——所有字段采用默认值。

### 环境变量

- `SQLLOG2DB_CONFIG` — 设置默认配置文件路径（可被 `-c` 标志覆盖）
- `NO_COLOR` — 禁用彩色终端输出
- `RUST_LOG` — 使用 `env_logger` 时覆盖日志级别
