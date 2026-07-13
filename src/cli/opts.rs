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

    Override input paths from the command line:
        sqllog2db run -c config.toml --input 'sqllogs/2025-*.log'

    Generate a default configuration file:
        sqllog2db init -o config.toml

    Validate a configuration file:
        sqllog2db validate -c config.toml"
)]
pub struct Cli {
    /// Show per-file processing details
    #[arg(
        short = 'v',
        long = "verbose",
        global = true,
        conflicts_with = "quiet",
        help = "Show per-file processing details on stderr."
    )]
    pub verbose: bool,

    /// Suppress non-error output
    #[arg(short = 'q', long = "quiet", global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Run the log export task
    #[command(
        long_about = "Run the log export task. Parses DM database SQL log files and exports them to CSV or SQLite based on the configuration file.",
        after_help = "\
EXAMPLES:
    Export using a custom configuration path:
        sqllog2db run -c /path/to/config.toml

    Pipe log data via stdin:
        cat sqllogs/2025-01-15.log | sqllog2db run -c config.toml

    Override input paths from CLI:
        sqllog2db run -c config.toml --input 'sqllogs/*.log' --input archive.log

Configuration file sections: [csv] / [sqlite] for output, [filter] for filters (include, exclude, indicators, sql)."
    )]
    Run {
        /// TOML configuration file path
        #[arg(
            short = 'c',
            long = "config",
            default_value = "config.toml",
            env = "SQLLOG2DB_CONFIG",
            help = "TOML configuration file path. See [csv], [sqlite], [filter] sections."
        )]
        config: String,
        /// Input log paths. Repeat for multiple entries. Overrides config \[sqllog\].inputs.
        #[arg(
            short = 'i',
            long = "input",
            action = clap::ArgAction::Append,
            help = "Input log paths. Repeat for multiple entries. Overrides config [sqllog].inputs."
        )]
        input: Option<Vec<String>>,
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
        /// Start interactive configuration wizard
        #[arg(short = 'i', long = "interactive")]
        interactive: bool,
    },
    /// Validate a configuration file
    #[command(
        long_about = "Validate a TOML configuration file for correctness. Checks that all required sections and fields are present and valid.",
        after_help = "\
EXAMPLES:
    Validate a configuration file:
        sqllog2db validate -c config.toml

    Validate in quiet mode (suppress non-error output):
        sqllog2db validate -c config.toml --quiet"
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
    /// Run statistical analysis on SQL log files
    #[command(
        long_about = "Analyse DM database SQL log files and report slow SQL and high-frequency SQL patterns.",
        after_help = "\
EXAMPLES:
    Run statistics with default top-20:
        sqllog2db stats -c config.toml

    Limit output to top 5 per table:
        sqllog2db stats -c config.toml --top 5

    Filter records by time range:
        sqllog2db stats -c config.toml --from \"2024-01-01\" --to \"2024-01-31\""
    )]
    Stats {
        /// TOML configuration file path
        #[arg(
            short = 'c',
            long = "config",
            default_value = "config.toml",
            env = "SQLLOG2DB_CONFIG",
            help = "TOML configuration file path. See [csv], [sqlite], [sqllog] sections."
        )]
        config: String,
        /// Number of top records to display per table (default: 20)
        #[arg(
            long = "top",
            value_parser = clap::value_parser!(u32).range(1..),
            help = "Number of top records per table. Must be >= 1 (default: 20)."
        )]
        top: Option<u32>,
        /// Start of time range (overrides config `[stats].from`)
        #[arg(
            long = "from",
            value_name = "DATETIME",
            help = "Start of time range. Formats: \"YYYY-MM-DD\" or \"YYYY-MM-DD HH:MM:SS\". Overrides config [stats].from."
        )]
        from: Option<String>,
        /// End of time range (overrides config `[stats].to`)
        #[arg(
            long = "to",
            value_name = "DATETIME",
            help = "End of time range. Formats: \"YYYY-MM-DD\" or \"YYYY-MM-DD HH:MM:SS\". Overrides config [stats].to."
        )]
        to: Option<String>,
    },
    // watch 子命令暂时下线：当前只支持追加写出到文件，功能价值有限。
    // 领域实现（src/watch/）与测试保留编译；恢复时取消下方注释并还原 main.rs 的
    // `mod watch;` 与 Watch 分支即可。
    //
    // /// Watch directory for new .log files and process them automatically
    // #[command(
    //     long_about = "Watch configured input directories for new .log files. Automatically triggers processing when new files appear. Press Ctrl+C to stop.",
    //     after_help = "\
    // EXAMPLES:
    //     Watch and process new log files automatically:
    //         sqllog2db watch -c config.toml
    //
    //     Watch in quiet mode (suitable for cron/background):
    //         sqllog2db watch -c config.toml --quiet"
    // )]
    // Watch {
    //     /// TOML configuration file path
    //     #[arg(
    //         short = 'c',
    //         long = "config",
    //         default_value = "config.toml",
    //         env = "SQLLOG2DB_CONFIG",
    //         help = "TOML configuration file path."
    //     )]
    //     config: String,
    // },
}
