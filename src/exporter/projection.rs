use crate::pipeline::FIELD_NAMES;

/// 根据有序字段索引列表返回对应的字段名列表。
///
/// 用于 `SQLite` SQL 构建路径（`build_insert_sql` / `build_create_sql`）的字段名拼接，
/// 不应用于 CSV 写入热路径（`write_record_preparsed`）以避免每条记录产生 Vec 分配。
pub(crate) fn projected_field_names(ordered_indices: &[usize]) -> Vec<&'static str> {
    ordered_indices.iter().map(|&i| FIELD_NAMES[i]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_projected_field_names_full() {
        let indices: Vec<usize> = (0..FIELD_NAMES.len()).collect();
        let names = projected_field_names(&indices);
        assert_eq!(names.len(), FIELD_NAMES.len());
        assert_eq!(names, FIELD_NAMES.to_vec());
    }

    #[test]
    fn test_projected_field_names_subset() {
        let indices = vec![10, 4];
        let names = projected_field_names(&indices);
        assert_eq!(names, vec!["sql", "username"]);
    }

    #[test]
    fn test_projected_field_names_order_preserved() {
        let indices = vec![4, 10, 0];
        let names = projected_field_names(&indices);
        assert_eq!(names, vec!["username", "sql", "ts"]);
    }
}
