use super::*;

#[test]
fn test_basic_where_number_and_string() {
    assert_eq!(
        normalize_sql("SELECT * FROM t WHERE id = 42 AND name = 'alice'"),
        "SELECT * FROM t WHERE id = ? AND name = ?"
    );
}

#[test]
fn test_multiple_numeric_literals() {
    assert_eq!(
        normalize_sql("INSERT INTO t VALUES (1, 2, 3)"),
        "INSERT INTO t VALUES (?, ?, ?)"
    );
}

#[test]
fn test_escaped_quote_in_string() {
    assert_eq!(normalize_sql("WHERE name = 'O''Brien'"), "WHERE name = ?");
}

#[test]
fn test_no_literals_unchanged() {
    let sql_with_placeholder = "SELECT col FROM t WHERE id = ?";
    assert_eq!(normalize_sql(sql_with_placeholder), sql_with_placeholder);

    let sql_plain = "SELECT col FROM t";
    assert_eq!(normalize_sql(sql_plain), sql_plain);
}

#[test]
fn test_insert_multiple_columns_with_float() {
    assert_eq!(
        normalize_sql("INSERT INTO orders (id, name, amount) VALUES (100, 'test', 3.14)"),
        "INSERT INTO orders (id, name, amount) VALUES (?, ?, ?)"
    );
}

#[test]
fn test_identifier_with_digits_not_replaced() {
    assert_eq!(
        normalize_sql("SELECT col1, table2 FROM t WHERE id = 1"),
        "SELECT col1, table2 FROM t WHERE id = ?"
    );
}

#[test]
fn test_unclosed_string_does_not_panic() {
    // 未闭合字符串：进入字符串状态后到达末尾，应产生单个 `?` 且不 panic
    let result = normalize_sql("SELECT 'unclosed");
    assert_eq!(result, "SELECT ?");
}
