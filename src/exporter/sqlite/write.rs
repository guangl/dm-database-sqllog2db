use super::super::{f32_ms_to_i64, strip_ip_prefix};
use crate::pipeline::FieldMask;
use dm_database_parser_sqllog::Sqllog;

/// 热路径：使用 `Sqllog` 直接字段插入（parser 库已物化所有字段）。
pub(super) fn do_insert_preparsed(
    stmt: &mut rusqlite::CachedStatement<'_>,
    sqllog: &Sqllog,
    normalized_sql: Option<&str>,
    field_mask: FieldMask,
    ordered_indices: &[usize],
) -> std::result::Result<(), rusqlite::Error> {
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

    if field_mask == FieldMask::ALL {
        stmt.execute(rusqlite::params![
            sqllog.ts.as_str(),
            sqllog.ep,
            sqllog.sess_id.as_str(),
            sqllog.thrd_id.as_str(),
            sqllog.username.as_str(),
            sqllog.trxid.as_str(),
            sqllog.statement.as_str(),
            sqllog.appname.as_str(),
            strip_ip_prefix(&sqllog.client_ip),
            sqllog.tag.as_deref(),
            sqllog.sql.as_str(),
            exec_time_ms,
            row_count,
            exec_id,
            normalized_sql
        ])?;
        return Ok(());
    }

    use rusqlite::types::Value;
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
    let selected: Vec<&Value> = ordered_indices.iter().map(|&i| &all[i]).collect();
    stmt.execute(rusqlite::params_from_iter(selected))?;
    Ok(())
}
