//! CLI arguments, command dispatch, configuration setup and presentation.
mod runtime;

pub mod completion;
pub mod init;
pub mod opts;
pub mod stats;
pub mod validate;

use crate::cli::{
    self as cli,
    runtime::{
        apply_cli_inputs_to_config, format_error_output, format_validate_error,
        init_config_logging, init_simple_logging, load_config,
    },
};
use crate::error::{Error, ErrorStats, Result};
use crate::{config::Config, engine, preflight};
use clap::{Command, parser::ValueSource};
use log::info;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const EXIT_PARTIAL: i32 = 1;
const EXIT_FATAL: i32 = 2;
const EXIT_INTERRUPTED: i32 = 130;

/// Configuration file used when the CLI runs on its own and neither
/// `-c/--config` nor `SQLLOG2DB_CONFIG` provides a path.
const DEFAULT_CONFIG_FILE: &str = "config.toml";

/// Where the current launch takes its default configuration path from.
#[derive(Debug, Clone)]
struct LaunchDefaults {
    /// Path used when neither `-c/--config` nor `SQLLOG2DB_CONFIG` is provided.
    config: PathBuf,
    /// Seed the default path with the default template when it does not exist yet.
    create_missing_config: bool,
}

impl Default for LaunchDefaults {
    fn default() -> Self {
        Self {
            config: PathBuf::from(DEFAULT_CONFIG_FILE),
            create_missing_config: false,
        }
    }
}

impl LaunchDefaults {
    /// Defaults for the dameng-cli plugin entry: the host owns the plugin's
    /// configuration directory and passes it as `DM_PLUGIN_CONFIG_DIR`, so the
    /// plugin's own `config.toml` inside it is used, and created on first use.
    fn for_plugin() -> Self {
        match std::env::var_os("DM_PLUGIN_CONFIG_DIR") {
            Some(dir) => Self {
                config: PathBuf::from(dir).join(dm_plugin_sdk::CONFIG_FILE),
                create_missing_config: true,
            },
            None => Self::default(),
        }
    }
}

enum CommandOutcome {
    Finished(i32),
    Exported(ErrorStats, bool),
}

/// Execute the CLI and return its process exit code.
#[must_use]
pub fn run() -> i32 {
    run_cli(&LaunchDefaults::default(), None)
}

/// Execute the CLI through `dameng-cli` and show the host command in help output.
///
/// Configuration defaults to the plugin's own file,
/// `$DM_PLUGIN_CONFIG_DIR/config.toml`, which is created with default
/// contents on first use.
#[must_use]
pub fn run_as_plugin() -> i32 {
    run_cli(&LaunchDefaults::for_plugin(), Some("dm sqllog2db"))
}

fn run_cli(defaults: &LaunchDefaults, bin_name: Option<&'static str>) -> i32 {
    match dispatch(defaults, bin_name) {
        Ok(CommandOutcome::Exported(stats, quiet)) => {
            if stats.has_fatal() {
                return EXIT_FATAL;
            }
            if stats.has_errors() {
                if !quiet {
                    eprintln!(
                        "Completed with {} error(s) ({} parse, {} export).",
                        stats.total_errors, stats.parse_errors, stats.export_errors
                    );
                }
                return EXIT_PARTIAL;
            }
            // EXIT_CLEAN (0) is default
        }
        Ok(CommandOutcome::Finished(code)) => return code,
        Err(e) => {
            if matches!(e, Error::Interrupted) {
                return EXIT_INTERRUPTED;
            }
            eprintln!("{}", format_error_output(&e));
            return EXIT_FATAL;
        }
    }
    0
}

/// Point every `-c/--config` (and `init -o/--output`) option at this
/// launch's default configuration path, so `--help` reports the real default.
fn apply_default_config_path(cmd: Command, config: &Path) -> Command {
    let value = config.to_string_lossy().into_owned();
    let with_config = |subcommand: Command| {
        let value = value.clone();
        subcommand.mut_arg("config", move |arg| arg.default_value(value))
    };
    cmd.mut_subcommand("run", with_config)
        .mut_subcommand("validate", with_config)
        .mut_subcommand("stats", with_config)
        .mut_subcommand("init", |subcommand| {
            subcommand.mut_arg("output", |arg| arg.default_value(value))
        })
}

/// Create the launch default configuration file when it is missing.
///
/// Only plugin launches seed a file: the host hands the plugin a private
/// configuration directory, while a standalone run must not drop a
/// `config.toml` into the caller's working directory. An explicit `-c` or
/// `SQLLOG2DB_CONFIG` path never triggers creation.
///
/// Returns whether a new file was written.
fn create_default_config_if_needed(
    path: &str,
    defaults: &LaunchDefaults,
    source: Option<ValueSource>,
) -> Result<bool> {
    if source != Some(ValueSource::DefaultValue)
        || !defaults.create_missing_config
        || Path::new(path) != defaults.config.as_path()
    {
        return Ok(false);
    }
    cli::init::ensure_config_file(Path::new(path))
}

fn dispatch(defaults: &LaunchDefaults, bin_name: Option<&'static str>) -> Result<CommandOutcome> {
    use clap::{CommandFactory, FromArgMatches};

    let mut cmd = cli::opts::Cli::command();
    if let Some(bin_name) = bin_name {
        cmd = cmd.bin_name(bin_name);
    }
    let cmd = apply_default_config_path(cmd, &defaults.config);
    let matches = cmd.get_matches();
    let cli = cli::opts::Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());
    let config_source = matches.subcommand().and_then(|(name, args)| {
        (name != "init")
            .then(|| args.value_source("config"))
            .flatten()
    });

    let needs_simple_logging = !matches!(
        &cli.command,
        Some(cli::opts::Commands::Run { .. } | cli::opts::Commands::Stats { .. })
    );
    if needs_simple_logging {
        init_simple_logging(cli.quiet, cli.verbose);
    }

    match &cli.command {
        Some(cli::opts::Commands::Init {
            output,
            force,
            interactive,
        }) => {
            if *interactive {
                cli::init::handle_init_interactive(output, *force)?;
            } else {
                cli::init::handle_init(output, *force)?;
            }
            Ok(CommandOutcome::Finished(0))
        }
        Some(cli::opts::Commands::Run { config, input }) => {
            let created_default = create_default_config_if_needed(config, defaults, config_source)?;
            let mut cfg = load_config(config)?;
            apply_cli_inputs_to_config(&mut cfg, input.clone());
            cfg.validate()?;

            init_config_logging(&mut cfg, cli.verbose, cli.quiet)?;
            if created_default {
                info!("Created default configuration file: {config}");
            }
            info!("Application started");
            info!("Configuration validation passed");

            let pf = preflight::check(&cfg);
            if pf.print_and_check() {
                return Ok(CommandOutcome::Finished(EXIT_FATAL));
            }

            let interrupted = Arc::new(AtomicBool::new(false));
            let interrupted_flag = Arc::clone(&interrupted);
            ctrlc::set_handler(move || {
                interrupted_flag.store(true, Ordering::Release);
            })
            .ok();

            let stats = engine::run(&cfg, cli.quiet, cli.verbose, &interrupted)?;
            Ok(CommandOutcome::Exported(stats, cli.quiet))
        }
        Some(cli::opts::Commands::Validate { config }) => {
            let created_default = create_default_config_if_needed(config, defaults, config_source)?;
            let cfg = Config::from_file(Path::new(config))?;
            if created_default {
                info!("Created default configuration file: {config}");
            }
            if let Err(e) = cfg.validate() {
                eprintln!("{}", format_validate_error(&e));
                return Ok(CommandOutcome::Finished(EXIT_FATAL));
            }
            cli::validate::handle_validate(&cfg);
            Ok(CommandOutcome::Finished(0))
        }
        Some(cli::opts::Commands::Stats {
            config,
            top,
            from,
            to,
        }) => {
            let created_default = create_default_config_if_needed(config, defaults, config_source)?;
            let mut cfg = Config::from_file(Path::new(config))?;
            cfg.validate_for_stats()?;
            init_config_logging(&mut cfg, cli.verbose, cli.quiet)?;
            if created_default {
                info!("Created default configuration file: {config}");
            }
            cli::stats::handle_stats(&cfg, *top, from.clone(), to.clone())?;
            Ok(CommandOutcome::Finished(0))
        }
        None => {
            cli::opts::Cli::command().print_help().ok();
            Ok(CommandOutcome::Finished(0))
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/cli/mod.rs"]
mod tests;
