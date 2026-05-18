use super::super::{f32_ms_to_i64, strip_ip_prefix};
use crate::pipeline::FieldMask;
use dm_database_parser_sqllog::{MetaParts, PerformanceMetrics, Sqllog};

/// 热路径：使用预解析的 `MetaParts` 和 `PerformanceMetrics` 直接插入。
/// 全量掩码走 `params![]` 快速路径；投影掩码走动态 Value 路径。
///
/// 调用方通过 `prepare_cached()` 获取 `stmt`，利用 `StatementCache`（LRU，容量 16）
/// 复用已编译的 statement，开销为 `RefCell::borrow_mut()` + `HashMap` lookup (O(1))，
/// 而非 `sqlite3_prepare_v3()`（O(parse)）。PERF-06 满足。
pub(super) fn do_insert_preparsed(
    stmt: &mut rusqlite::CachedStatement<'_>,
    sqllog: &Sqllog<'_>,
    meta: &MetaParts<'_>,
    pm: &PerformanceMetrics<'_>,
    normalized_sql: Option<&str>,
    field_mask: FieldMask,
    ordered_indices: &[usize],
) -> std::result::Result<(), rusqlite::Error> {
    let (exec_time_ms, row_count, exec_id) =
        if pm.exec_id != 0 || pm.exectime > 0.0 || pm.rowcount != 0 {
            // 与 CSV 路径保持一致：截断为整数毫秒（f32_ms_to_i64）
            (
                Some(f32_ms_to_i64(pm.exectime)),
                Some(pm.rowcount),
                Some(pm.exec_id),
            )
        } else {
            (None, None, None)
        };

    if field_mask == FieldMask::ALL {
        // 全量掩码快速路径：直接绑定全部 15 个参数
        stmt.execute(rusqlite::params![
            sqllog.ts.as_ref(),
            meta.ep,
            meta.sess_id.as_ref(),
            meta.thrd_id.as_ref(),
            meta.username.as_ref(),
            meta.trxid.as_ref(),
            meta.statement.as_ref(),
            meta.appname.as_ref(),
            strip_ip_prefix(meta.client_ip.as_ref()),
            sqllog.tag.as_deref(),
            pm.sql.as_ref(),
            exec_time_ms,
            row_count,
            exec_id,
            normalized_sql
        ])?;
        return Ok(());
    }

    // 投影路径：按有序索引从全量 Value 数组中选取（使用引用避免 move）
    use rusqlite::types::Value;
    let all: [Value; 15] = [
        Value::Text(sqllog.ts.as_ref().to_string()),
        Value::Integer(i64::from(meta.ep)),
        Value::Text(meta.sess_id.as_ref().to_string()),
        Value::Text(meta.thrd_id.as_ref().to_string()),
        Value::Text(meta.username.as_ref().to_string()),
        Value::Text(meta.trxid.as_ref().to_string()),
        Value::Text(meta.statement.as_ref().to_string()),
        Value::Text(meta.appname.as_ref().to_string()),
        Value::Text(strip_ip_prefix(meta.client_ip.as_ref()).to_string()),
        sqllog
            .tag
            .as_deref()
            .map_or(Value::Null, |t| Value::Text(t.to_string())),
        Value::Text(pm.sql.as_ref().to_string()),
        exec_time_ms.map_or(Value::Null, Value::Integer),
        row_count.map_or(Value::Null, |v| Value::Integer(i64::from(v))),
        exec_id.map_or(Value::Null, Value::Integer),
        normalized_sql.map_or(Value::Null, |s| Value::Text(s.to_string())),
    ];
    let selected: Vec<&Value> = ordered_indices.iter().map(|&i| &all[i]).collect();
    stmt.execute(rusqlite::params_from_iter(selected))?;
    Ok(())
}
