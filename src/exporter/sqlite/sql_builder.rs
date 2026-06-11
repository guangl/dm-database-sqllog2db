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

/// 生成多行批量 INSERT SQL：
/// - 全量投影：`INSERT INTO "table" VALUES (?, ...), ...`
/// - 部分投影：`INSERT INTO "table" (col1, col2) VALUES (?, ...), ...`
///
/// 与 `build_insert_sql` 保持一致——部分投影时显式列出列名，
/// 避免追加模式下列顺序不同导致的静默错误。
pub(super) fn build_multi_row_insert_sql(
    table_name: &str,
    ordered_indices: &[usize],
    row_count: usize,
) -> String {
    let col_count = ordered_indices.len();
    debug_assert!(row_count > 0);
    debug_assert!(col_count > 0);
    let one_row = format!("({})", vec!["?"; col_count].join(", "));
    let value_rows = std::iter::repeat_n(one_row.as_str(), row_count)
        .collect::<Vec<_>>()
        .join(", ");
    if ordered_indices.len() == FIELD_NAMES.len() {
        format!("INSERT INTO \"{table_name}\" VALUES {value_rows}")
    } else {
        let cols = projected_field_names(ordered_indices);
        format!(
            "INSERT INTO \"{table_name}\" ({}) VALUES {value_rows}",
            cols.join(", ")
        )
    }
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

    #[test]
    fn test_build_multi_row_insert_sql_basic() {
        // 部分投影（3列），应包含列名
        let sql = build_multi_row_insert_sql("t", &[10, 4, 0], 2);
        assert_eq!(
            sql,
            "INSERT INTO \"t\" (sql, username, ts) VALUES (?, ?, ?), (?, ?, ?)"
        );
    }

    #[test]
    fn test_build_multi_row_insert_sql_single_row() {
        let all_indices: Vec<usize> = (0..15).collect();
        let single_row_sql = build_multi_row_insert_sql("t", &all_indices, 1);
        let full_path_sql = build_insert_sql("t", &all_indices);
        assert_eq!(single_row_sql, full_path_sql);
    }

    #[test]
    fn test_build_multi_row_insert_sql_64_rows() {
        let all_indices: Vec<usize> = (0..15).collect();
        let sql = build_multi_row_insert_sql("t", &all_indices, 64);
        // 验证以 INSERT INTO "t" VALUES 开头
        assert!(sql.starts_with("INSERT INTO \"t\" VALUES "));
        // 验证包含 64 个 "(" 开头的占位符组
        let paren_count = sql.matches('(').count();
        assert_eq!(
            paren_count, 64,
            "应有 64 个 '(' 即 64 行，实际: {paren_count}"
        );
        // 验证参数总数：64 × 15 = 960 个 '?'
        let placeholder_count = sql.matches('?').count();
        assert_eq!(
            placeholder_count, 960,
            "应有 960 个占位符，实际: {placeholder_count}"
        );
    }
}
