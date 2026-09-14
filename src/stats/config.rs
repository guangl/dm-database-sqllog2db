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
#[path = "../../tests/unit/stats/config.rs"]
mod tests;
