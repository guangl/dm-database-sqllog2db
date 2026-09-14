//! Parameter parsing, SQL substitution and per-session record correlation.
use std::collections::HashMap;
use std::sync::Arc;

/// 参数替换缓冲区：外层 key = `sess_id`，内层 key = `statement`。
///
/// Key 使用 `sess_id` 而非 `trxid`：DM 日志中 PARAMS 记录携带绑定时的 `trxid`，
/// 但对应的 DML 执行记录在自动提交场景下 `trxid` 为 0，导致 key 不匹配。
/// `sess_id` 在 PARAMS 和执行记录之间始终一致，是更稳定的关联键。
///
/// 二级结构使查询路径可直接传入 `&str`（`HashMap<String, _>` 实现了 `Borrow<str>`），
/// 避免每条 DML 执行记录构造元组 key 时的两次 `String::clone`（MEM-01 热路径优化）。
/// insert 路径（低频 PARAMS 记录）仍需 clone key，可接受。
///
/// Value 使用 `Arc<Vec<ParamValue>>`：热路径 `.clone()` 仅复制
/// 引用计数（O(1) 原子操作），而非深拷贝整个 Vec（H-3 优化）。
pub type ParamBuffer = HashMap<String, HashMap<String, Arc<Vec<ParamValue>>>>;

/// Helper used by the execution engine to update the params buffer and compute the
/// `normalized_sql` value for a single log record.
///
/// Accepts pre-parsed `meta` and `pm_sql` to avoid re-parsing inside this
/// function. For PARAMS records `pm_sql` equals the record body (the two are
/// identical when there are no performance indicators). For DML records it is
/// the SQL statement extracted from `PerformanceMetrics::sql`.
///
/// - If the record is a `PARAMS(...)` record, its values are stored in `buffer`
///   (keyed by `(sess_id, stmt)`) and `None` is returned.
/// - If the record is an execution record whose tag is included in `tags` and it
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
/// # Returns
///
/// - `Some(&str)` — the SQL with all placeholders replaced by their bound values,
///   written into `scratch`. The reference borrows `scratch`; the caller must not
///   modify `scratch` while it is live.
/// - `None` — if any of the following hold:
///   - the record has no `tag` (e.g. a `PARAMS` record — its values are stored in `buffer`)
///   - the tag is not included in `tags`
///   - the SQL contains no recognisable placeholders
///   - no matching params entry exists in `buffer` for this (`sess_id`, `stmt`) key
///   - the number of bound params does not equal the number of placeholders in the SQL
///
/// # Panics
///
/// Will not panic in practice: all bytes written to `scratch` are either taken verbatim
/// from the UTF-8 input SQL or from UTF-8 `ParamValue` strings. The `expect` below
/// is an internal consistency assertion that should never fire.
pub fn compute_normalized<'a>(
    record: &dm_database_parser_sqllog::Sqllog,
    pm_sql: &str,
    buffer: &mut ParamBuffer,
    tags: &[String],
    placeholder_override: Option<bool>,
    scratch: &'a mut Vec<u8>,
) -> Option<&'a str> {
    if record.tag.is_none() {
        // 无 tag → 可能是 PARAMS 记录。
        if pm_sql.starts_with("PARAMS(")
            && let Some(params) = parse_params(pm_sql)
        {
            buffer
                .entry(record.sess_id.clone())
                .or_default()
                .insert(record.statement.clone(), Arc::new(params));
        }
        return None;
    }

    // 有 tag → SQL 执行记录；仅处理配置指定的标签。
    let tag = record.tag.as_deref()?;
    if !tags.iter().any(|configured| configured == tag) {
        return None;
    }

    let (placeholder_count, detected_colon) = count_placeholders(pm_sql);
    if placeholder_count == 0 {
        return None;
    }

    let params = buffer
        .get(record.sess_id.as_str())?
        .get(record.statement.as_str())?
        .clone();

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

    // All bytes in `scratch` come from two UTF-8 sources:
    //   1. verbatim slices of `pm_sql` (already valid UTF-8)
    //   2. ParamValue::Quoted/Bare strings (Rust String — always valid UTF-8)
    // ASCII literals used as delimiters ('?', ':', '\'') are single-byte and
    // cannot appear in the interior of a multi-byte UTF-8 sequence, so no
    // sequence is broken. The debug_assert guards this invariant cheaply in
    // debug builds; the expect is a final consistency guard.
    debug_assert!(
        std::str::from_utf8(scratch).is_ok(),
        "apply_params_into produced invalid UTF-8 — safety invariant violated"
    );
    Some(std::str::from_utf8(scratch).expect("apply_params_into produced invalid UTF-8"))
}

#[cfg(test)]
#[path = "../../tests/unit/pipeline/normalizer.rs"]
mod tests;

use serde::Deserialize;

/// `[replace_parameters]` 配置段
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct NormalizeConfig {
    /// 需要回填参数的 SQL 日志标签。默认仅处理查询语句。
    #[serde(default = "default_tags")]
    pub tags: Vec<String>,

    /// 显式声明 SQL 中使用的占位符列表，例如 `["?"]` 或 `[":1"]`。
    /// - 只含 `"?"` → 仅匹配 `?` 顺序占位符
    /// - 含任意 `:N` 形式（如 `":1"`）→ 仅匹配 `:N` 序号占位符
    /// - 空数组（默认）→ 自动检测
    #[serde(default)]
    pub placeholders: Vec<String>,
}

fn default_tags() -> Vec<String> {
    vec!["SEL".to_string()]
}

impl Default for NormalizeConfig {
    fn default() -> Self {
        Self {
            tags: default_tags(),
            placeholders: Vec::new(),
        }
    }
}

impl NormalizeConfig {
    /// 将 `placeholders` 列表转换为 `compute_normalized` 所需的 `placeholder_override`：
    /// - `None`        → 自动检测
    /// - `Some(false)` → 强制 `?` 风格
    /// - `Some(true)`  → 强制 `:N` 风格
    #[must_use]
    pub fn placeholder_override(&self) -> Option<bool> {
        let has_question = self.placeholders.iter().any(|p| p == "?");
        let has_colon = self.placeholders.iter().any(|p| {
            p.starts_with(':') && p[1..].chars().next().is_some_and(|c| c.is_ascii_digit())
        });
        match (has_question, has_colon) {
            (true, false) => Some(false),
            (false, true) => Some(true),
            _ => None,
        }
    }
}

/// A single parameter value parsed from a `PARAMS(...)` log record.
#[derive(Debug, Clone)]
pub enum ParamValue {
    /// Single-quoted string already including the surrounding quotes, e.g. `'3USJ29'`.
    Quoted(String),
    /// Bare numeric literal, e.g. `2370075`.
    Bare(String),
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
#[must_use]
pub fn parse_params(body: &str) -> Option<Vec<ParamValue>> {
    // memmem 使用 Two-Way + SIMD 算法，比 str::find 快
    let brace = memchr::memmem::find(body.as_bytes(), b"={")?;
    let inner = body[brace + 2..].strip_suffix('}')?;

    let mut params = Vec::new();
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
        Some((ParamValue::Quoted(String::from(quoted)), tail))
    } else {
        // Bare number or empty — memchr for closing ')'
        let end = memchr::memchr(b')', s.as_bytes())?;
        let raw = s[..end].trim();
        let tail = &s[end + 1..];
        let value = if raw.is_empty() {
            ParamValue::Null
        } else {
            ParamValue::Bare(String::from(raw))
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
pub(super) fn apply_params_into(
    sql: &str,
    params: &[ParamValue],
    colon_style: bool,
    out: &mut Vec<u8>,
) {
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
