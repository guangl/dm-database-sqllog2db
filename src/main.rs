mod cli;
mod config;
mod error;
mod exporter;
mod logging;
mod parser;
mod pipeline;
mod preflight;

use config::Config;
use error::{Error, ErrorStats, Result};
use log::{info, warn};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

// 退出码约定：
// 0 = 全部成功，零错误
// 1 = 处理完成但有非致命错误（部分成功）
// 2 = 致命错误，无法完成处理
// 130 = 被用户中断（Ctrl+C），遵循 Unix 128+SIGINT(2) 惯例
const EXIT_PARTIAL: i32 = 1;
const EXIT_FATAL: i32 = 2;
const EXIT_INTERRUPTED: i32 = 130;

/// Initialize simple console logging for non-run commands
fn init_simple_logging(_verbose: bool, quiet: bool) {
    let level = if quiet { "error" } else { "info" };

    let filter = match level {
        "error" => log::LevelFilter::Error,
        _ => log::LevelFilter::Info,
    };

    let _ = env_logger::Builder::from_default_env()
        .filter_level(filter)
        .try_init();
}

/// Apply CLI verbosity flags to configuration
fn apply_verbosity_to_config(cfg: &mut Config, _verbose: bool, quiet: bool) {
    if quiet {
        cfg.logging.level = "error".to_string();
    }
}

/// Apply CLI --input overrides to configuration.
/// Per D-05: CLI inputs completely replace config inputs when Some; not merged.
fn apply_cli_inputs_to_config(cfg: &mut Config, cli_inputs: Option<Vec<String>>) {
    if let Some(inputs) = cli_inputs {
        if !inputs.is_empty() {
            cfg.sqllog.inputs = inputs;
        }
    }
}

/// Format a fatal error into a multi-line string for stderr output.
/// First line: `[SEVERITY] display_text`
/// Second line (if hint non-empty): `  hint: suggestion_text`
fn format_error_output(error: &Error) -> String {
    let severity = error.severity();
    let hint = error.suggestion();
    if hint.is_empty() {
        format!("[{severity}] {error}")
    } else {
        format!("[{severity}] {error}\n  hint: {hint}")
    }
}

fn main() {
    match run() {
        Ok(Some((stats, quiet))) => {
            if stats.has_fatal() {
                std::process::exit(EXIT_FATAL);
            }
            if stats.has_errors() {
                if !quiet {
                    eprintln!(
                        "Completed with {} error(s) ({} parse, {} export).",
                        stats.total_errors, stats.parse_errors, stats.export_errors
                    );
                }
                std::process::exit(EXIT_PARTIAL);
            }
            // EXIT_CLEAN (0) is default
        }
        Ok(None) => {} // non-Run commands: normal exit
        Err(e) => {
            if matches!(e, Error::Interrupted) {
                std::process::exit(EXIT_INTERRUPTED);
            }
            eprintln!("{}", format_error_output(&e));
            std::process::exit(EXIT_FATAL);
        }
    }
}

fn run() -> Result<Option<(ErrorStats, bool)>> {
    use clap::{CommandFactory, FromArgMatches, Parser};

    let cmd = cli::opts::Cli::command();
    let matches = cmd.get_matches();
    let cli = cli::opts::Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

    let needs_simple_logging = !matches!(&cli.command, Some(cli::opts::Commands::Run { .. }));
    if needs_simple_logging {
        init_simple_logging(cli.verbose, cli.quiet);
    }

    match &cli.command {
        Some(cli::opts::Commands::Init { output, force }) => {
            cli::init::handle_init(output, *force)?;
            Ok(None)
        }
        Some(cli::opts::Commands::Run { config, input }) => {
            let mut cfg = load_config(config)?;
            apply_cli_inputs_to_config(&mut cfg, input.clone());
            let compiled_filters = cfg.validate_and_compile()?;

            apply_verbosity_to_config(&mut cfg, cli.verbose, cli.quiet);
            logging::init_logging(&cfg.logging, false)?;
            info!("Application started");
            info!("Configuration validation passed");

            let pf = preflight::check(&cfg);
            if pf.print_and_check() {
                std::process::exit(EXIT_FATAL);
            }

            let interrupted = Arc::new(AtomicBool::new(false));
            let interrupted_flag = Arc::clone(&interrupted);
            ctrlc::set_handler(move || {
                interrupted_flag.store(true, Ordering::Relaxed);
            })
            .ok();

            let stats =
                cli::run::handle_run(&cfg, cli.quiet, cli.verbose, &interrupted, compiled_filters)?;
            Ok(Some((stats, cli.quiet)))
        }
        Some(cli::opts::Commands::Validate { config }) => {
            let cfg = load_config(config)?;
            if let Err(e) = cfg.validate() {
                let hint = e.suggestion();
                if hint.is_empty() {
                    eprintln!("[FAIL] {e}");
                } else {
                    eprintln!("[FAIL] {e}\n  hint: {hint}");
                }
                std::process::exit(EXIT_FATAL);
            }
            cli::validate::handle_validate(&cfg);
            Ok(None)
        }
        None => {
            let _ = cli::opts::Cli::try_parse_from(["sqllog2db", "--help"]);
            std::process::exit(1);
        }
    }
}

fn load_config(config_path: &str) -> Result<Config> {
    let path = Path::new(config_path);
    match Config::from_file(path) {
        Ok(c) => {
            info!("Loaded configuration file: {config_path}");
            Ok(c)
        }
        Err(e) => {
            if let Error::Config(crate::error::ConfigError::NotFound(_)) = &e {
                warn!("Configuration file not found: {config_path}, using default configuration");
                info!("Tip: run 'sqllog2db init' to generate a configuration file");
                Ok(Config::default())
            } else {
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{ConfigError, ExportError, ParserError};

    #[test]
    fn test_exit_code_clean() {
        let stats = ErrorStats::default();
        assert!(!stats.has_errors());
        assert!(!stats.has_fatal());
    }

    #[test]
    fn test_exit_code_partial_errors() {
        let mut stats = ErrorStats::default();
        stats.add_parse_error();
        assert!(stats.has_errors());
        assert!(!stats.has_fatal());
    }

    #[test]
    fn test_exit_code_fatal_error() {
        let mut stats = ErrorStats::default();
        stats.set_fatal("test fatal".into());
        assert!(stats.has_fatal());
    }

    #[test]
    fn test_error_is_fatal_for_config() {
        let e = Error::Config(ConfigError::NoExporters);
        assert!(e.is_fatal());
        assert_eq!(e.severity(), crate::error::ErrorSeverity::Critical);
    }

    #[test]
    fn test_error_is_fatal_for_parse_error() {
        let e = Error::Parser(ParserError::PathNotFound {
            path: "/tmp".into(),
        });
        assert!(!e.is_fatal());
        assert_eq!(e.severity(), crate::error::ErrorSeverity::Warning);
    }

    #[test]
    fn test_error_suggestion_for_config_not_found() {
        let e = Error::Config(ConfigError::NotFound("/tmp/config.toml".into()));
        assert!(e.suggestion().contains("sqllog2db init"));
    }

    #[test]
    fn test_error_suggestion_for_export_write_failed() {
        let e = Error::Export(ExportError::WriteFailed {
            path: "/tmp/out.csv".into(),
            reason: "disk full".into(),
        });
        assert!(!e.is_fatal());
        assert!(!e.suggestion().is_empty());
    }

    #[test]
    fn test_apply_verbosity_verbose() {
        let mut cfg = Config::default();
        apply_verbosity_to_config(&mut cfg, true, false);
        assert_eq!(cfg.logging.level, Config::default().logging.level);
    }

    #[test]
    fn test_apply_verbosity_quiet() {
        let mut cfg = Config::default();
        apply_verbosity_to_config(&mut cfg, false, true);
        assert_eq!(cfg.logging.level, "error");
    }

    #[test]
    fn test_apply_verbosity_neither() {
        let mut cfg = Config::default();
        let original = cfg.logging.level.clone();
        apply_verbosity_to_config(&mut cfg, false, false);
        assert_eq!(cfg.logging.level, original);
    }

    #[test]
    fn test_load_config_not_found_returns_default() {
        let result = load_config("/nonexistent/path/config.toml");
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_config_invalid_toml_returns_error() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, "not valid toml ][[[").unwrap();
        let result = load_config(path.to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_cli_inputs_none_keeps_config() {
        let mut cfg = Config::default();
        // Default inputs = ["sqllogs"]
        assert_eq!(cfg.sqllog.inputs, vec!["sqllogs".to_string()]);
        apply_cli_inputs_to_config(&mut cfg, None);
        assert_eq!(
            cfg.sqllog.inputs,
            vec!["sqllogs".to_string()],
            "None should not change config inputs"
        );
    }

    #[test]
    fn test_apply_cli_inputs_some_replaces() {
        let mut cfg = Config::default();
        cfg.sqllog.inputs = vec!["a".to_string()];
        apply_cli_inputs_to_config(&mut cfg, Some(vec!["b".to_string(), "c".to_string()]));
        assert_eq!(
            cfg.sqllog.inputs,
            vec!["b".to_string(), "c".to_string()],
            "Some(non-empty) should completely replace config inputs"
        );
    }

    #[test]
    fn test_apply_cli_inputs_empty_vec_keeps_config() {
        let mut cfg = Config::default();
        cfg.sqllog.inputs = vec!["x".to_string()];
        apply_cli_inputs_to_config(&mut cfg, Some(vec![]));
        assert_eq!(
            cfg.sqllog.inputs,
            vec!["x".to_string()],
            "Some(empty vec) should not change config inputs"
        );
    }

    #[test]
    fn test_error_io_suggestion_non_empty() {
        let e = Error::Io(std::io::Error::other("disk full"));
        let suggestion = e.suggestion();
        assert!(!suggestion.is_empty(), "Io suggestion should not be empty");
        assert!(
            suggestion.contains("filesystem"),
            "Io suggestion should mention filesystem, got: {suggestion}"
        );
    }

    #[test]
    fn test_error_print_format_uses_hint_prefix() {
        let e = Error::Export(ExportError::WriteFailed {
            path: "/tmp/out.csv".into(),
            reason: "disk full".into(),
        });
        let formatted = format_error_output(&e);
        assert!(
            formatted.contains("\n  hint: "),
            "formatted output should contain hint prefix, got: {formatted}"
        );
        assert!(
            !formatted.contains("Suggestion:"),
            "formatted output should not contain old Suggestion: prefix, got: {formatted}"
        );
        assert!(
            formatted.starts_with("[ERROR]"),
            "first line should start with [ERROR], got: {formatted}"
        );
    }
}
