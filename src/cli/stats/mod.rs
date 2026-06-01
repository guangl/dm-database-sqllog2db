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
    let _ = quiet; // quiet 模式在本命令不改变输出行为
    crate::stats::run_stats(cfg, top)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, CsvExporterConfig, ExporterConfig, SqllogConfig};
    use crate::error::{ConfigError, Error};

    fn make_test_config_with_log() -> (Config, tempfile::TempDir) {
        let dir = tempfile::TempDir::new().unwrap();
        let log_file = dir.path().join("test.log");
        std::fs::write(
            &log_file,
            "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT id FROM t WHERE id=1. EXECTIME: 5(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n",
        ).unwrap();
        let cfg = Config {
            sqllog: SqllogConfig {
                inputs: vec![log_file.to_str().unwrap().to_string()],
                path_deprecated: None,
            },
            exporter: ExporterConfig {
                csv: Some(CsvExporterConfig {
                    file: dir.path().join("out.csv").to_str().unwrap().to_string(),
                    overwrite: true,
                    ..CsvExporterConfig::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        (cfg, dir)
    }

    #[test]
    fn test_handle_stats_top_default_passes() {
        let (cfg, _dir) = make_test_config_with_log();
        let result = handle_stats(&cfg, 20, false);
        assert!(result.is_ok(), "top=20 should succeed, got: {result:?}");
    }

    #[test]
    fn test_handle_stats_top_nonzero_passes() {
        let (cfg, _dir) = make_test_config_with_log();
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
