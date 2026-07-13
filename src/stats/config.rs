//! Stats 子命令配置：时间段过滤字段与时间格式验证工具函数。

use crate::error::{ConfigError, Error};

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

/// 验证 `StatsConfig` 的 from/to 字段格式；供 `Config::validate` 和 `run_stats` 共用（IN-02）。
///
/// # Errors
///
/// from/to 不符合 `YYYY-MM-DD[ HH:MM:SS]` 格式，或 from 晚于 to 时返回错误。
pub fn validate_stats_time_range(stats: &StatsConfig) -> crate::error::Result<()> {
    if let Some(from) = &stats.from {
        validate_time_str(from).map_err(|reason| {
            Error::Config(ConfigError::InvalidValue {
                field: "stats.from".to_string(),
                value: from.clone(),
                reason,
            })
        })?;
    }
    if let Some(to) = &stats.to {
        validate_time_str(to).map_err(|reason| {
            Error::Config(ConfigError::InvalidValue {
                field: "stats.to".to_string(),
                value: to.clone(),
                reason,
            })
        })?;
    }
    if let (Some(from), Some(to)) = (&stats.from, &stats.to) {
        // Compare only the common prefix so "2024-01-15 00:00:00" and "2024-01-15"
        // are treated as equal at the day boundary (matches aggregate.rs prefix logic).
        let cmp_len = from.len().min(to.len());
        if from.as_bytes()[..cmp_len] > to.as_bytes()[..cmp_len] {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "stats.from".to_string(),
                value: from.clone(),
                reason: format!("stats.from ({from}) must be <= stats.to ({to})"),
            }));
        }
    }
    Ok(())
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

/// 检查 bytes[0..10] 是否符合 `YYYY-MM-DD` 格式（位置 + 数字 + 月/日范围校验）。
fn check_date_part(bytes: &[u8]) -> bool {
    debug_assert!(bytes.len() >= 10, "check_date_part: need at least 10 bytes");
    if !(bytes[4] == b'-' && bytes[7] == b'-') {
        return false;
    }
    if !bytes[..4].iter().all(u8::is_ascii_digit) {
        return false;
    }
    if !bytes[5..7].iter().all(u8::is_ascii_digit) {
        return false;
    }
    if !bytes[8..10].iter().all(u8::is_ascii_digit) {
        return false;
    }
    let month = (bytes[5] - b'0') * 10 + (bytes[6] - b'0');
    let day = (bytes[8] - b'0') * 10 + (bytes[9] - b'0');
    if !(1..=12).contains(&month) {
        return false;
    }
    let max_day: u8 = match month {
        2 => 29,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=max_day).contains(&day)
}

/// 检查 bytes[10..19] 是否符合 ` HH:MM:SS` 格式（位置 + 数字 + 时/分/秒范围校验）。
fn check_time_part(bytes: &[u8]) -> bool {
    debug_assert!(bytes.len() >= 19, "check_time_part: need at least 19 bytes");
    if !(bytes[10] == b' ' && bytes[13] == b':' && bytes[16] == b':') {
        return false;
    }
    if !bytes[11..13]
        .iter()
        .chain(bytes[14..16].iter())
        .chain(bytes[17..19].iter())
        .all(u8::is_ascii_digit)
    {
        return false;
    }
    let hour = (bytes[11] - b'0') * 10 + (bytes[12] - b'0');
    let min = (bytes[14] - b'0') * 10 + (bytes[15] - b'0');
    let sec = (bytes[17] - b'0') * 10 + (bytes[18] - b'0');
    hour <= 23 && min <= 59 && sec <= 59
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
    fn test_validate_time_str_rejects_month_zero() {
        assert!(validate_time_str("2024-00-01").is_err());
    }

    #[test]
    fn test_validate_time_str_rejects_month_13() {
        assert!(validate_time_str("2024-13-01").is_err());
    }

    #[test]
    fn test_validate_time_str_rejects_day_zero() {
        assert!(validate_time_str("2024-01-00").is_err());
    }

    #[test]
    fn test_validate_time_str_rejects_day_32() {
        assert!(validate_time_str("2024-01-32").is_err());
    }

    #[test]
    fn test_validate_time_str_rejects_hour_24() {
        assert!(validate_time_str("2024-01-01 24:00:00").is_err());
    }

    #[test]
    fn test_validate_time_str_rejects_minute_60() {
        assert!(validate_time_str("2024-01-01 00:60:00").is_err());
    }

    #[test]
    fn test_validate_time_str_rejects_second_60() {
        assert!(validate_time_str("2024-01-01 00:00:60").is_err());
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

    #[test]
    fn test_validate_stats_time_range_rejects_from_after_to() {
        let cfg = StatsConfig {
            from: Some("2024-01-31".to_string()),
            to: Some("2024-01-01".to_string()),
            top: None,
        };
        let result = validate_stats_time_range(&cfg);
        assert!(result.is_err(), "from > to should return Err");
        match result.unwrap_err() {
            crate::error::Error::Config(crate::error::ConfigError::InvalidValue {
                field,
                value,
                reason,
            }) => {
                assert_eq!(field, "stats.from");
                assert_eq!(value, "2024-01-31");
                assert!(
                    reason.contains("must be <="),
                    "reason should contain 'must be <=': {reason}"
                );
                assert!(
                    reason.contains("2024-01-31"),
                    "reason should contain from value: {reason}"
                );
                assert!(
                    reason.contains("2024-01-01"),
                    "reason should contain to value: {reason}"
                );
            }
            err => panic!("expected ConfigError::InvalidValue, got: {err:?}"),
        }
    }

    #[test]
    fn test_validate_stats_time_range_accepts_equal_from_to() {
        let cfg = StatsConfig {
            from: Some("2024-01-15".to_string()),
            to: Some("2024-01-15".to_string()),
            top: None,
        };
        assert!(
            validate_stats_time_range(&cfg).is_ok(),
            "from == to should be accepted"
        );
    }

    #[test]
    fn test_validate_stats_time_range_accepts_from_only() {
        let cfg_from_only = StatsConfig {
            from: Some("2024-01-15".to_string()),
            to: None,
            top: None,
        };
        assert!(
            validate_stats_time_range(&cfg_from_only).is_ok(),
            "only from should be accepted"
        );
        let cfg_to_only = StatsConfig {
            from: None,
            to: Some("2024-01-15".to_string()),
            top: None,
        };
        assert!(
            validate_stats_time_range(&cfg_to_only).is_ok(),
            "only to should be accepted"
        );
    }

    #[test]
    fn test_validate_stats_time_range_accepts_ordered() {
        let cfg = StatsConfig {
            from: Some("2024-01-01".to_string()),
            to: Some("2024-01-31".to_string()),
            top: None,
        };
        assert!(
            validate_stats_time_range(&cfg).is_ok(),
            "from < to should be accepted"
        );
    }

    #[test]
    fn test_validate_stats_time_range_accepts_datetime_from_with_date_to() {
        let cfg = StatsConfig {
            from: Some("2024-01-15 00:00:00".to_string()),
            to: Some("2024-01-15".to_string()),
            top: None,
        };
        assert!(
            validate_stats_time_range(&cfg).is_ok(),
            "datetime from at start of to-date should be accepted"
        );
    }

    #[test]
    fn test_validate_time_str_rejects_feb_31() {
        assert!(validate_time_str("2024-02-31").is_err());
    }

    #[test]
    fn test_validate_time_str_rejects_apr_31() {
        assert!(validate_time_str("2024-04-31").is_err());
    }

    #[test]
    fn test_validate_time_str_accepts_feb_29() {
        assert!(validate_time_str("2024-02-29").is_ok());
    }
}
