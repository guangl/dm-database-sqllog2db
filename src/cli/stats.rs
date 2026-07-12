//! Stats 子命令入口：合并 CLI 与配置选项后委托 `crate::stats::run_stats`。

use crate::config::Config;
use crate::error::Result;

fn merge_stats_options(
    cfg: &Config,
    cli_top: Option<u32>,
    cli_from: Option<String>,
    cli_to: Option<String>,
) -> (u32, Option<String>, Option<String>) {
    let effective_top: u32 = cli_top.or(cfg.stats.top).unwrap_or(20);
    let effective_from: Option<String> = cli_from.or_else(|| cfg.stats.from.clone());
    let effective_to: Option<String> = cli_to.or_else(|| cfg.stats.to.clone());
    (effective_top, effective_from, effective_to)
}

/// Handle the `stats` subcommand.
///
/// Merges CLI args with config values using priority: CLI > config > default.
/// `top` defaults to 20 when neither CLI nor config provides a value.
/// `cfg` must already have verbosity applied before calling this function.
pub fn handle_stats(
    cfg: &Config,
    top: Option<u32>,
    from: Option<String>,
    to: Option<String>,
) -> Result<()> {
    let (effective_top, effective_from, effective_to) = merge_stats_options(cfg, top, from, to);

    let mut merged_cfg = cfg.clone();
    merged_cfg.stats.top = Some(effective_top);
    merged_cfg.stats.from = effective_from;
    merged_cfg.stats.to = effective_to;

    log::info!(
        "stats: top={effective_top} from={:?} to={:?}",
        merged_cfg.stats.from,
        merged_cfg.stats.to
    );

    crate::stats::run_stats(&merged_cfg, effective_top)
}

#[cfg(test)]
mod tests {
    use super::{handle_stats, merge_stats_options};
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
        let result = handle_stats(&cfg, Some(20), None, None);
        assert!(result.is_ok(), "top=20 should succeed, got: {result:?}");
    }

    #[test]
    fn test_handle_stats_top_nonzero_passes() {
        let (cfg, _dir) = make_test_config_with_log();
        let result = handle_stats(&cfg, Some(5), None, None);
        assert!(result.is_ok(), "top=5 should succeed, got: {result:?}");
    }

    #[test]
    fn test_handle_stats_cli_none_config_none_falls_back_to_20() {
        let (cfg, _dir) = make_test_config_with_log();
        let result = handle_stats(&cfg, None, None, None);
        assert!(
            result.is_ok(),
            "default top=20 fallback should succeed, got: {result:?}"
        );
    }

    #[test]
    fn test_handle_stats_cli_top_overrides_config_top() {
        let (mut cfg, _dir) = make_test_config_with_log();
        cfg.stats.top = Some(10);
        let result = handle_stats(&cfg, Some(5), None, None);
        assert!(
            result.is_ok(),
            "CLI top=5 should override config top=10, got: {result:?}"
        );
    }

    #[test]
    fn test_handle_stats_config_top_used_when_cli_none() {
        let (mut cfg, _dir) = make_test_config_with_log();
        cfg.stats.top = Some(7);
        let result = handle_stats(&cfg, None, None, None);
        assert!(
            result.is_ok(),
            "config top=7 should be used when CLI=None, got: {result:?}"
        );
    }

    #[test]
    fn test_merge_stats_options_cli_top_priority() {
        let cfg = Config::default();
        let (top, from, to) = merge_stats_options(&cfg, Some(5), None, None);
        assert_eq!(top, 5);
        assert_eq!(from, None);
        assert_eq!(to, None);
    }

    #[test]
    fn test_merge_stats_options_config_top_fallback() {
        let mut cfg = Config::default();
        cfg.stats.top = Some(10);
        cfg.stats.from = Some("2024-01-01".to_string());
        let (top, from, to) = merge_stats_options(&cfg, None, None, None);
        assert_eq!(top, 10);
        assert_eq!(from, Some("2024-01-01".to_string()));
        assert_eq!(to, None);
    }

    #[test]
    fn test_merge_stats_options_default_fallback() {
        let cfg = Config::default();
        let (top, from, to) = merge_stats_options(&cfg, None, None, None);
        assert_eq!(top, 20);
        assert_eq!(from, None);
        assert_eq!(to, None);
    }
}
