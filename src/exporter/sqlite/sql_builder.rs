use super::super::projection::projected_field_names;
use crate::pipeline::FIELD_NAMES;

const COL_TYPES: &[&str] = &[
    "TEXT NOT NULL",    // ts        0
    "INTEGER NOT NULL", // ep        1
    "TEXT NOT NULL",    // sess_id   2
    "TEXT NOT NULL",    // thrd_id   3
    "TEXT NOT NULL",    // username  4
    "TEXT NOT NULL",    // trx_id    5
    "TEXT",             // statement 6
    "TEXT",             // appname   7
    "TEXT",             // client_ip 8
    "TEXT",             // tag       9
    "TEXT NOT NULL",    // sql       10
    "INTEGER",          // exec_time_ms 11
    "INTEGER",          // row_count 12
    "INTEGER",          // exec_id   13
    "TEXT",             // normalized_sql 14
];

/// 根据有序字段索引列表生成 INSERT SQL。
/// 全量快速路径：与 `new()` 的默认 `insert_sql` 一致（不调用 projection）。
/// 投影路径：使用 `projected_field_names` 构建列名列表。
pub(super) fn build_insert_sql(table_name: &str, ordered_indices: &[usize]) -> String {
    if ordered_indices.len() == FIELD_NAMES.len() {
        // 全量快速路径：与 new() 的默认 insert_sql 一致
        return format!(
            "INSERT INTO \"{table_name}\" VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        );
    }
    let cols = projected_field_names(ordered_indices);
    let placeholders = vec!["?"; ordered_indices.len()].join(", ");
    format!(
        "INSERT INTO \"{table_name}\" ({}) VALUES ({placeholders})",
        cols.join(", ")
    )
}

/// 根据有序字段索引列表生成 CREATE TABLE SQL。
/// 字段名使用 `projected_field_names` 构建，类型从 `COL_TYPES` 映射。
pub(super) fn build_create_sql(table_name: &str, ordered_indices: &[usize]) -> String {
    let cols: Vec<String> = ordered_indices
        .iter()
        .map(|&i| format!("{} {}", FIELD_NAMES[i], COL_TYPES[i]))
        .collect();
    format!(
        "CREATE TABLE IF NOT EXISTS \"{table_name}\" ({})",
        cols.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_insert_sql_ordered() {
        let sql = build_insert_sql("t", &[10, 4]);
        assert_eq!(sql, "INSERT INTO \"t\" (sql, username) VALUES (?, ?)");
    }

    #[test]
    fn test_build_create_sql_ordered() {
        let sql = build_create_sql("t", &[10, 4]);
        assert_eq!(
            sql,
            "CREATE TABLE IF NOT EXISTS \"t\" (sql TEXT NOT NULL, username TEXT NOT NULL)"
        );
    }

    #[test]
    fn test_build_insert_sql_full_fast_path() {
        let all_indices: Vec<usize> = (0..15).collect();
        let sql = build_insert_sql("t", &all_indices);
        assert_eq!(
            sql,
            "INSERT INTO \"t\" VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        );
    }
}
