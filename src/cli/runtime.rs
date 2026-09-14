//! CLI configuration, logging setup and error presentation.
use crate::config::Config;
use crate::error::{Error, Result};
use log::{info, warn};
use std::path::Path;

/// Initialize console diagnostics without creating a log file.
/// Also used by run/stats when no logging section is configured.
pub(super) fn init_simple_logging(quiet: bool, verbose: bool) {
    let filter = if quiet {
        log::LevelFilter::Error
    } else if verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    let _ = env_logger::Builder::from_default_env()
        .target(env_logger::Target::Stdout)
        .filter_level(filter)
        .try_init();
}

/// Apply CLI verbosity flags to configuration.
/// Sets the file logging level to match the CLI verbosity:
/// - verbose=true → "debug" (more detail in log file)
/// - quiet=true   → "error" (suppress most log output)
/// - neither      → leave the config value unchanged
pub(super) fn apply_verbosity_to_config(cfg: &mut Config, verbose: bool, quiet: bool) {
    let Some(logging) = cfg.logging.as_mut() else {
        return;
    };
    if verbose {
        logging.level = "debug".to_string();
    } else if quiet {
        logging.level = "error".to_string();
    }
}

/// Apply CLI --input overrides to configuration.
/// Per D-05: CLI inputs completely replace config inputs when Some and non-empty.
/// Some(empty vec) keeps the config value and emits a warning.
pub(super) fn apply_cli_inputs_to_config(cfg: &mut Config, cli_inputs: Option<Vec<String>>) {
    if let Some(inputs) = cli_inputs {
        if inputs.is_empty() {
            log::warn!("--input provided but empty; using config inputs");
            return;
        }
        cfg.sqllog.inputs = inputs;
    }
}

pub(super) fn load_config(config_path: &str) -> Result<Config> {
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

/// Initialize logging once CLI verbosity has been applied.
pub(super) fn init_config_logging(cfg: &mut Config, verbose: bool, quiet: bool) -> Result<()> {
    apply_verbosity_to_config(cfg, verbose, quiet);
    if let Some(logging_config) = &cfg.logging {
        crate::logging::init_logging(logging_config, false)
    } else {
        init_simple_logging(quiet, verbose);
        Ok(())
    }
}

/// Format a runtime error with its severity and recovery hint.
pub(super) fn format_error_output(error: &Error) -> String {
    format_diagnostic(error, &error.severity().to_string())
}

/// Format a configuration validation failure.
pub(super) fn format_validate_error(error: &Error) -> String {
    format_diagnostic(error, "FAIL")
}

fn format_diagnostic(error: &Error, label: &str) -> String {
    let hint = error.suggestion();
    if hint.is_empty() {
        format!("[{label}] {error}")
    } else {
        format!("[{label}] {error}\n  hint: {hint}")
    }
}
