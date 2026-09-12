//! init 子命令使用的默认配置模板（TOML 文本资产）。
//!
//! 由 `cli::init` 的向导按导出器类型选择并做占位符替换后写出。

pub(crate) const CONFIG_TEMPLATE_CSV: &str = r#"# sqllog2db 默认配置文件（请按需修改）

[sqllog]
# SQL 日志路径列表：可为目录、单个文件或 glob 模式（例如 "./logs/2025-*.log"）
# 支持配置多个条目。
inputs = ["sqllogs"]

# [logging]
# 应用日志文件路径
# file = "logs/sqllog2db.log"
# 日志级别：trace | debug | info | warn | error
# level = "info"
# 日志保留天数（1-365）
# retention_days = 7

# 参数替换：取消下一行注释即启用；不配置时不生成 normalized_sql。
# [replace_parameters]
# placeholders = []  # 自动检测，也可指定 ["?"] 或 [":1"]

# 过滤条件按需填写；无条件时不过滤，不需要 enable 开关。
# 元数据逐条匹配；SQL / 指标命中整笔事务。完整字段见 docs/config-reference.md。
# [filter.include]
# users = ["SYSDBA"]
# tags = ["INS", "UPD", "DEL"]
# min_runtime_ms = 1000       # 保留含慢 SQL 的事务
# sql = ["FROM USER_TABLES"] # 保留包含指定 SQL 的事务

# [filter.exclude]
# users = ["guest"]
# sql = ["SELECT 1"]         # 排除包含指定 SQL 的整笔事务

# --- stats 子命令的时间范围过滤（可选） ---
# [stats]
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

# [logging]
# 应用日志文件路径
# file = "logs/sqllog2db.log"
# 日志级别：trace | debug | info | warn | error
# level = "info"
# 日志保留天数（1-365）
# retention_days = 7

# 参数替换：取消下一行注释即启用；不配置时不生成 normalized_sql。
# [replace_parameters]
# placeholders = []  # 自动检测，也可指定 ["?"] 或 [":1"]

# 过滤条件按需填写；无条件时不过滤，不需要 enable 开关。
# 元数据逐条匹配；SQL / 指标命中整笔事务。完整字段见 docs/config-reference.md。
# [filter.include]
# users = ["SYSDBA"]
# tags = ["INS", "UPD", "DEL"]
# min_runtime_ms = 1000       # 保留含慢 SQL 的事务
# sql = ["FROM USER_TABLES"] # 保留包含指定 SQL 的事务

# [filter.exclude]
# users = ["guest"]
# sql = ["SELECT 1"]         # 排除包含指定 SQL 的整笔事务

# --- stats 子命令的时间范围过滤（可选） ---
# [stats]
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
