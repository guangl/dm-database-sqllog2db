use crate::config::Config;
use crate::error::{ConfigError, Error, Result};

/// Handle the `stats` subcommand.
///
/// Validates `top` is non-zero, then delegates to Phase 52 statistics logic.
/// `cfg` must already have verbosity applied before calling this function.
pub fn handle_stats(cfg: &Config, top: u32, quiet: bool) -> Result<()> {
    if top == 0 {
        return Err(Error::Config(ConfigError::InvalidValue {
            field: "--top".to_string(),
            value: "0".to_string(),
            reason: "must be >= 1".to_string(),
        }));
    }
    log::info!("stats: top={top}");
    // TODO(Phase 52): statistics logic
    let _ = (cfg, quiet); // suppress unused warnings until Phase 52
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::error::{ConfigError, Error};

    #[test]
    fn test_handle_stats_top_default_passes() {
        let cfg = Config::default();
        let result = handle_stats(&cfg, 20, false);
        assert!(result.is_ok(), "top=20 should succeed, got: {result:?}");
    }

    #[test]
    fn test_handle_stats_top_nonzero_passes() {
        let cfg = Config::default();
        let result = handle_stats(&cfg, 5, false);
        assert!(result.is_ok(), "top=5 should succeed, got: {result:?}");
    }

    #[test]
    fn test_handle_stats_top_zero_returns_invalid_value_error() {
        let cfg = Config::default();
        let result = handle_stats(&cfg, 0, false);
        assert!(result.is_err(), "top=0 should return an error");
        match result.unwrap_err() {
            Error::Config(ConfigError::InvalidValue {
                field,
                value,
                reason,
            }) => {
                assert_eq!(field, "--top");
                assert_eq!(value, "0");
                assert!(
                    reason.contains("must be >= 1"),
                    "reason should contain 'must be >= 1', got: {reason}"
                );
            }
            other => panic!("expected ConfigError::InvalidValue, got: {other:?}"),
        }
    }

    #[test]
    fn test_handle_stats_top_zero_quiet_still_errors() {
        let cfg = Config::default();
        let result = handle_stats(&cfg, 0, true);
        assert!(
            result.is_err(),
            "top=0 with quiet=true should still return an error"
        );
        match result.unwrap_err() {
            Error::Config(ConfigError::InvalidValue { field, .. }) => {
                assert_eq!(field, "--top");
            }
            other => panic!("expected ConfigError::InvalidValue, got: {other:?}"),
        }
    }
}
