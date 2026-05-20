use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod cli;
mod config;
mod error;
mod exporter;
mod logging;
mod parser;
mod pipeline;
mod preflight;

use config::Config;
use error::Result;
use log::{info, warn};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

// 退出码约定：
// 0  = 成功
// 1  = 未分类错误
// 2  = 配置错误
// 3  = 输入/文件/解析错误
// 4  = 导出错误
// 130 = 被用户中断（Ctrl+C），遵循 Unix 128+SIGINT(2) 惯例
const EXIT_CONFIG: i32 = 2;
const EXIT_IO: i32 = 3;
const EXIT_EXPORT: i32 = 4;
const EXIT_INTERRUPTED: i32 = 130;

fn exit_code_for(e: &error::Error) -> i32 {
    match e {
        error::Error::Config(_) => EXIT_CONFIG,
        error::Error::File(_) | error::Error::Parser(_) | error::Error::Io(_) => EXIT_IO,
        error::Error::Export(_) => EXIT_EXPORT,
        error::Error::Interrupted => EXIT_INTERRUPTED,
    }
}

/// Initialize simple console logging for non-run commands
fn init_simple_logging(verbose: u8, quiet: bool) {
    let level = if verbose >= 2 {
        "trace"
    } else if verbose >= 1 {
        "debug"
    } else if quiet {
        "error"
    } else {
        "info"
    };

    let filter = match level {
        "trace" => log::LevelFilter::Trace,
        "debug" => log::LevelFilter::Debug,
        "error" => log::LevelFilter::Error,
        _ => log::LevelFilter::Info,
    };

    let _ = env_logger::Builder::from_default_env()
        .filter_level(filter)
        .try_init();
}

/// Apply CLI verbosity flags to configuration
fn apply_verbosity_to_config(cfg: &mut Config, verbose: u8, quiet: bool) {
    if verbose >= 1 {
        cfg.logging.level = "debug".to_string();
    } else if quiet {
        cfg.logging.level = "error".to_string();
    }
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => {
            let code = exit_code_for(&e);
            if code != EXIT_INTERRUPTED {
                eprintln!("Error: {e}");
            }
            std::process::exit(code);
        }
    }
}

fn run() -> Result<()> {
    use clap::{CommandFactory, FromArgMatches, Parser};

    let cmd = cli::opts::Cli::command();
    let matches = cmd.get_matches();
    let cli = cli::opts::Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

    let needs_simple_logging = !matches!(&cli.command, Some(cli::opts::Commands::Run { .. }));
    if needs_simple_logging {
        init_simple_logging(cli.verbose, cli.quiet);
    }

    match &cli.command {
        Some(cli::opts::Commands::Init { output, force }) => cli::init::handle_init(output, *force),
        Some(cli::opts::Commands::Run { config }) => {
            let mut cfg = load_config(config)?;
            let compiled_filters = cfg.validate_and_compile()?;

            apply_verbosity_to_config(&mut cfg, cli.verbose, cli.quiet);
            logging::init_logging(&cfg.logging, false)?;
            info!("Application started");
            info!("Configuration validation passed");

            let pf = preflight::check(&cfg);
            if pf.print_and_check() {
                std::process::exit(EXIT_CONFIG);
            }

            let interrupted = Arc::new(AtomicBool::new(false));
            let interrupted_flag = Arc::clone(&interrupted);
            ctrlc::set_handler(move || {
                interrupted_flag.store(true, Ordering::Relaxed);
            })
            .ok();

            cli::run::handle_run(&cfg, cli.quiet, &interrupted, compiled_filters)
        }
        Some(cli::opts::Commands::Validate { config }) => {
            let mut cfg = load_config(config)?;
            cfg.validate()?;

            apply_verbosity_to_config(&mut cfg, cli.verbose, cli.quiet);
            logging::init_logging(&cfg.logging, true)?;
            info!("Application started");
            info!("Configuration validation passed");

            cli::validate::handle_validate(&cfg);
            Ok(())
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
            if let error::Error::Config(error::ConfigError::NotFound(_)) = &e {
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
    use crate::error::{ConfigError, ExportError, FileError, ParserError};

    #[test]
    fn test_exit_code_config_error() {
        let e = error::Error::Config(ConfigError::NoExporters);
        assert_eq!(exit_code_for(&e), EXIT_CONFIG);
    }

    #[test]
    fn test_exit_code_file_error() {
        let e = error::Error::File(FileError::CreateDirectoryFailed {
            path: "/tmp".into(),
            reason: "test".into(),
        });
        assert_eq!(exit_code_for(&e), EXIT_IO);
    }

    #[test]
    fn test_exit_code_parser_error() {
        let e = error::Error::Parser(ParserError::PathNotFound {
            path: "/tmp".into(),
        });
        assert_eq!(exit_code_for(&e), EXIT_IO);
    }

    #[test]
    fn test_exit_code_io_error() {
        let e = error::Error::Io(std::io::Error::other("test io"));
        assert_eq!(exit_code_for(&e), EXIT_IO);
    }

    #[test]
    fn test_exit_code_export_error() {
        let e = error::Error::Export(ExportError::DatabaseFailed {
            reason: "test".into(),
        });
        assert_eq!(exit_code_for(&e), EXIT_EXPORT);
    }

    #[test]
    fn test_exit_code_interrupted() {
        assert_eq!(exit_code_for(&error::Error::Interrupted), EXIT_INTERRUPTED);
    }

    #[test]
    fn test_apply_verbosity_verbose() {
        let mut cfg = Config::default();
        apply_verbosity_to_config(&mut cfg, 1, false);
        assert_eq!(cfg.logging.level, "debug");
    }

    #[test]
    fn test_apply_verbosity_quiet() {
        let mut cfg = Config::default();
        apply_verbosity_to_config(&mut cfg, 0, true);
        assert_eq!(cfg.logging.level, "error");
    }

    #[test]
    fn test_apply_verbosity_neither() {
        let mut cfg = Config::default();
        let original = cfg.logging.level.clone();
        apply_verbosity_to_config(&mut cfg, 0, false);
        assert_eq!(cfg.logging.level, original);
    }

    #[test]
    fn test_apply_verbosity_trace() {
        let mut cfg = Config::default();
        apply_verbosity_to_config(&mut cfg, 2, false);
        assert_eq!(cfg.logging.level, "debug");
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
    fn test_init_simple_logging_info() {
        init_simple_logging(0, false);
    }

    #[test]
    fn test_init_simple_logging_verbose() {
        init_simple_logging(1, false);
    }

    #[test]
    fn test_init_simple_logging_quiet() {
        init_simple_logging(0, true);
    }

    #[test]
    fn test_init_simple_logging_trace() {
        init_simple_logging(2, false);
    }
}
