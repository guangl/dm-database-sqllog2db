use crate::config::Config;
use crate::error::Result;

/// Handle the `stats` subcommand.
///
/// Delegates to Phase 52 statistics logic. `top` must be >= 1 (enforced by clap's
/// `value_parser` before reaching this function).
/// `cfg` must already have verbosity applied before calling this function.
pub fn handle_stats(cfg: &Config, top: u32) -> Result<()> {
    log::info!("stats: top={top}");
    crate::stats::run_stats(cfg, top)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, CsvExporterConfig, ExporterConfig, SqllogConfig};

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
        let result = handle_stats(&cfg, 20);
        assert!(result.is_ok(), "top=20 should succeed, got: {result:?}");
    }

    #[test]
    fn test_handle_stats_top_nonzero_passes() {
        let (cfg, _dir) = make_test_config_with_log();
        let result = handle_stats(&cfg, 5);
        assert!(result.is_ok(), "top=5 should succeed, got: {result:?}");
    }
}
