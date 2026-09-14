//! CLI arguments, command dispatch, configuration setup and presentation.
mod runtime;

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
use log::info;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const EXIT_PARTIAL: i32 = 1;
const EXIT_FATAL: i32 = 2;
const EXIT_INTERRUPTED: i32 = 130;

enum CommandOutcome {
    Finished(i32),
    Exported(ErrorStats, bool),
}

/// Execute the CLI and return its process exit code.
#[must_use]
pub fn run() -> i32 {
    match dispatch() {
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

fn dispatch() -> Result<CommandOutcome> {
    use clap::{CommandFactory, FromArgMatches};

    let cmd = cli::opts::Cli::command();
    let matches = cmd.get_matches();
    let cli = cli::opts::Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

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
            let mut cfg = load_config(config)?;
            apply_cli_inputs_to_config(&mut cfg, input.clone());
            cfg.validate()?;

            init_config_logging(&mut cfg, cli.verbose, cli.quiet)?;
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
            let cfg = Config::from_file(Path::new(config))?;
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
            let mut cfg = Config::from_file(Path::new(config))?;
            cfg.validate_for_stats()?;
            init_config_logging(&mut cfg, cli.verbose, cli.quiet)?;
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
