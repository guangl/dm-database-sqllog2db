//! init 子命令使用的默认配置模板（TOML 文本资产）。
//!
//! 由 `cli::init` 的向导按导出器类型选择并做占位符替换后写出。

pub(crate) const CONFIG_TEMPLATE_CSV: &str = r#"# sqllog2db 默认配置文件（请按需修改）

[sqllog]
# SQL 日志路径列表：可为目录、单个文件或 glob 模式（例如 "./logs/2025-*.log"）
# 支持配置多个条目。
inputs = ["sqllogs"]

[logging]
# 应用日志文件路径
file = "logs/sqllog2db.log"
# 日志级别：trace | debug | info | warn | error
level = "info"
# 日志保留天数（1-365）
retention_days = 7

[replace_parameters]
# 是否在导出结果中写入 normalized_sql 列（默认：true）。
# 对 INS/DEL/UPD/ORA 记录，会将参数值回填到 SQL 占位符中。
enable = true

[filter]
# 是否启用过滤管道
enable = false

# --- 包含过滤器（记录级，AND 语义：每个已配置字段都必须匹配） ---
# 元数据字段使用精确字符串匹配。
[filter.include]
# users      = ["SYSDBA"]                       # 精确匹配：要保留的用户名列表
# ips        = ["127.0.0.1", "192.168.1.100"]   # 精确匹配：要保留的客户端 IP 列表
# sessions   = ["0x7f41435437a8"]               # 精确匹配：要保留的会话 ID 列表（十六进制字符串）
# threads    = ["2188515"]                      # 精确匹配：要保留的线程 ID 列表
# statements = ["INS", "UPD", "DEL"]            # 语句类型（INS/UPD/DEL/SEL/SET/OTH/ORA），匹配日志方括号标签
# apps       = ["DMSQL"]                        # 精确匹配：要保留的应用名列表
# tags       = ["SEL", "INS"]                   # 日志标签，不带方括号（与 statements 同义；日志中的 [SEL] 取值 "SEL"）
# start_ts   = "2023-01-01 00:00:00"            # 记录时间戳的闭区间下界（格式：YYYY-MM-DD HH:MM:SS）
# end_ts     = "2023-01-01 23:59:59"            # 记录时间戳的闭区间上界（格式：YYYY-MM-DD HH:MM:SS）
# trxids     = ["257809109", "257809110"]       # 精确匹配：要保留的事务 ID 列表

# --- 排除过滤器（记录级，OR 否决：任意一项匹配即丢弃该记录） ---
# 元数据字段使用精确字符串匹配。
[filter.exclude]
# users      = ["guest", "anon"]                # 精确匹配：要排除的用户名列表
# ips        = ["10.0.0.1", "172.16.0.1"]       # 精确匹配：要排除的客户端 IP 列表
# sessions   = ["0x0000000000000000"]           # 精确匹配：要排除的会话 ID 列表（十六进制字符串）
# threads    = ["0"]                            # 精确匹配：要排除的线程 ID 列表
# statements = ["SEL", "SET"]                   # 语句类型（INS/UPD/DEL/SEL/SET/OTH/ORA），匹配日志方括号标签
# apps       = ["monitor", "health"]            # 精确匹配：要排除的应用名列表
# tags       = ["SET", "OTH"]                   # 日志标签，不带方括号（与 statements 同义；日志中的 [SET] 取值 "SET"）

# --- 指标过滤器（事务级：命中即保留整笔事务，需要预扫描） ---
[filter.indicators]
# exec_ids = [257809109, 257809110]   # 事务级：任一记录的 exec_id 命中则保留整笔事务
# min_runtime_ms = 1000               # 事务级：任一语句执行时长（毫秒）≥ 阈值则保留整笔事务
# min_row_count = 100                 # 事务级：任一语句影响行数 ≥ 阈值则保留整笔事务

# --- SQL 内容过滤器（事务级：命中即保留整笔事务，需要预扫描） ---
[filter.sql]
# includes = ["FROM USER_TABLES", "DELETE FROM"]   # 事务级：任一 SQL 文本包含所列任一子串则保留整笔事务
# excludes = ["SELECT 1", "DUAL"]                  # 事务级：任一 SQL 文本包含所列任一子串则丢弃整笔事务

# --- stats 子命令的时间范围过滤（可选） ---
[stats]
# from = "2024-01-01"   # 时间范围起点。格式："YYYY-MM-DD" 或 "YYYY-MM-DD HH:MM:SS"
# to   = "2024-01-31"   # 时间范围终点。格式同 from。
# top  = 20             # 默认 top-N 数量。命令行 --top 会覆盖此值。
# 命令行参数 --from / --to / --top 会覆盖以上配置。命令行与配置均未提供时，stats 不做时间过滤（top 默认为 20）。

# ===================== 导出器配置 =====================
# 同一时刻只能启用一个导出器。优先级：csv > sqlite

# 方案 1：CSV 导出（默认）
[exporter.csv]
# CSV 输出文件路径
file = "outputs/sqllog.csv"
# 写入前删除并重建文件（true/false）
overwrite = true
# 追加到已有 CSV 文件而非覆盖（true/false）
append = false

# 方案 2：SQLite 数据库导出
# [exporter.sqlite]
# SQLite 数据库文件路径
# database_url = "export/sqllog2db.db"
# 写入记录的表名（仅限 ASCII 标识符：[A-Za-z_][A-Za-z0-9_]*）
# table_name = "sqllog_records"
# 写入前删除并重建该表（true/false）
# overwrite = true
# 追加行到已有表而非覆盖（true/false）
# append = false
"#;

pub(crate) const CONFIG_TEMPLATE_SQLITE: &str = r#"# sqllog2db 默认配置文件（请按需修改）

[sqllog]
# SQL 日志路径列表：可为目录、单个文件或 glob 模式（例如 "./logs/2025-*.log"）
# 支持配置多个条目。
inputs = ["sqllogs"]

[logging]
# 应用日志文件路径
file = "logs/sqllog2db.log"
# 日志级别：trace | debug | info | warn | error
level = "info"
# 日志保留天数（1-365）
retention_days = 7

[replace_parameters]
# 是否在导出结果中写入 normalized_sql 列（默认：true）。
# 对 INS/DEL/UPD/ORA 记录，会将参数值回填到 SQL 占位符中。
enable = true

[filter]
# 是否启用过滤管道
enable = false

# --- 包含过滤器（记录级，AND 语义：每个已配置字段都必须匹配） ---
# 元数据字段使用精确字符串匹配。
[filter.include]
# users      = ["SYSDBA"]                       # 精确匹配：要保留的用户名列表
# ips        = ["127.0.0.1", "192.168.1.100"]   # 精确匹配：要保留的客户端 IP 列表
# sessions   = ["0x7f41435437a8"]               # 精确匹配：要保留的会话 ID 列表（十六进制字符串）
# threads    = ["2188515"]                      # 精确匹配：要保留的线程 ID 列表
# statements = ["INS", "UPD", "DEL"]            # 语句类型（INS/UPD/DEL/SEL/SET/OTH/ORA），匹配日志方括号标签
# apps       = ["DMSQL"]                        # 精确匹配：要保留的应用名列表
# tags       = ["SEL", "INS"]                   # 日志标签，不带方括号（与 statements 同义；日志中的 [SEL] 取值 "SEL"）
# start_ts   = "2023-01-01 00:00:00"            # 记录时间戳的闭区间下界（格式：YYYY-MM-DD HH:MM:SS）
# end_ts     = "2023-01-01 23:59:59"            # 记录时间戳的闭区间上界（格式：YYYY-MM-DD HH:MM:SS）
# trxids     = ["257809109", "257809110"]       # 精确匹配：要保留的事务 ID 列表

# --- 排除过滤器（记录级，OR 否决：任意一项匹配即丢弃该记录） ---
# 元数据字段使用精确字符串匹配。
[filter.exclude]
# users      = ["guest", "anon"]                # 精确匹配：要排除的用户名列表
# ips        = ["10.0.0.1", "172.16.0.1"]       # 精确匹配：要排除的客户端 IP 列表
# sessions   = ["0x0000000000000000"]           # 精确匹配：要排除的会话 ID 列表（十六进制字符串）
# threads    = ["0"]                            # 精确匹配：要排除的线程 ID 列表
# statements = ["SEL", "SET"]                   # 语句类型（INS/UPD/DEL/SEL/SET/OTH/ORA），匹配日志方括号标签
# apps       = ["monitor", "health"]            # 精确匹配：要排除的应用名列表
# tags       = ["SET", "OTH"]                   # 日志标签，不带方括号（与 statements 同义；日志中的 [SET] 取值 "SET"）

# --- 指标过滤器（事务级：命中即保留整笔事务，需要预扫描） ---
[filter.indicators]
# exec_ids = [257809109, 257809110]   # 事务级：任一记录的 exec_id 命中则保留整笔事务
# min_runtime_ms = 1000               # 事务级：任一语句执行时长（毫秒）≥ 阈值则保留整笔事务
# min_row_count = 100                 # 事务级：任一语句影响行数 ≥ 阈值则保留整笔事务

# --- SQL 内容过滤器（事务级：命中即保留整笔事务，需要预扫描） ---
[filter.sql]
# includes = ["FROM USER_TABLES", "DELETE FROM"]   # 事务级：任一 SQL 文本包含所列任一子串则保留整笔事务
# excludes = ["SELECT 1", "DUAL"]                  # 事务级：任一 SQL 文本包含所列任一子串则丢弃整笔事务

# --- stats 子命令的时间范围过滤（可选） ---
[stats]
# from = "2024-01-01"   # 时间范围起点。格式："YYYY-MM-DD" 或 "YYYY-MM-DD HH:MM:SS"
# to   = "2024-01-31"   # 时间范围终点。格式同 from。
# top  = 20             # 默认 top-N 数量。命令行 --top 会覆盖此值。
# 命令行参数 --from / --to / --top 会覆盖以上配置。命令行与配置均未提供时，stats 不做时间过滤（top 默认为 20）。

# ===================== 导出器配置 =====================
# 同一时刻只能启用一个导出器。优先级：csv > sqlite

# 方案 1：CSV 导出（默认）
# [exporter.csv]
# CSV 输出文件路径
# file = "outputs/sqllog.csv"
# 写入前删除并重建文件（true/false）
# overwrite = true
# 追加到已有 CSV 文件而非覆盖（true/false）
# append = false
# Max rows per CSV file before splitting into sqllog_1.csv, sqllog_2.csv, ...
# (unset or 0 = single file, split mode requires overwrite = true)
# max_rows_per_file = 1000000

# 方案 2：SQLite 数据库导出
[exporter.sqlite]
# SQLite 数据库文件路径
database_url = "export/sqllog2db.db"
# 写入记录的表名（仅限 ASCII 标识符：[A-Za-z_][A-Za-z0-9_]*）
table_name = "sqllog_records"
# 写入前删除并重建该表（true/false）
overwrite = true
# 追加行到已有表而非覆盖（true/false）
append = false
"#;
