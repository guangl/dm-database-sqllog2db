use crate::error::{Error, FileError, Result};
use log::{debug, error, info, warn};
use std::fs;
use std::path::Path;

/// 生成默认配置文件
pub fn handle_init(output_path: &str, force: bool) -> Result<()> {
    let path = Path::new(output_path);

    info!("Preparing to generate configuration file: {output_path}");

    let file_existed = path.exists();

    if file_existed && !force {
        error!("Configuration file already exists: {output_path}");
        info!("Tip: use --force to overwrite");
        return Err(Error::File(FileError::AlreadyExists {
            path: path.to_path_buf(),
        }));
    }

    if file_existed && force {
        warn!("Will overwrite existing configuration file");
    }

    debug!("Generating default configuration content...");
    let content = CONFIG_TEMPLATE_EN;

    if let Some(parent) = path.parent().filter(|p| !p.exists()) {
        info!("Creating directory: {}", parent.display());
        fs::create_dir_all(parent).map_err(|e| {
            Error::File(FileError::CreateDirectoryFailed {
                path: parent.to_path_buf(),
                reason: e.to_string(),
            })
        })?;
    }

    debug!("Writing configuration file...");
    fs::write(path, content).map_err(|e| {
        Error::File(FileError::WriteFailed {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })
    })?;

    if file_existed {
        info!("Configuration file overwritten: {output_path}");
    } else {
        info!("Configuration file generated: {output_path}");
    }

    info!("Next steps:");
    info!("  1. Edit configuration file: {output_path}");
    info!("  2. Validate configuration: sqllog2db validate -c {output_path}");
    info!("  3. Run export: sqllog2db run -c {output_path}");

    Ok(())
}

// ── Templates ────────────────────────────────────────────────────────────────

const CONFIG_TEMPLATE_EN: &str = r#"# sqllog2db default configuration file (edit as needed)

[sqllog]
# SQL log path list: directories, single files, or glob patterns (e.g. "./logs/2025-*.log")
# Multiple entries are supported.
inputs = ["sqllogs"]

[logging]
# Application log file path
file = "logs/sqllog2db.log"
# Log level: trace | debug | info | warn | error
level = "info"
# Log retention in days (1-365)
retention_days = 7

[replace_parameters]
# Write a normalized_sql column in export output (default: true).
# For INS/DEL/UPD/ORA records, parameter values are substituted into SQL placeholders.
enable = true

[filter]
# Enable the filter pipeline
enable = false

# --- Include filters (record-level, AND semantics: every configured field must match) ---
# Metadata fields use exact string matching.
[filter.include]
# users      = ["SYSDBA"]                       # Exact-match list of usernames to include
# ips        = ["127.0.0.1", "192.168.1.100"]   # Exact-match list of client IP addresses to include
# sessions   = ["0x7f41435437a8"]               # Exact-match list of session IDs (hex strings) to include
# threads    = ["2188515"]                       # Exact-match list of thread IDs to include
# statements = ["INS", "UPD", "DEL"]            # Statement types to include (INS/UPD/DEL/SEL/SET/OTH/ORA)
# apps       = ["DMSQL"]                        # Exact-match list of application names to include
# tags       = ["[SEL]"]                        # Exact-match list of record tags to include (e.g. [SEL], [INS])
# start_ts   = "2023-01-01 00:00:00"            # Inclusive lower bound of record timestamp (format: YYYY-MM-DD HH:MM:SS)
# end_ts     = "2023-01-01 23:59:59"            # Inclusive upper bound of record timestamp (format: YYYY-MM-DD HH:MM:SS)
# trxids     = ["257809109", "257809110"]        # Exact-match list of transaction IDs to include

# --- Exclude filters (record-level, OR-veto: any match drops the record) ---
# Metadata fields use exact string matching.
[filter.exclude]
# users      = ["guest", "anon"]                # Exact-match list of usernames to exclude
# ips        = ["10.0.0.1", "172.16.0.1"]       # Exact-match list of client IP addresses to exclude
# sessions   = ["0x0000000000000000"]           # Exact-match list of session IDs (hex strings) to exclude
# threads    = ["0"]                            # Exact-match list of thread IDs to exclude
# statements = ["SEL", "SET"]                   # Statement types to exclude (INS/UPD/DEL/SEL/SET/OTH/ORA)
# apps       = ["monitor", "health"]            # Exact-match list of application names to exclude
# tags       = ["[SET]", "[OTH]"]              # Exact-match list of record tags to exclude

# --- Indicator filters (transaction-level: match retains the whole transaction; requires pre-scan) ---
[filter.indicators]
# exec_ids = [257809109, 257809110]   # Transaction-level: retain whole transaction if any record's exec_id matches
# min_runtime_ms = 1000               # Transaction-level: retain whole transaction if any statement's runtime (ms) >= threshold
# min_row_count = 100                 # Transaction-level: retain whole transaction if any statement's row_count >= threshold

# --- SQL filters (transaction-level: match retains the whole transaction; requires pre-scan) ---
[filter.sql]
# includes = ["FROM USER_TABLES", "DELETE FROM"]   # Transaction-level: retain whole transaction if any SQL text contains any substring listed
# excludes = ["SELECT 1", "DUAL"]                  # Transaction-level: drop whole transaction if any SQL text contains any substring listed

# --- Stats subcommand time-range filter (optional) ---
[stats]
# from = "2024-01-01"   # Start of time range. Formats: "YYYY-MM-DD" or "YYYY-MM-DD HH:MM:SS"
# to   = "2024-01-31"   # End of time range. Same formats as from.
# top  = 20             # Default top-N. CLI --top overrides this value.
# CLI args --from / --to / --top override the values above. When both CLI and config are absent, stats runs without time filtering (top defaults to 20).

# ===================== Exporter Configuration =====================
# Only one exporter can be active at a time. Priority: csv > sqlite

# Option 1: CSV export (default)
[exporter.csv]
# CSV output file path
file = "outputs/sqllog.csv"
# Drop and recreate the file before writing (true/false)
overwrite = true
# Append to existing CSV file instead of overwriting (true/false)
append = false

# Option 2: SQLite database export
# [exporter.sqlite]
# SQLite database file path
# database_url = "export/sqllog2db.db"
# Table name to write records into (ASCII identifiers only: [A-Za-z_][A-Za-z0-9_]*)
# table_name = "sqllog_records"
# Drop and recreate the table before writing (true/false)
# overwrite = true
# Append rows to existing table instead of overwriting (true/false)
# append = false
"#;
