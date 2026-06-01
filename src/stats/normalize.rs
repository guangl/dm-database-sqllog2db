//! SQL 字面量标准化：将 SQL 文本中的字符串字面量（单引号包裹，含 `''` 转义）
//! 和数字字面量（整数、浮点）替换为 `?` 占位符。
//!
//! 标识符中的数字（如 `col1`、`table2`）保持原样，不被替换。
//! 用于把参数不同但模板相同的 SQL 调用归并为同一字符串键（Phase 51/52 统计聚合）。

/// 将 SQL 文本中的字符串字面量（单引号包裹，含 `''` 转义）和数字字面量
/// （整数、浮点）替换为 `?` 占位符。标识符中的数字（如 `col1`）保持原样。
///
/// 用于把参数不同但模板相同的 SQL 调用归并为同一字符串键。
///
/// # Panics
///
/// 不会在实践中 panic：输出字节要么来自 UTF-8 输入的原样复制，要么是 ASCII
/// 字节 `b'?'`（单字节 ASCII 不会破坏多字节 UTF-8 序列）。`expect` 是内部
/// 一致性断言，正常情况下不会触发。
#[must_use]
pub fn normalize_sql(sql: &str) -> String {
    let bytes = sql.as_bytes();
    let len = bytes.len();
    let mut output = Vec::with_capacity(len);
    let mut cursor = 0usize;
    let mut prev_was_ident_char = false;

    while cursor < len {
        let byte = bytes[cursor];
        match byte {
            b'\'' => {
                cursor = skip_string_literal(bytes, cursor + 1, len);
                output.push(b'?');
                prev_was_ident_char = false;
            }
            byte_val if byte_val.is_ascii_digit() && !prev_was_ident_char => {
                cursor = skip_number_literal(bytes, cursor, len);
                output.push(b'?');
                prev_was_ident_char = false;
            }
            b'-' | b'+'
                if !prev_was_ident_char
                    && cursor + 1 < len
                    && bytes[cursor + 1].is_ascii_digit() =>
            {
                // 负号或正号紧跟数字时，整体视为一个带符号的数字字面量，用单个 `?` 替换
                cursor = skip_number_literal(bytes, cursor + 1, len);
                output.push(b'?');
                prev_was_ident_char = false;
            }
            _ => {
                output.push(byte);
                prev_was_ident_char = byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$';
                cursor += 1;
            }
        }
    }

    String::from_utf8(output).expect("normalize_sql produced invalid UTF-8")
}

/// 跳过单引号字符串字面量，处理 `''` 转义引号。
///
/// `start` 是第一个开始引号之后的位置（即字符串内容起始处）。
/// 返回字符串结束后的下一个游标位置（即结束引号 `'` 之后）。
/// 若字符串未闭合，则返回 `len`（字节末尾）。
fn skip_string_literal(bytes: &[u8], start: usize, len: usize) -> usize {
    let mut cursor = start;
    loop {
        let Some(relative_pos) = memchr::memchr(b'\'', &bytes[cursor..]) else {
            // 未闭合字符串——跳到末尾
            return len;
        };
        cursor += relative_pos + 1;
        if cursor < len && bytes[cursor] == b'\'' {
            // `''` 转义引号——继续扫描字符串内容
            cursor += 1;
        } else {
            // 字符串正常结束
            return cursor;
        }
    }
}

/// 跳过数字字面量（整数或浮点数）。
///
/// `start` 是数字字面量的第一个数字位置。
/// 返回数字字面量结束后的下一个游标位置。
/// 支持浮点格式：整数部分后跟 `.` 再跟至少一个数字（如 `3.14`）。
fn skip_number_literal(bytes: &[u8], start: usize, len: usize) -> usize {
    let mut cursor = start;
    // 跳过整数部分
    while cursor < len && bytes[cursor].is_ascii_digit() {
        cursor += 1;
    }
    // 处理浮点小数部分：`.` 后必须跟数字才算浮点
    if cursor + 1 < len && bytes[cursor] == b'.' && bytes[cursor + 1].is_ascii_digit() {
        cursor += 1; // 跳过 `.`
        while cursor < len && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
    }
    cursor
}

#[cfg(test)]
mod tests {
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
}
