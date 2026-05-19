use compact_str::CompactString;
use smallvec::SmallVec;
use std::collections::HashMap;

/// 参数替换缓冲区类型：keyed by (`sess_id`, `stmt`)，value 为解析好的参数列表。
///
/// Key 使用 `sess_id` 而非 `trxid`：DM 日志中 PARAMS 记录携带绑定时的 `trxid`，
/// 但对应的 DML 执行记录在自动提交场景下 `trxid` 为 0，导致 key 不匹配。
/// `sess_id` 在 PARAMS 和执行记录之间始终一致，是更稳定的关联键。
///
/// - Key 使用 `CompactString`：`sess_id`（指针地址）和 `stmt` 通常 ≤23 字节，内联存储，无堆分配。
/// - Value 使用 `SmallVec<[ParamValue; 6]>`：≤6 个参数时不分配堆内存。
pub type ParamBuffer = ahash::HashMap<(CompactString, CompactString), SmallVec<[ParamValue; 6]>>;

/// A single parameter value parsed from a `PARAMS(...)` log record.
///
/// `CompactString` stores strings ≤ 24 bytes inline (no heap allocation),
/// which covers virtually all numeric literals and short string params.
#[derive(Debug, Clone)]
pub enum ParamValue {
    /// Single-quoted string already including the surrounding quotes, e.g. `'3USJ29'`.
    Quoted(CompactString),
    /// Bare numeric literal, e.g. `2370075`.
    Bare(CompactString),
    /// NULL, BLOB, or any empty-value entry.
    Null,
}

impl ParamValue {
    fn as_sql(&self) -> &str {
        match self {
            Self::Quoted(s) | Self::Bare(s) => s.as_str(),
            Self::Null => "NULL",
        }
    }
}

/// Parse a `PARAMS(SEQNO, TYPE, DATA)={...}` record body into an ordered list of values.
///
/// Returns `None` if the body does not match the expected format.
///
/// Uses `SmallVec<[ParamValue; 6]>` to avoid heap allocation for typical param lists (≤6 values).
#[must_use]
pub fn parse_params(body: &str) -> Option<SmallVec<[ParamValue; 6]>> {
    // memmem 使用 Two-Way + SIMD 算法，比 str::find 快
    let brace = memchr::memmem::find(body.as_bytes(), b"={")?;
    let inner = body[brace + 2..].strip_suffix('}')?;

    let mut params = SmallVec::new();
    // trim_start：只需去除前导空格，尾部空格在下一次迭代自然消耗
    let mut rest = inner.trim_start();

    while !rest.is_empty() {
        let (value, tail) = parse_one_entry(rest)?;
        params.push(value);
        rest = tail.trim_start();
        if let Some(t) = rest.strip_prefix(',') {
            rest = t.trim_start();
        }
    }

    Some(params)
}

/// Parse one `(seqno, type, value)` entry from the front of `s`.
/// Returns `(parsed_value, remaining_input)`.
fn parse_one_entry(s: &str) -> Option<(ParamValue, &str)> {
    let s = s.strip_prefix('(')?;

    // Skip SEQNO (integer up to first comma) — memchr for SIMD acceleration
    let comma1 = memchr::memchr(b',', s.as_bytes())?;
    let s = s[comma1 + 1..].trim_start();

    // Skip TYPE (up to next comma)
    let comma2 = memchr::memchr(b',', s.as_bytes())?;
    let s = s[comma2 + 1..].trim_start();

    // Parse VALUE then the closing ')'
    if s.starts_with('\'') {
        // Quoted string — use memchr to skip to the next single-quote, same pattern as
        // count_placeholders / apply_params, avoiding the byte-by-byte inner loop.
        let bytes = s.as_bytes();
        let mut i = 1;
        loop {
            let rel = memchr::memchr(b'\'', &bytes[i..])?;
            i += rel + 1;
            // '' is an escaped quote inside the string — consume both and keep scanning
            if i < bytes.len() && bytes[i] == b'\'' {
                i += 1;
            } else {
                break;
            }
        }
        // s[..i] is the quoted string including both surrounding quotes
        let quoted = &s[..i];
        let tail = s[i..].trim_start().strip_prefix(')')?;
        Some((ParamValue::Quoted(CompactString::new(quoted)), tail))
    } else {
        // Bare number or empty — memchr for closing ')'
        let end = memchr::memchr(b')', s.as_bytes())?;
        let raw = s[..end].trim();
        let tail = &s[end + 1..];
        let value = if raw.is_empty() {
            ParamValue::Null
        } else {
            ParamValue::Bare(CompactString::new(raw))
        };
        Some((value, tail))
    }
}

/// Detect which placeholder style the SQL uses and count the number of slots,
/// skipping over single-quoted string literals.
///
/// Returns `(count, is_colon_style)`:
/// - `is_colon_style = false` → `?` style; count = number of `?` outside literals
/// - `is_colon_style = true`  → `:N` Oracle style; count = highest ordinal seen
///
/// If the SQL contains no recognisable placeholders, returns `(0, false)`.
#[inline]
#[must_use]
pub fn count_placeholders(sql: &str) -> (usize, bool) {
    let bytes = sql.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut question_count = 0usize;
    let mut max_colon_ordinal = 0usize;

    while i < len {
        // 用 memchr3 跳过无关字节，直接定位到下一个特殊字符
        let Some(rel) = memchr::memchr3(b'\'', b'?', b':', &bytes[i..]) else {
            break; // 无更多特殊字节
        };
        i += rel;

        match bytes[i] {
            b'\'' => {
                // Skip string literal verbatim — use memchr to jump to next quote
                i += 1;
                loop {
                    let Some(r) = memchr::memchr(b'\'', &bytes[i..]) else {
                        i = len;
                        break;
                    };
                    i += r + 1;
                    if i < len && bytes[i] == b'\'' {
                        i += 1; // '' escape, keep scanning
                    } else {
                        break;
                    }
                }
            }
            b'?' => {
                question_count += 1;
                i += 1;
            }
            b':' => {
                // `:N` where N is one or more decimal digits
                let start = i + 1;
                let mut j = start;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j > start {
                    // `:N` 内的字节均为 ASCII 数字（已 while 保证），直接累加避免 from_utf8 + parse 开销
                    // 使用 saturating 算术防止超长序号（>20 位）在 debug 构建下 panic（WR-03）
                    let n: usize = bytes[start..j].iter().fold(0usize, |acc, &b| {
                        acc.saturating_mul(10).saturating_add((b - b'0') as usize)
                    });
                    max_colon_ordinal = max_colon_ordinal.max(n);
                    i = j;
                } else {
                    i += 1;
                }
            }
            _ => unreachable!(),
        }
    }

    if max_colon_ordinal > 0 {
        (max_colon_ordinal, true)
    } else {
        (question_count, false)
    }
}

/// Replace parameter placeholders in `sql` with values from `params`, writing
/// the result into `out` (which is cleared first).
///
/// Internal hot-path used by both `apply_params` and [`compute_normalized`].
/// Avoids a `String` allocation when the caller already owns a reusable `Vec<u8>`.
///
/// # Safety invariant
/// `out` will contain valid UTF-8 on return: all bytes are either taken verbatim
/// from `sql` (already valid UTF-8) or are ASCII literals from params.
/// ASCII bytes (0x00–0x7F) can never appear in the interior of a multi-byte
/// UTF-8 sequence (continuation bytes are 0x80–0xBF), so no sequence is broken.
#[inline]
fn apply_params_into(sql: &str, params: &[ParamValue], colon_style: bool, out: &mut Vec<u8>) {
    out.clear();
    if params.is_empty() {
        out.extend_from_slice(sql.as_bytes());
        return;
    }

    let extra: usize = params
        .iter()
        .map(|p| p.as_sql().len().saturating_sub(1))
        .sum();
    out.reserve(sql.len() + extra);
    let bytes = sql.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut seq_idx = 0usize; // used for `?` style

    while i < len {
        // 用 memchr2 跳过无关字节：问号模式找 ' 和 ?，冒号模式找 ' 和 :
        let special = if colon_style {
            memchr::memchr2(b'\'', b':', &bytes[i..])
        } else {
            memchr::memchr2(b'\'', b'?', &bytes[i..])
        };
        let Some(rel) = special else {
            out.extend_from_slice(&bytes[i..]);
            break;
        };
        // 批量复制特殊字节之前的普通内容
        if rel > 0 {
            out.extend_from_slice(&bytes[i..i + rel]);
        }
        i += rel;

        match bytes[i] {
            b'\'' => {
                // Copy string literal verbatim — use memchr to bulk-copy chunks between quotes
                out.push(b'\'');
                i += 1;
                loop {
                    let Some(r) = memchr::memchr(b'\'', &bytes[i..]) else {
                        out.extend_from_slice(&bytes[i..]);
                        i = len;
                        break;
                    };
                    out.extend_from_slice(&bytes[i..=(i + r)]); // copy up to and including the '
                    i += r + 1;
                    if i < len && bytes[i] == b'\'' {
                        out.push(b'\''); // '' escape: emit second '
                        i += 1;
                    } else {
                        break;
                    }
                }
            }
            b'?' if !colon_style => {
                if let Some(p) = params.get(seq_idx) {
                    out.extend_from_slice(p.as_sql().as_bytes());
                } else {
                    out.push(b'?');
                }
                seq_idx += 1;
                i += 1;
            }
            b':' if colon_style => {
                let start = i + 1;
                let mut j = start;
                while j < len && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j > start {
                    // `:N` 内的字节均为 ASCII 数字，直接累加避免 from_utf8 + parse 开销
                    // 使用 saturating 算术防止超长序号（>20 位）在 debug 构建下 panic（WR-03）
                    let n: usize = bytes[start..j].iter().fold(0usize, |acc, &b| {
                        acc.saturating_mul(10).saturating_add((b - b'0') as usize)
                    });
                    // :N is 1-indexed
                    if let Some(p) = n.checked_sub(1).and_then(|idx| params.get(idx)) {
                        out.extend_from_slice(p.as_sql().as_bytes());
                    } else {
                        out.extend_from_slice(&bytes[i..j]);
                    }
                    i = j;
                } else {
                    out.push(b':');
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
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
#[cfg(test)]
fn apply_params(sql: &str, params: &[ParamValue], colon_style: bool) -> String {
    let mut buf = Vec::new();
    apply_params_into(sql, params, colon_style, &mut buf);
    String::from_utf8(buf).expect("apply_params produced invalid UTF-8")
}

/// Helper used in `cli/run.rs` to update the params buffer and compute the
/// `normalized_sql` value for a single log record.
///
/// Accepts pre-parsed `meta` and `pm_sql` to avoid re-parsing inside this
/// function. For PARAMS records `pm_sql` equals the record body (the two are
/// identical when there are no performance indicators). For DML records it is
/// the SQL statement extracted from `PerformanceMetrics::sql`.
///
/// - If the record is a `PARAMS(...)` record, its values are stored in `buffer`
///   (keyed by `(sess_id, stmt)`) and `None` is returned.
/// - If the record is an `[INS]`/`[DEL]`/`[UPD]`/`[SEL]` execution record that
///   has a matching entry in `buffer`, the SQL with substituted parameters is
///   returned as `Some(String)`.
/// - For all other records, `None` is returned.
///
/// `placeholder_override`:
/// - `None`        → auto-detect from the SQL (`:N` takes priority over `?`)
/// - `Some(true)`  → force colon-style (`:N`)
/// - `Some(false)` → force question-style (`?`)
///
/// `scratch` is a caller-owned reusable buffer. On a successful substitution the
/// result is written there and a `&str` pointing into it is returned, eliminating
/// a per-record heap allocation. The caller must not modify `scratch` while the
/// returned reference is live.
///
/// # Panics
///
/// Returns `None` only if the result contains bytes that are neither valid UTF-8 nor
/// valid GB18030 (extremely rare). For GB18030 files, the result is automatically
/// transcoded to UTF-8.
pub fn compute_normalized<'a, S: std::hash::BuildHasher>(
    record: &dm_database_parser_sqllog::Sqllog<'_>,
    meta: &dm_database_parser_sqllog::MetaParts<'_>,
    pm_sql: &str,
    buffer: &mut HashMap<(CompactString, CompactString), SmallVec<[ParamValue; 6]>, S>,
    placeholder_override: Option<bool>,
    scratch: &'a mut Vec<u8>,
) -> Option<&'a str> {
    if record.tag.is_none() {
        // 无 tag → 可能是 PARAMS 记录。
        // pm_sql 对于 PARAMS 记录等价于 body()（无性能指标时两者相同），
        // 直接复用，节省一次 find_indicators_split() 后向扫描。
        if pm_sql.starts_with("PARAMS(") {
            if let Some(params) = parse_params(pm_sql) {
                // CompactString 对短字符串（≤23 字节）内联存储，消除堆分配。
                // sess_id（指针如 "0xfffb81a474a0"）和 statement（如 "0x1"）通常都满足此条件。
                buffer.insert(
                    (
                        CompactString::from(meta.sess_id.as_ref()),
                        CompactString::from(meta.statement.as_ref()),
                    ),
                    params,
                );
            }
        }
        return None;
    }

    // 有 tag → DML/SEL 执行记录
    let tag = record.tag.as_deref()?;
    if !matches!(tag, "INS" | "DEL" | "UPD" | "SEL") {
        return None;
    }

    // 先扫描 SQL 是否含占位符，大多数 SQL 没有占位符，可以提前返回，
    // 避免两次 CompactString 分配（trxid + statement key）。
    let (placeholder_count, detected_colon) = count_placeholders(pm_sql);
    if placeholder_count == 0 {
        return None;
    }

    let key = (
        CompactString::from(meta.sess_id.as_ref()),
        CompactString::from(meta.statement.as_ref()),
    );

    // buffer 条目保留不删除：DM 有时对同一次执行记录两条 SEL 日志（相同 EXEC_ID，
    // 不同 ROWCOUNT），它们共享同一个 PARAMS。下一次绑定时新的 buffer.insert 会覆盖旧值。
    let params = buffer.get(&key)?.clone();

    let colon_style = placeholder_override.unwrap_or(detected_colon);

    if params.len() != placeholder_count {
        log::warn!(
            "replace_parameters: param count mismatch (params={}, placeholders={}) for sql: {}",
            params.len(),
            placeholder_count,
            pm_sql
                .char_indices()
                .nth(80)
                .map_or(pm_sql, |(i, _)| &pm_sql[..i])
        );
        return None;
    }

    apply_params_into(pm_sql, &params, colon_style, scratch);

    // 常规路径：UTF-8 文件，直接返回。
    // GB18030 fallback：上游 parser 将 GB18030 文件按 UTF-8 解析时，param 替换后
    // 的字节序列可能含 GB18030 双字节序列（如汉字），导致 UTF-8 校验失败。
    // GB18030 是 ASCII 的超集，可安全处理纯 ASCII 与混合内容。
    if std::str::from_utf8(scratch).is_err() {
        let (decoded, _, had_errors) = encoding_rs::GB18030.decode(scratch);
        if had_errors {
            log::warn!(
                "replace_parameters: GB18030 fallback had unmappable bytes for sql: {}",
                &pm_sql[..pm_sql.len().min(60)]
            );
        }
        // into_owned() 释放对 scratch 的借用，之后才能 clear + 写回
        let decoded_string = decoded.into_owned();
        scratch.clear();
        scratch.extend_from_slice(decoded_string.as_bytes());
    }

    Some(std::str::from_utf8(scratch).expect("scratch contains valid UTF-8"))
}

/// SQL 模板归一化所需的特殊字节查找表
const NEEDS_SPECIAL_NORM: [bool; 256] = {
    let mut t = [false; 256];
    t[b'\'' as usize] = true;
    t[b' ' as usize] = true;
    t[b'\t' as usize] = true;
    t[b'\n' as usize] = true;
    t[b'\r' as usize] = true;
    t[0x0B_usize] = true;
    t[0x0C_usize] = true;
    let mut d = b'0';
    while d <= b'9' {
        t[d as usize] = true;
        d += 1;
    }
    t[b'-' as usize] = true;
    t[b'/' as usize] = true;
    t
};

/// 将 SQL 字符串归一化为模板 key：去除注释、折叠 IN 列表、统一关键字大小写、折叠空白。
///
/// 结构相同的 SQL（无论字面量值或数量）将得到同一模板 key，用于模板聚合统计。
/// 原位于 `fingerprint.rs`，为保留模板管道功能迁移至此。
#[must_use]
pub fn normalize_template(sql: &str) -> String {
    scan_sql_bytes(sql)
}

fn scan_sql_bytes(sql: &str) -> String {
    let bytes = sql.as_bytes();
    let len = bytes.len();
    let mut out: Vec<u8> = Vec::with_capacity(sql.len());
    let mut i = 0;
    while i < len {
        let bulk_start = i;
        while i < len && !NEEDS_SPECIAL_NORM[bytes[i] as usize] && !bytes[i].is_ascii_alphabetic() {
            i += 1;
        }
        if i > bulk_start {
            out.extend_from_slice(&bytes[bulk_start..i]);
        }
        if i >= len {
            break;
        }
        i = dispatch_byte(bytes, i, len, &mut out);
    }
    let out_str = String::from_utf8(out).expect("scan_sql_bytes: invalid UTF-8");
    let trimmed = out_str.trim_ascii();
    if trimmed.len() == out_str.len() {
        out_str
    } else {
        trimmed.to_string()
    }
}

fn dispatch_byte(bytes: &[u8], i: usize, len: usize, out: &mut Vec<u8>) -> usize {
    match bytes[i] {
        b'\'' => handle_quote(bytes, i, out),
        b'-' if i + 1 < len && bytes[i + 1] == b'-' => handle_line_comment(bytes, i, out),
        b'/' if i + 1 < len && bytes[i + 1] == b'*' => handle_block_comment(bytes, i, out),
        b if b.is_ascii_whitespace() => {
            if !matches!(out.last(), Some(&b' ')) {
                out.push(b' ');
            }
            let mut j = i + 1;
            while j < len && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            j
        }
        b if b.is_ascii_alphabetic() => handle_word(bytes, i, len, out),
        b => {
            out.push(b);
            i + 1
        }
    }
}

/// 处理单引号字符串字面量。始终保留原文（normalize 路径）。
fn handle_quote(bytes: &[u8], i: usize, out: &mut Vec<u8>) -> usize {
    let literal_start = i;
    let mut j = i + 1;
    let len = bytes.len();
    loop {
        let Some(rel) = memchr::memchr(b'\'', &bytes[j..]) else {
            j = len;
            break;
        };
        j += rel + 1;
        if j < len && bytes[j] == b'\'' {
            j += 1; // '' 转义，继续消费
        } else {
            break;
        }
    }
    out.extend_from_slice(&bytes[literal_start..j]);
    j
}

/// 跳过单行注释（`--` 到行尾），i 指向第一个 `-`。
/// 在注释前插入一个空格，防止注释后的内容与前面的 token 粘连（如 `SELECT 1--comment\nFROM t` → `SELECT 1 FROM t`）。
fn handle_line_comment(bytes: &[u8], i: usize, out: &mut Vec<u8>) -> usize {
    if !matches!(out.last(), Some(&b' ')) {
        out.push(b' ');
    }
    match memchr::memchr(b'\n', &bytes[i..]) {
        Some(rel) => i + rel + 1,
        None => bytes.len(),
    }
}

/// 跳过块注释（`/* ... */`），i 指向 `/`，替换为单空格避免 token 粘连。
fn handle_block_comment(bytes: &[u8], i: usize, out: &mut Vec<u8>) -> usize {
    let len = bytes.len();
    let mut j = i + 2;
    match memchr::memmem::find(&bytes[j..], b"*/") {
        Some(rel) => j += rel + 2,
        None => j = len,
    }
    if !matches!(out.last(), Some(&b' ')) {
        out.push(b' ');
    }
    j
}

/// 处理单词（normalize 路径）：关键字大写化，IN 列表尝试折叠。
fn handle_word(bytes: &[u8], i: usize, len: usize, out: &mut Vec<u8>) -> usize {
    let start = i;
    let mut j = i;
    while j < len && is_ident_byte(bytes[j]) {
        j += 1;
    }
    let word = &bytes[start..j];
    if prev_is_ident_byte(out) {
        // 处于标识符中部（如 t.column 中的 column），直接复制
        out.extend_from_slice(word);
        return j;
    }
    if is_keyword(word) {
        for &b in word {
            out.push(b.to_ascii_uppercase());
        }
        if word.len() == 2 && word.eq_ignore_ascii_case(b"IN") {
            if let Some(new_j) = try_fold_in_list(bytes, j, len, out) {
                return new_j;
            }
        }
    } else {
        out.extend_from_slice(word);
    }
    j
}

/// 尝试将 IN (...) 折叠为 IN (?)；含子查询则放弃并返回 None。
fn try_fold_in_list(bytes: &[u8], mut i: usize, len: usize, out: &mut Vec<u8>) -> Option<usize> {
    while i < len && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= len || bytes[i] != b'(' {
        return None;
    }
    let mut j = i + 1;
    let mut depth = 1usize;
    while j < len && depth > 0 {
        match bytes[j] {
            b'(' => {
                depth += 1;
                j += 1;
            }
            b')' => {
                depth -= 1;
                j += 1;
            }
            b'\'' => j = skip_quoted(bytes, j + 1),
            _ => j += 1,
        }
    }
    if depth != 0 {
        return None;
    }
    let inner = &bytes[i + 1..j - 1];
    if is_subquery(inner) {
        return None;
    }
    out.extend_from_slice(b" (?)");
    Some(j)
}

/// 从 j（引号后第一字节）跳过单引号字符串，返回闭合引号后的位置。
fn skip_quoted(bytes: &[u8], mut j: usize) -> usize {
    let len = bytes.len();
    loop {
        let Some(rel) = memchr::memchr(b'\'', &bytes[j..]) else {
            return len;
        };
        j += rel + 1;
        if j < len && bytes[j] == b'\'' {
            j += 1;
        } else {
            return j;
        }
    }
}

/// 检测 IN 列表内容中是否含子查询（包含独立的 SELECT 或 FROM 关键字）。
fn is_subquery(inner: &[u8]) -> bool {
    let len = inner.len();
    let mut i = 0;
    while i < len {
        if inner[i].is_ascii_alphabetic() {
            let start = i;
            while i < len && is_ident_byte(inner[i]) {
                i += 1;
            }
            let word = &inner[start..i];
            if word.eq_ignore_ascii_case(b"SELECT") || word.eq_ignore_ascii_case(b"FROM") {
                return true;
            }
        } else if inner[i] == b'\'' {
            i = skip_quoted(inner, i + 1);
        } else {
            i += 1;
        }
    }
    false
}

/// 判断 word 是否为 SQL 关键字（大小写不敏感）。
fn is_keyword(word: &[u8]) -> bool {
    if word.len() > 8 {
        return false;
    }
    let mut buf = [0u8; 8];
    for (idx, &b) in word.iter().enumerate() {
        buf[idx] = b.to_ascii_uppercase();
    }
    let s = &buf[..word.len()];
    matches!(
        s,
        b"SELECT"
            | b"FROM"
            | b"WHERE"
            | b"AND"
            | b"OR"
            | b"JOIN"
            | b"ON"
            | b"AS"
            | b"INSERT"
            | b"UPDATE"
            | b"DELETE"
            | b"INTO"
            | b"VALUES"
            | b"SET"
            | b"GROUP"
            | b"ORDER"
            | b"BY"
            | b"HAVING"
            | b"UNION"
            | b"DISTINCT"
            | b"LIMIT"
            | b"CREATE"
            | b"DROP"
            | b"ALTER"
            | b"IN"
            | b"NOT"
            | b"NULL"
            | b"IS"
            | b"BETWEEN"
            | b"LIKE"
            | b"EXISTS"
            | b"CASE"
            | b"WHEN"
            | b"THEN"
            | b"ELSE"
            | b"END"
    )
}

/// 单字节是否为标识符字节（字母/数字/下划线/点）。
fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'.'
}

/// 上一个输出字节是否是标识符字节（字母/数字/下划线/点）。
fn prev_is_ident_byte(out: &[u8]) -> bool {
    out.last().is_some_and(|&b| is_ident_byte(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bare(s: &str) -> ParamValue {
        ParamValue::Bare(CompactString::new(s))
    }
    fn quoted(s: &str) -> ParamValue {
        ParamValue::Quoted(CompactString::new(s))
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

    // ── normalize_template 测试（从 fingerprint.rs 迁移） ───────────────────

    #[test]
    fn test_normalize_line_comment_removed() {
        assert_eq!(normalize_template("-- comment\nSELECT 1"), "SELECT 1");
    }

    #[test]
    fn test_normalize_block_comment_replaced() {
        assert_eq!(normalize_template("/* multi */ SELECT 1"), "SELECT 1");
    }

    #[test]
    fn test_normalize_in_list_numbers_same_key() {
        let a = normalize_template("SELECT * FROM t WHERE id IN (1, 2, 3)");
        let b = normalize_template("SELECT * FROM t WHERE id IN (10, 20, 30, 40)");
        assert_eq!(a, b);
    }

    #[test]
    fn test_normalize_in_list_strings_same_key() {
        let a = normalize_template("SELECT * FROM t WHERE name IN ('a', 'b')");
        let b = normalize_template("SELECT * FROM t WHERE name IN ('xx', 'yy', 'zz')");
        assert_eq!(a, b);
    }

    #[test]
    fn test_normalize_keyword_uppercase() {
        let result = normalize_template("select * from t where id = 1");
        assert!(result.contains("SELECT"), "expected SELECT in {result}");
        assert!(result.contains("FROM"), "expected FROM in {result}");
        assert!(result.contains("WHERE"), "expected WHERE in {result}");
    }

    #[test]
    fn test_normalize_ident_with_underscore_preserved() {
        let result = normalize_template("SELECT a FROM outer_join_t");
        assert!(
            result.contains("outer_join_t"),
            "expected outer_join_t in {result}"
        );
    }

    #[test]
    fn test_normalize_string_literal_hides_comment_marker() {
        let result = normalize_template("WHERE col = '-- not a comment'");
        assert!(
            result.contains("'-- not a comment'"),
            "expected literal preserved in {result}"
        );
    }

    #[test]
    fn test_normalize_whitespace_collapsed() {
        assert_eq!(normalize_template("SELECT  *  FROM  t"), "SELECT * FROM t");
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_normalize_template_is_idempotent(s in any::<String>()) {
            let once = normalize_template(&s);
            let twice = normalize_template(&once);
            prop_assert_eq!(&once, &twice, "normalize_template should be idempotent but got different results");
        }

        #[test]
        fn prop_normalize_template_literal_protection(inner in "[A-Za-z0-9 ]{0,50}") {
            let sql = format!("WHERE col = '{inner}-- not a comment'");
            let result = normalize_template(&sql);
            prop_assert!(
                result.contains("-- not a comment"),
                "literal comment marker should survive in: {}",
                result
            );
        }
    }
}
