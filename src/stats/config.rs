//! Stats 子命令配置：时间段过滤字段与时间格式验证工具函数。

/// Stats 子命令的配置字段：起止时间（可选）与 top-N 数量（可选）。
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct StatsConfig {
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub top: Option<u32>,
}

/// 验证时间字符串格式。
///
/// 支持两种格式：
/// - `"YYYY-MM-DD"`（10 个字符）
/// - `"YYYY-MM-DD HH:MM:SS"`（19 个字符）
///
/// # Errors
///
/// 如果格式不符合要求，返回包含格式说明的错误字符串。
pub fn validate_time_str(s: &str) -> Result<(), String> {
    let err = || r#"格式不合法，支持 "YYYY-MM-DD" 或 "YYYY-MM-DD HH:MM:SS""#.to_string();

    if !s.is_ascii() {
        return Err(err());
    }

    let bytes = s.as_bytes();
    match bytes.len() {
        10 => {
            if check_date_part(bytes) {
                Ok(())
            } else {
                Err(err())
            }
        }
        19 => {
            if check_date_part(bytes) && check_time_part(bytes) {
                Ok(())
            } else {
                Err(err())
            }
        }
        _ => Err(err()),
    }
}

/// 检查 bytes[0..10] 是否符合 `YYYY-MM-DD` 格式（位置校验 + 数字校验）。
fn check_date_part(bytes: &[u8]) -> bool {
    bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_digit()
        && bytes[5].is_ascii_digit()
        && bytes[6].is_ascii_digit()
        && bytes[8].is_ascii_digit()
        && bytes[9].is_ascii_digit()
}

/// 检查 bytes[10..19] 是否符合 ` HH:MM:SS` 格式（位置校验 + 数字校验）。
fn check_time_part(bytes: &[u8]) -> bool {
    bytes[10] == b' '
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes[11].is_ascii_digit()
        && bytes[12].is_ascii_digit()
        && bytes[14].is_ascii_digit()
        && bytes[15].is_ascii_digit()
        && bytes[17].is_ascii_digit()
        && bytes[18].is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_time_str_accepts_date_only() {
        assert!(validate_time_str("2024-01-01").is_ok());
    }

    #[test]
    fn test_validate_time_str_accepts_datetime() {
        assert!(validate_time_str("2024-12-31 23:59:59").is_ok());
    }

    #[test]
    fn test_validate_time_str_rejects_no_separator() {
        let result = validate_time_str("20240101");
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("YYYY-MM-DD"),
            "error should contain YYYY-MM-DD: {msg}"
        );
        assert!(
            msg.contains("YYYY-MM-DD HH:MM:SS"),
            "error should contain YYYY-MM-DD HH:MM:SS: {msg}"
        );
    }

    #[test]
    fn test_validate_time_str_rejects_not_a_date() {
        let result = validate_time_str("not-a-date");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_time_str_rejects_short_date() {
        let result = validate_time_str("2024-1-1");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_time_str_rejects_t_separator() {
        let result = validate_time_str("2024-01-01T12:00:00");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_time_str_rejects_slash_separator() {
        let result = validate_time_str("2024/01/01");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_time_str_rejects_empty() {
        let result = validate_time_str("");
        assert!(result.is_err());
    }

    #[test]
    fn test_stats_config_default_all_none() {
        let cfg = StatsConfig::default();
        assert!(cfg.from.is_none());
        assert!(cfg.to.is_none());
        assert!(cfg.top.is_none());
    }

    #[test]
    fn test_stats_config_deserialize_empty_toml() {
        #[derive(serde::Deserialize)]
        struct W {
            stats: StatsConfig,
        }
        let w: W = toml::from_str("[stats]").unwrap();
        assert!(w.stats.from.is_none());
        assert!(w.stats.to.is_none());
        assert!(w.stats.top.is_none());
    }

    #[test]
    fn test_stats_config_deserialize_partial_toml() {
        #[derive(serde::Deserialize)]
        struct W {
            stats: StatsConfig,
        }
        let w: W = toml::from_str("[stats]\nfrom = \"2024-01-01\"\ntop = 10").unwrap();
        assert_eq!(w.stats.from, Some("2024-01-01".to_string()));
        assert!(w.stats.to.is_none());
        assert_eq!(w.stats.top, Some(10));
    }
}
