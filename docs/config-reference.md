# 配置参考

本文档描述 sqllog2db 中所有可用的配置选项。配置文件使用 TOML 格式编写。默认配置由 `sqllog2db init -o config.toml --force` 生成。以下每节记录一个配置块，包含字段、默认值和使用说明。

所有配置段默认均不启用，写入配置才生效。配置段内部仍可有参数默认值，例如显式写 `[logging]` 后，未填的 level 使用 info。

- 省略 `[replace_parameters]`：不替换参数，不输出 `normalized_sql` 列。
- 省略 `[logging]` / `[error]`：不创建对应日志文件，运行日志默认输出到 stdout。
- 省略所有 `[exporter.*]`：不默认选择导出器；`run` / `validate` 会提示需要配置导出器，`stats` 无需导出器。
- 省略 `[sqllog]`：不默认读取 sqllogs 目录；通过配置 inputs 或命令行 `--input` 提供输入。
- 省略 filter / output / stats：不附加过滤、字段投影或统计选项。统计命令自身的 top-N 等默认参数不变。

`init` 生成的模板会显式配置输入和一个导出器，可选功能以注释形式展示。启用功能只需添加对应配置段，关闭则删除或注释该段；不再使用 enable 开关。

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

控制应用日志（程序自身运行日志）的输出路径、级别和滚动保留。省略该段或未填写 file 时，运行日志输出到 stdout，不写日志文件；只有显式填写 file 才写文件。stdout 日志默认为 info，--verbose 使用 debug，--quiet 仅输出 error。

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
| `file` | String | 未配置（输出到 stdout） | 显式指定后写入该文件，不可为空 |
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

## [replace_parameters]（可选）

该段存在即启用参数替换，将日志中的 PARAMS 回填到 SQL 占位符，并在导出中增加 `normalized_sql` 列。省略该段时关闭替换，也不会输出该列。原始 `sql` 列保持不变。

```toml
[replace_parameters]
# 默认仅回填 SEL；可增加 "INS"、"UPD"、"DEL" 等日志标签
# tags = ["SEL"]
# 可省略，默认自动检测；也可指定 ["?"] 或 [":1"]
# placeholders = []
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `tags` | [String] | `["SEL"]` | 仅对这些日志标签对应的 SQL 回填参数 |
| `placeholders` | [String] | `[]` | 空列表自动检测，仅 `?` 使用顺序占位符，仅 `:N` 使用序号占位符 |

**迁移：** 旧 `enable = true` 改为仅保留配置段；旧 `enable = false` 改为删除或注释整个配置段。enable 和未知字段会直接报错。若 output.fields 不包含 normalized_sql，则不会计算或导出替换结果。

---

## [filter.include] / [filter.exclude]

过滤只需两组：`include` 保留，`exclude` 排除。**写了条件就生效**；不写过滤器、空子表或空列表表示不过滤。已移除 enable 开关；关闭过滤时删除或注释对应条件。

```toml
[filter.include]
users = ["SYSDBA"]
min_runtime_ms = 1000
# sql = ["UPDATE", "DELETE FROM"]

[filter.exclude]
tags = ["ORA"]
sql = ["SELECT 1", "FROM DUAL"]
```

上例保留含执行时间 ≥ 1000 ms 语句的事务，再仅输出其中 SYSDBA 的非 ORA 记录；如果事务中任一 SQL 包含 `SELECT 1` 或 `FROM DUAL`，整笔事务都会被排除。

### 记录条件

元数据为精确字符串匹配，不支持正则。同一字段内多个值为 OR；include 的不同记录字段为 AND；exclude 的任意记录条件命中就丢弃该条记录。

| 字段 | 类型 | 可用组 | 含义 |
|------|------|--------|------|
| `users` | [String] | 两组 | 用户名 |
| `ips` | [String] | 两组 | 客户端 IP |
| `sessions` | [String] | 两组 | 会话 ID（十六进制字符串） |
| `threads` | [String] | 两组 | 线程 ID |
| `apps` | [String] | 两组 | 应用名 |
| `tags` | [String] | 两组 | 日志标签，如 `SEL`、`INS`、`UPD`、`DEL`、`SET`、`OTH`、`ORA`，不带方括号 |
| `start_ts` | String | include | 时间闭区间下界，`YYYY-MM-DD HH:MM:SS` |
| `end_ts` | String | include | 时间闭区间上界，格式同上 |
| `trxids` | [String] | include | 事务 ID 列表 |

语句类型只使用 `tags`，旧字段 `statements` 已移除。日志里的 `stmt:` 句柄不是语句类型。

### 事务条件

以下字段直接写在 include 或 exclude 中。**同一组的事务条件按 OR 匹配**，任一记录命中即可选中或排除整笔事务；exclude 优先，即使该事务也命中 include。跨文件的同一事务也适用。

| 字段 | 类型 | 含义 |
|------|------|------|
| `sql` | [String] | SQL 文本包含任一字面量子串，区分大小写，不支持正则 |
| `exec_ids` | [i64] | 执行 ID 命中任一值 |
| `min_runtime_ms` | number | 执行时长（毫秒）≥ 阈值，支持小数，必须非负且有限 |
| `min_row_count` | u32 | 影响行数 ≥ 阈值（0 匹配所有记录） |

事务条件需要额外一遍预扫描。没有 include 事务条件时，从全部事务中排除；有 include 事务条件时，从命中事务中排除。显式 `include.trxids` 与预扫描选中 ID 取并集，但仍受事务排除约束。选中的事务还要经过上述记录条件，因此配置记录条件后，导出结果可能只包含事务的部分记录。

stdin 无法预扫描，会输出警告并将 SQL / 指标条件降级为逐条匹配，不能保证整笔事务的保留或排除。

### 旧配置迁移

这是不兼容变更：旧字段和未知字段会在加载时直接报错，请先按下表迁移。

| 旧写法 | 新写法 |
|--------|--------|
| `[filter] enable = true` | 删除此开关，条件自动生效 |
| `[filter] enable = false` | 删除过滤条件，或将其注释掉 |
| `[filter.indicators]` 下的指标 | 移到 `[filter.include]`，字段名不变 |
| `[filter.sql] includes` / `include_patterns` | `[filter.include] sql` |
| `[filter.sql] excludes` / `exclude_patterns` | `[filter.exclude] sql` |
| `statements` | `tags` |
| `[filter] usernames`、`client_ips` 等旧扁平字段 | `[filter.include] users`、`ips` 等 |
| `[filter] exclude_usernames` 等 | `[filter.exclude] users` 等 |

新旧格式不能混用。`filter` 只接受 `include` 和 `exclude`，两组内的拼写错误和已移除字段也会报错。

**行为修正：** SQL 排除现在真正排除整笔事务。旧实现可能因为同事务的另一条 SQL 未命中排除条件而将它重新保留；迁移前后此类数据的导出结果会不同。所有已配置条件都会自动生效。

---

## [exporter.parquet]

Parquet 是 init 模板显式选择的导出格式，适合大数据量归档和分析；省略 exporter 时不会自动启用。导出器按 row group 流式写入，并在缓冲数据约达 64 MiB 时提前刷新，避免长 SQL 导致内存无界增长。

```toml
[exporter.parquet]
file = "outputs/sqllog.parquet"
overwrite = true
compression = "zstd"
row_group_rows = 65536
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `file` | String | `"outputs/sqllog.parquet"` | 输出文件路径 |
| `overwrite` | bool | `true` | 是否覆盖已有文件；为 `false` 时目标必须不存在 |
| `compression` | String | `"zstd"` | `zstd`、`snappy` 或 `uncompressed` |
| `row_group_rows` | usize | `65536` | 每个 row group 的最大记录数，必须大于 0 |

---

## [exporter.csv]

CSV 导出配置。

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

**说明：** CSV 使用 1 MiB `BufWriter` + `itoa` 零分配整数格式化。`overwrite` 与 `append` 不能同时为 `false`（否则会静默截断已有文件），验证阶段会报错。列的投影与顺序由独立的 `[output]` 段控制（见下）。

---

## [output]（可选）

字段投影：选择导出哪些列以及列的顺序，对 Parquet 和 CSV 同时生效。省略该段则输出全部 15 个字段的默认顺序。

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

每次运行只有一个导出器处于活动状态。优先级：Parquet > CSV。建议一次只保留一个启用的导出器配置节。

### 处理管道快速路径

当没有启用任何过滤器时，整个处理管道通过单个 `pipeline.is_empty()` 检查绕过。这意味着可选功能在禁用时不会增加任何运行时开销。

### 配置验证

运行前使用 `sqllog2db validate -c config.toml` 检查配置。验证会一次性报告所有错误（而非遇错即停）。常见问题：缺少必填字段、无效路径、TOML 语法错误。

### 命令行子命令

sqllog2db 提供四个子命令：

**`sqllog2db init`** — 生成默认配置文件。支持 `-o` 指定输出路径、`--force` 强制覆盖。

**`sqllog2db validate`** — 校验配置文件。`-c` 指定配置文件路径，通过时静默退出（exit 0），失败时输出 `[FAIL] <字段>: <原因>` 并以非零码退出。

**`sqllog2db run`** — 执行日志导出。`-c` 指定配置文件路径，`-v` 详细模式，`-q` 静默模式。

**`sqllog2db stats`** — 统计分析。流式扫描日志文件，聚合慢 SQL 和高频 SQL，并直接在终端打印结果。
- `-c` 指定配置文件路径（使用 `[sqllog]` 输入配置和 `[stats]` 统计配置）
- `--top N`（默认 20）：每类结果显示 Top N 条记录
- 不生成 Parquet/CSV 文件；`[exporter]` 配置对该命令不生效


示例见[快速入门指南](quickstart.md)。

### 字段顺序

各 TOML 节中的字段可以按任意顺序排列。配置使用 `serde` 反序列化，与顺序无关。可选节可以完全省略——所有字段采用默认值。

### 环境变量

- `SQLLOG2DB_CONFIG` — 设置默认配置文件路径（可被 `-c` 标志覆盖）
- `NO_COLOR` — 禁用彩色终端输出
- `RUST_LOG` — 使用 `env_logger` 时覆盖日志级别
