use clap::{Parser, Subcommand};

/// SQL log exporter tool for DM database
#[derive(Debug, Parser)]
#[command(
    name = "sqllog2db",
    version,
    about = "Parse DM database SQL logs and export to CSV/SQLite",
    long_about = "A lightweight and efficient CLI tool for parsing DM database SQL logs (streaming) and exporting to CSV or SQLite.",
    after_help = "\
EXAMPLES:
    Export all records from SQL log files:
        sqllog2db run -c config.toml

    Export with SQL indicators filter configured:
        sqllog2db run -c config.toml

    Generate a default configuration file:
        sqllog2db init -o config.toml

    Validate a configuration file:
        sqllog2db validate -c config.toml"
)]
pub(crate) struct Cli {
    /// Show per-file processing details
    #[arg(
        short = 'v',
        long = "verbose",
        global = true,
        conflicts_with = "quiet",
        help = "Show per-file processing details on stderr."
    )]
    pub(crate) verbose: bool,

    /// Suppress non-error output
    #[arg(short = 'q', long = "quiet", global = true, conflicts_with = "verbose")]
    pub(crate) quiet: bool,

    #[command(subcommand)]
    pub(crate) command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    /// Run the log export task
    #[command(
        long_about = "Run the log export task. Parses DM database SQL log files and exports them to CSV or SQLite based on the configuration file.",
        after_help = "\
EXAMPLES:
    Export using a custom configuration path:
        sqllog2db run -c /path/to/config.toml

    // TODO(Phase 37): replace with actual stdin pipe example
    Pipe log data via stdin (requires --input flag):
        sqllog2db run -c config.toml --input -

Configuration file sections: [csv] / [sqlite] for output, [pipeline] for filters (include, exclude, indicators, sql)."
    )]
    Run {
        /// TOML configuration file path
        #[arg(
            short = 'c',
            long = "config",
            default_value = "config.toml",
            env = "SQLLOG2DB_CONFIG",
            help = "TOML configuration file path. See [csv], [sqlite], [pipeline] sections."
        )]
        config: String,
    },
    /// Generate a default configuration file
    #[command(
        long_about = "Generate a default TOML configuration file with all available options and their default values.",
        after_help = "\
EXAMPLES:
    Generate a config file, overwriting if it already exists:
        sqllog2db init -o myconfig.toml --force"
    )]
    Init {
        /// Path for the generated default configuration file
        #[arg(
            short = 'o',
            long = "output",
            default_value = "config.toml",
            help = "Path for the generated default configuration file."
        )]
        output: String,
        /// Force overwrite if file exists
        #[arg(short = 'f', long = "force")]
        force: bool,
    },
    /// Validate a configuration file
    #[command(
        long_about = "Validate a TOML configuration file for correctness. Checks that all required sections and fields are present and valid.",
        after_help = "\
EXAMPLES:
    Validate a configuration file:
        sqllog2db validate -c config.toml"
    )]
    Validate {
        /// TOML configuration file path to validate
        #[arg(
            short = 'c',
            long = "config",
            default_value = "config.toml",
            env = "SQLLOG2DB_CONFIG",
            help = "TOML configuration file path to validate."
        )]
        config: String,
    },
}
