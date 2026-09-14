use super::*;

fn bare(s: &str) -> ParamValue {
    ParamValue::Bare(String::from(s))
}
fn quoted(s: &str) -> ParamValue {
    ParamValue::Quoted(String::from(s))
}

// ── parse_params ──────────────────────────────────────────────────────────

#[test]
fn test_parse_single_varchar() {
    let params = parse_params("PARAMS(SEQNO, TYPE, DATA)={(0, VARCHAR, 'SM')}").unwrap();
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].as_sql(), "'SM'");
}

#[test]
fn test_parse_mixed_types() {
    let params = parse_params(
        "PARAMS(SEQNO, TYPE, DATA)={(0, DEC, 3), (1, VARCHAR, 'send ok'), (2, DEC, 0), (3, INTEGER, 42)}",
    )
    .unwrap();
    assert_eq!(params.len(), 4);
    assert_eq!(params[0].as_sql(), "3");
    assert_eq!(params[1].as_sql(), "'send ok'");
    assert_eq!(params[2].as_sql(), "0");
    assert_eq!(params[3].as_sql(), "42");
}

#[test]
fn test_parse_blob_empty() {
    let params = parse_params("PARAMS(SEQNO, TYPE, DATA)={(0, DEC, 1), (1, BLOB, )}").unwrap();
    assert_eq!(params.len(), 2);
    assert_eq!(params[0].as_sql(), "1");
    assert_eq!(params[1].as_sql(), "NULL");
}

#[test]
fn test_parse_quoted_with_escaped_quote() {
    let params = parse_params("PARAMS(SEQNO, TYPE, DATA)={(0, VARCHAR, 'O''Brien')}").unwrap();
    assert_eq!(params[0].as_sql(), "'O''Brien'");
}

#[test]
fn test_parse_invalid_returns_none() {
    assert!(parse_params("not a params record").is_none());
}

// ── apply_params ──────────────────────────────────────────────────────────

#[test]
fn test_apply_single_string_param() {
    let params = vec![quoted("'3USJ29'")];
    let result = apply_params("WHERE code = ?", &params, false);
    assert_eq!(result, "WHERE code = '3USJ29'");
}

#[test]
fn test_apply_numeric_param() {
    let params = vec![bare("42")];
    let result = apply_params("WHERE id = ?", &params, false);
    assert_eq!(result, "WHERE id = 42");
}

#[test]
fn test_apply_null_param() {
    let params = vec![ParamValue::Null];
    let result = apply_params("WHERE tag = ?", &params, false);
    assert_eq!(result, "WHERE tag = NULL");
}

#[test]
fn test_apply_multiple_params() {
    let params = vec![bare("2370075"), quoted("'SJ-1'"), ParamValue::Null];
    let result = apply_params("VALUES (?, ?, ?)", &params, false);
    assert_eq!(result, "VALUES (2370075, 'SJ-1', NULL)");
}

#[test]
fn test_apply_no_placeholders() {
    let params = vec![bare("1")];
    let result = apply_params("SELECT 1", &params, false);
    assert_eq!(result, "SELECT 1");
}

#[test]
fn test_apply_skip_literal_contents() {
    // The '?' inside the string literal should NOT be replaced
    let params = vec![quoted("'real'")];
    let result = apply_params("WHERE a = '?' AND b = ?", &params, false);
    assert_eq!(result, "WHERE a = '?' AND b = 'real'");
}

#[test]
fn test_apply_insert_with_function() {
    // current_timestamp is not a placeholder; only the bare ? are replaced
    let params = vec![bare("1"), quoted("'hello'"), bare("99")];
    let result = apply_params(
        "INSERT INTO t VALUES (?,current_timestamp,?,?)",
        &params,
        false,
    );
    assert_eq!(
        result,
        "INSERT INTO t VALUES (1,current_timestamp,'hello',99)"
    );
}

#[test]
fn test_apply_chinese_in_param() {
    let params = vec![quoted("'张三'")];
    let result = apply_params("WHERE name = ?", &params, false);
    assert_eq!(result, "WHERE name = '张三'");
}

// ── colon-style placeholders ───────────────────────────────────────────────

#[test]
fn test_apply_colon_style_basic() {
    let params = vec![bare("10"), quoted("'abc'")];
    let result = apply_params("WHERE id = :1 AND code = :2", &params, true);
    assert_eq!(result, "WHERE id = 10 AND code = 'abc'");
}

#[test]
fn test_apply_colon_style_out_of_order() {
    let params = vec![bare("1"), bare("2"), bare("3")];
    let result = apply_params("SELECT :3, :1, :2", &params, true);
    assert_eq!(result, "SELECT 3, 1, 2");
}

#[test]
fn test_count_placeholders_question() {
    let (count, colon_style) = count_placeholders("WHERE a = ? AND b = ?");
    assert_eq!(count, 2);
    assert!(!colon_style);
}

#[test]
fn test_count_placeholders_colon() {
    let (count, colon_style) = count_placeholders("WHERE a = :1 AND b = :2 AND c = :3");
    assert_eq!(count, 3);
    assert!(colon_style);
}

#[test]
fn test_count_placeholders_skips_literals() {
    let (count, colon_style) = count_placeholders("WHERE a = '?' AND b = ?");
    assert_eq!(count, 1);
    assert!(!colon_style);
}

#[test]
fn test_count_placeholders_none() {
    let (count, colon_style) = count_placeholders("SELECT 1");
    assert_eq!(count, 0);
    assert!(!colon_style);
}

#[test]
fn test_count_placeholders_unclosed_string() {
    // Unclosed string literal — covers the `None` branch in the inner loop
    let (count, _) = count_placeholders("SELECT 'unclosed");
    assert_eq!(count, 0);
}

#[test]
fn test_count_placeholders_escaped_quote() {
    // SQL with '' (escaped quote inside string) — covers the '' escape branch
    let (count, _) = count_placeholders("WHERE name = 'O''Brien' AND id = ?");
    assert_eq!(count, 1);
}

#[test]
fn test_count_placeholders_colon_not_followed_by_digit() {
    // ':' not followed by digits → i += 1 branch (line 168)
    let (count, colon_style) = count_placeholders("SELECT a::text");
    assert_eq!(count, 0);
    assert!(!colon_style);
}

#[test]
fn test_apply_params_empty_params_returns_sql_unchanged() {
    // Empty params list → early return with sql copy (lines 197-198)
    let result = apply_params("SELECT * FROM t", &[], false);
    assert_eq!(result, "SELECT * FROM t");
}

#[test]
fn test_apply_params_with_string_literal_verbatim_copy() {
    // String literal in SQL is copied verbatim, ? inside is NOT replaced
    let params = vec![bare("42")];
    let result = apply_params("WHERE code = '?' AND id = ?", &params, false);
    assert_eq!(result, "WHERE code = '?' AND id = 42");
}

#[test]
fn test_apply_params_escaped_quote_in_literal() {
    // '' escape inside a string literal — covers lines 242-243
    let params = vec![bare("1")];
    let result = apply_params("WHERE name = 'O''Brien' AND id = ?", &params, false);
    assert_eq!(result, "WHERE name = 'O''Brien' AND id = 1");
}

#[test]
fn test_apply_params_unclosed_string_literal() {
    // Unclosed string literal in SQL — covers lines 235-237 in apply_params_into
    let params = vec![bare("1")];
    let result = apply_params("SELECT 'unclosed", &params, false);
    // Unclosed string: no ? found outside literal, result == original sql
    assert_eq!(result, "SELECT 'unclosed");
}

#[test]
fn test_compute_normalized_nested_lookup_missing_statement() {
    // sess_id 存在但 statement 不存在时应返回 None（不 panic）
    let mut buffer: ParamBuffer = ParamBuffer::new();
    // 先插入 sess_id="s1", statement="stmt_a"
    buffer
        .entry("s1".to_string())
        .or_default()
        .insert("stmt_a".to_string(), Arc::new(vec![]));

    // 查询 sess_id="s1", statement="stmt_b"（不存在）
    let inner = buffer.get("s1");
    assert!(inner.is_some(), "outer key 应存在");
    let result = inner.unwrap().get("stmt_b");
    assert!(result.is_none(), "inner key 不存在时应返回 None");
}

/// Replace parameter placeholders in `sql` with values from `params`.
///
/// Supports two placeholder styles:
/// - `?`  — replaced sequentially: first `?` → `params[0]`, second → `params[1]`, …
/// - `:N` — replaced by ordinal:   `:1` → `params[0]`, `:2` → `params[1]`, …
///
/// String params are already single-quoted (e.g. `'hello'`); numeric and NULL params
/// are written bare or as `NULL`. Placeholders inside single-quoted SQL string literals
/// are never replaced.
///
/// **Callers must verify that `params.len()` equals `count_placeholders(sql).0`
/// before calling this function.**  If counts differ the result is unspecified.
///
/// # Panics
///
/// Will not panic in practice: the output is valid UTF-8 (original SQL bytes plus
/// ASCII param literals). The `expect` is an internal consistency assertion.
fn apply_params(sql: &str, params: &[ParamValue], colon_style: bool) -> String {
    let mut buf = Vec::new();
    apply_params_into(sql, params, colon_style, &mut buf);
    String::from_utf8(buf).expect("apply_params produced invalid UTF-8")
}
