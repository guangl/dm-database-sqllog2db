use clap::{Parser, Subcommand};

/// SQL log exporter tool for DM database
#[derive(Debug, Parser)]
#[command(
    name = "sqllog2db",
    version,
    about = "Parse DM database SQL logs and export to CSV/SQLite",
    long_about = "A lightweight and efficient CLI tool for parsing DM database SQL logs (streaming) and exporting to CSV or SQLite."
)]
pub(crate) struct Cli {
    /// Verbose output (-v for debug, -vv for trace)
    #[arg(short = 'v', action = clap::ArgAction::Count, global = true)]
    pub(crate) verbose: u8,

    /// Suppress non-error output
    #[arg(short = 'q', global = true, conflicts_with = "verbose")]
    pub(crate) quiet: bool,

    #[command(subcommand)]
    pub(crate) command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    /// Run the log export task
    Run {
        /// Configuration file path
        #[arg(
            short = 'c',
            long = "config",
            default_value = "config.toml",
            env = "SQLLOG2DB_CONFIG"
        )]
        config: String,
    },
    /// Generate a default configuration file
    Init {
        /// Output configuration file path
        #[arg(short = 'o', long = "output", default_value = "config.toml")]
        output: String,
        /// Force overwrite if file exists
        #[arg(short = 'f', long = "force")]
        force: bool,
    },
    /// Validate a configuration file
    Validate {
        /// Configuration file path
        #[arg(
            short = 'c',
            long = "config",
            default_value = "config.toml",
            env = "SQLLOG2DB_CONFIG"
        )]
        config: String,
    },
}
