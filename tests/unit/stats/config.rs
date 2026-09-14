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
