use super::super::{f32_ms_to_i64, strip_ip_prefix};
use dm_database_parser_sqllog::Sqllog;
use rusqlite::types::Value;

/// 将 `Sqllog` 记录转换为 `Vec<Value>`，按 `ordered_indices` 投影字段。
/// 用于 `row_buffer` 缓冲路径（全量与投影统一路径）。
pub(super) fn sqllog_to_values(
    sqllog: &Sqllog,
    normalized_sql: Option<&str>,
    ordered_indices: &[usize],
) -> Vec<Value> {
    let (exec_time_ms, row_count, exec_id) =
        if sqllog.exec_id != 0 || sqllog.exectime > 0.0 || sqllog.rowcount != 0 {
            (
                Some(f32_ms_to_i64(sqllog.exectime)),
                Some(sqllog.rowcount),
                Some(sqllog.exec_id),
            )
        } else {
            (None, None, None)
        };

    let all: [Value; 15] = [
        Value::Text(sqllog.ts.clone()),
        Value::Integer(i64::from(sqllog.ep)),
        Value::Text(sqllog.sess_id.clone()),
        Value::Text(sqllog.thrd_id.clone()),
        Value::Text(sqllog.username.clone()),
        Value::Text(sqllog.trxid.clone()),
        Value::Text(sqllog.statement.clone()),
        Value::Text(sqllog.appname.clone()),
        Value::Text(strip_ip_prefix(&sqllog.client_ip).to_string()),
        sqllog
            .tag
            .as_ref()
            .map_or(Value::Null, |t| Value::Text(t.clone())),
        Value::Text(sqllog.sql.clone()),
        exec_time_ms.map_or(Value::Null, Value::Integer),
        row_count.map_or(Value::Null, |v| Value::Integer(i64::from(v))),
        exec_id.map_or(Value::Null, Value::Integer),
        normalized_sql.map_or(Value::Null, |s| Value::Text(s.to_string())),
    ];
    ordered_indices.iter().map(|&i| all[i].clone()).collect()
}
