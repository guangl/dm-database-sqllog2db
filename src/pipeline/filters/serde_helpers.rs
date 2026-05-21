use regex::Regex;
use serde::{Deserialize, Deserializer};

/// `trxid` 过滤集合类型。
pub(super) type TrxidSet = std::collections::HashSet<String>;

pub(super) fn vec_to_hashset<'de, D>(deserializer: D) -> Result<Option<TrxidSet>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<Vec<String>> = Option::deserialize(deserializer)?;
    Ok(v.map(|items| items.into_iter().collect()))
}

pub(super) fn vec_to_i64_hashset<'de, D>(
    deserializer: D,
) -> Result<Option<std::collections::HashSet<i64>>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<Vec<i64>> = Option::deserialize(deserializer)?;
    Ok(v.map(|items| items.into_iter().collect()))
}

/// 将正则字符串列表编译为 `Vec<Regex>`。None 或空列表返回 `Ok(None)`（未配置）。
/// 遇到非法正则时返回 `ConfigError::InvalidValue`，field 参数用于错误消息。
pub(super) fn compile_patterns(
    field: &str,
    patterns: Option<&[String]>,
) -> crate::error::Result<Option<Vec<Regex>>> {
    match patterns {
        None | Some([]) => Ok(None),
        Some(v) => {
            let compiled = v
                .iter()
                .map(|p| {
                    Regex::new(p).map_err(|e| {
                        crate::error::Error::Config(crate::error::ConfigError::InvalidValue {
                            field: field.to_string(),
                            value: p.clone(),
                            reason: format!("invalid regex: {e}"),
                        })
                    })
                })
                .collect::<crate::error::Result<Vec<_>>>()?;
            Ok(Some(compiled))
        }
    }
}

/// None 表示"未配置，直接通过"；Some(patterns) 表示"任意一个匹配即满足"。
#[inline]
pub(super) fn match_any_regex(patterns: Option<&[Regex]>, val: &str) -> bool {
    match patterns {
        None | Some([]) => true,
        Some(p) => p.iter().any(|re| re.is_match(val)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── compile_patterns ───────────────────────────────────────
    #[test]
    fn test_compile_patterns_none() {
        let result = compile_patterns("test.field", None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_compile_patterns_empty() {
        let result = compile_patterns("test.field", Some(&[]));
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_compile_patterns_valid() {
        let patterns = vec!["^admin.*".to_string()];
        let result = compile_patterns("test.field", Some(&patterns));
        assert!(result.is_ok());
        let compiled = result.unwrap();
        assert!(compiled.is_some());
        assert_eq!(compiled.unwrap().len(), 1);
    }

    #[test]
    fn test_compile_patterns_invalid() {
        let patterns = vec!["[invalid".to_string()];
        let result = compile_patterns("test.field", Some(&patterns));
        assert!(result.is_err());
    }

    // ── match_any_regex ────────────────────────────────────────
    #[test]
    fn test_match_any_regex_none_passes() {
        assert!(match_any_regex(None, "anything"));
    }

    #[test]
    fn test_match_any_regex_empty_passes() {
        assert!(match_any_regex(Some(&[]), "anything"));
    }

    #[test]
    fn test_match_any_regex_match() {
        let re = Regex::new("^admin").unwrap();
        assert!(match_any_regex(Some(&[re]), "admin_dba"));
    }

    #[test]
    fn test_match_any_regex_no_match() {
        let re = Regex::new("^admin").unwrap();
        assert!(!match_any_regex(Some(&[re]), "sys_admin"));
    }
}
