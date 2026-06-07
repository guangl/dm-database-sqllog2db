use crate::error::{ConfigError, Error, FileError, Result};
use log::{error, info, warn};
use std::fs;
use std::io::{BufRead, Write};
use std::path::Path;

/// 生成默认配置文件
pub fn handle_init(output_path: &str, force: bool) -> Result<()> {
    let path = Path::new(output_path);
    let content = CONFIG_TEMPLATE_EN;
    write_config_file(path, content, force)?;
    info!("Next steps:");
    info!("  1. Edit configuration file: {output_path}");
    info!("  2. Validate configuration: sqllog2db validate -c {output_path}");
    info!("  3. Run export: sqllog2db run -c {output_path}");
    Ok(())
}

fn write_config_file(path: &Path, content: &str, force: bool) -> Result<()> {
    let output_path = path.to_string_lossy();
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
    if let Some(parent) = path.parent().filter(|p| !p.exists()) {
        info!("Creating directory: {}", parent.display());
        fs::create_dir_all(parent).map_err(|e| {
            Error::File(FileError::CreateDirectoryFailed {
                path: parent.to_path_buf(),
                reason: e.to_string(),
            })
        })?;
    }
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
    Ok(())
}

// ── Wizard types ─────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Eq)]
pub enum ExporterChoice {
    Csv,
    Sqlite,
}

#[derive(Debug, PartialEq, Eq)]
pub struct WizardAnswers {
    pub inputs: String,
    pub exporter: ExporterChoice,
    pub csv_file: Option<String>,
    pub sqlite_db: Option<String>,
    pub sqlite_table: Option<String>,
}

fn prompt_line(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    prompt: &str,
    default: &str,
    buf: &mut String,
) -> Result<String> {
    write!(writer, "{prompt}")?;
    writer.flush()?;
    buf.clear();
    reader.read_line(buf)?;
    Ok(if buf.trim().is_empty() {
        default.to_owned()
    } else {
        buf.trim().to_owned()
    })
}

fn ask_exporter(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    buf: &mut String,
) -> Result<ExporterChoice> {
    write!(writer, "导出格式 (csv/sqlite) [default: csv]: ")?;
    writer.flush()?;
    let mut last_input = String::new();
    for _ in 0..3 {
        buf.clear();
        reader.read_line(buf)?;
        buf.trim().clone_into(&mut last_input);
        match last_input.as_str() {
            "" | "csv" => return Ok(ExporterChoice::Csv),
            "sqlite" => return Ok(ExporterChoice::Sqlite),
            _ => {
                write!(writer, "无效格式\"{last_input}\"，请输入 csv 或 sqlite: ")?;
                writer.flush()?;
            }
        }
    }
    Err(Error::Config(ConfigError::InvalidValue {
        field: "exporter".to_owned(),
        value: last_input,
        reason: "must be 'csv' or 'sqlite'".to_owned(),
    }))
}

fn build_csv_answers(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    inputs: String,
    buf: &mut String,
) -> Result<WizardAnswers> {
    let csv_file = prompt_line(
        reader,
        writer,
        "CSV 输出文件路径 [default: outputs/sqllog.csv]: ",
        "outputs/sqllog.csv",
        buf,
    )?;
    Ok(WizardAnswers {
        inputs,
        exporter: ExporterChoice::Csv,
        csv_file: Some(csv_file),
        sqlite_db: None,
        sqlite_table: None,
    })
}

fn build_sqlite_answers(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    inputs: String,
    buf: &mut String,
) -> Result<WizardAnswers> {
    let sqlite_db = prompt_line(
        reader,
        writer,
        "SQLite 数据库路径 [default: export/sqllog2db.db]: ",
        "export/sqllog2db.db",
        buf,
    )?;
    let sqlite_table = prompt_line(
        reader,
        writer,
        "表名（仅含字母/数字/下划线）[default: sqllog_records]: ",
        "sqllog_records",
        buf,
    )?;
    Ok(WizardAnswers {
        inputs,
        exporter: ExporterChoice::Sqlite,
        csv_file: None,
        sqlite_db: Some(sqlite_db),
        sqlite_table: Some(sqlite_table),
    })
}

pub fn run_wizard(reader: &mut impl BufRead, writer: &mut impl Write) -> Result<WizardAnswers> {
    let mut buf = String::new();
    let inputs = prompt_line(
        reader,
        writer,
        "SQL log 输入目录（可以是目录、文件或 glob 模式）[default: sqllogs]: ",
        "sqllogs",
        &mut buf,
    )?;
    let exporter = ask_exporter(reader, writer, &mut buf)?;
    match exporter {
        ExporterChoice::Csv => build_csv_answers(reader, writer, inputs, &mut buf),
        ExporterChoice::Sqlite => build_sqlite_answers(reader, writer, inputs, &mut buf),
    }
}

/// Escape a user string for embedding inside a TOML basic string (double-quoted).
/// In TOML basic strings, backslash and double-quote must be escaped.
/// Forward-slash normalization also handles Windows paths.
fn toml_escape(s: &str) -> String {
    s.replace('\\', "/").replace('"', "\\\"")
}

fn apply_csv_substitutions(content: &str, answers: &WizardAnswers) -> String {
    let csv_file = answers.csv_file.as_deref().unwrap_or("outputs/sqllog.csv");
    let escaped = toml_escape(csv_file);
    content.replace(
        r#"file = "outputs/sqllog.csv""#,
        &format!(r#"file = "{escaped}""#),
    )
}

fn apply_sqlite_substitutions(content: &str, answers: &WizardAnswers) -> String {
    let sqlite_db = answers
        .sqlite_db
        .as_deref()
        .unwrap_or("export/sqllog2db.db");
    let sqlite_table = answers.sqlite_table.as_deref().unwrap_or("sqllog_records");
    let escaped_db = toml_escape(sqlite_db);
    let escaped_table = toml_escape(sqlite_table);
    // The SQLite template already has the correct exporter sections configured.
    // Only substitute the user-provided values.
    content
        .replace(
            r#"database_url = "export/sqllog2db.db""#,
            &format!(r#"database_url = "{escaped_db}""#),
        )
        .replace(
            r#"table_name = "sqllog_records""#,
            &format!(r#"table_name = "{escaped_table}""#),
        )
}

fn apply_wizard_answers_to_template(answers: &WizardAnswers) -> String {
    let escaped_inputs = toml_escape(&answers.inputs);
    let template = match answers.exporter {
        ExporterChoice::Csv => CONFIG_TEMPLATE_EN,
        ExporterChoice::Sqlite => CONFIG_TEMPLATE_SQLITE_EN,
    };
    let content = template.replace(
        r#"inputs = ["sqllogs"]"#,
        &format!(r#"inputs = ["{escaped_inputs}"]"#),
    );
    match answers.exporter {
        ExporterChoice::Csv => apply_csv_substitutions(&content, answers),
        ExporterChoice::Sqlite => apply_sqlite_substitutions(&content, answers),
    }
}

/// 交互式配置向导入口
pub fn handle_init_interactive(output_path: &str, force: bool) -> Result<()> {
    // Early-exit check: do not run the wizard if the file already exists and --force is not set.
    let path = std::path::Path::new(output_path);
    if path.exists() && !force {
        error!("Configuration file already exists: {output_path}");
        info!("Tip: use --force to overwrite");
        return Err(Error::File(FileError::AlreadyExists {
            path: path.to_path_buf(),
        }));
    }
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    let answers = run_wizard(&mut reader, &mut writer)?;
    let content = apply_wizard_answers_to_template(&answers);
    write_config_file(path, &content, force)?;
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

/// `SQLite`-mode template: CSV section commented out, `SQLite` section active.
/// Used by the interactive wizard when the user selects `SQLite` output.
/// Placeholder values are substituted by `apply_sqlite_substitutions`.
const CONFIG_TEMPLATE_SQLITE_EN: &str = r#"# sqllog2db default configuration file (edit as needed)

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
# [exporter.csv]
# CSV output file path
# file = "outputs/sqllog.csv"
# Drop and recreate the file before writing (true/false)
# overwrite = true
# Append to existing CSV file instead of overwriting (true/false)
# append = false

# Option 2: SQLite database export
[exporter.sqlite]
# SQLite database file path
database_url = "export/sqllog2db.db"
# Table name to write records into (ASCII identifiers only: [A-Za-z_][A-Za-z0-9_]*)
table_name = "sqllog_records"
# Drop and recreate the table before writing (true/false)
overwrite = true
# Append rows to existing table instead of overwriting (true/false)
append = false
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wizard_all_defaults() {
        let input = b"\n\n\n";
        let mut reader = std::io::Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();
        let answers = run_wizard(&mut reader, &mut writer).unwrap();
        assert_eq!(answers.inputs, "sqllogs");
        assert!(matches!(answers.exporter, ExporterChoice::Csv));
        assert_eq!(answers.csv_file.as_deref(), Some("outputs/sqllog.csv"));
        assert!(answers.sqlite_db.is_none());
        assert!(answers.sqlite_table.is_none());
    }

    #[test]
    fn test_wizard_custom_csv_path() {
        let input = b"my/logs\ncsv\nmy_out/result.csv\n";
        let mut reader = std::io::Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();
        let answers = run_wizard(&mut reader, &mut writer).unwrap();
        assert_eq!(answers.inputs, "my/logs");
        assert!(matches!(answers.exporter, ExporterChoice::Csv));
        assert_eq!(answers.csv_file.as_deref(), Some("my_out/result.csv"));
    }

    #[test]
    fn test_wizard_sqlite_path() {
        let input = b"\nsqlite\nmy.db\nmy_table\n";
        let mut reader = std::io::Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();
        let answers = run_wizard(&mut reader, &mut writer).unwrap();
        assert!(matches!(answers.exporter, ExporterChoice::Sqlite));
        assert_eq!(answers.sqlite_db.as_deref(), Some("my.db"));
        assert_eq!(answers.sqlite_table.as_deref(), Some("my_table"));
        assert!(answers.csv_file.is_none());
    }

    #[test]
    fn test_wizard_sqlite_defaults() {
        let input = b"\nsqlite\n\n\n";
        let mut reader = std::io::Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();
        let answers = run_wizard(&mut reader, &mut writer).unwrap();
        assert!(matches!(answers.exporter, ExporterChoice::Sqlite));
        assert_eq!(answers.sqlite_db.as_deref(), Some("export/sqllog2db.db"));
        assert_eq!(answers.sqlite_table.as_deref(), Some("sqllog_records"));
    }

    #[test]
    fn test_wizard_invalid_format_three_times_returns_err() {
        let input = b"\nbad\nbad\nbad\n";
        let mut reader = std::io::Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();
        let result = run_wizard(&mut reader, &mut writer);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, Error::Config(ConfigError::InvalidValue { field, .. }) if field == "exporter"),
            "expected ConfigError::InvalidValue with field='exporter', got: {err:?}"
        );
    }

    #[test]
    fn test_wizard_writer_receives_prompts() {
        let input = b"\n\n\n";
        let mut reader = std::io::Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();
        run_wizard(&mut reader, &mut writer).unwrap();
        let output = String::from_utf8(writer).unwrap();
        assert!(
            output.contains("SQL log 输入目录"),
            "prompt should contain 'SQL log 输入目录'"
        );
        assert!(
            output.contains("导出格式 (csv/sqlite)"),
            "prompt should contain '导出格式 (csv/sqlite)'"
        );
        assert!(
            output.contains("CSV 输出文件路径"),
            "prompt should contain 'CSV 输出文件路径'"
        );
    }

    #[test]
    fn test_apply_csv_default() {
        let answers = WizardAnswers {
            inputs: "sqllogs".to_owned(),
            exporter: ExporterChoice::Csv,
            csv_file: Some("outputs/sqllog.csv".to_owned()),
            sqlite_db: None,
            sqlite_table: None,
        };
        let output = apply_wizard_answers_to_template(&answers);
        assert_eq!(
            output, CONFIG_TEMPLATE_EN,
            "default CSV path should produce identical output to template"
        );
    }

    #[test]
    fn test_apply_csv_custom() {
        let answers = WizardAnswers {
            inputs: "my/dir".to_owned(),
            exporter: ExporterChoice::Csv,
            csv_file: Some("out/r.csv".to_owned()),
            sqlite_db: None,
            sqlite_table: None,
        };
        let output = apply_wizard_answers_to_template(&answers);
        assert!(
            output.contains(r#"inputs = ["my/dir"]"#),
            "custom inputs should appear in output"
        );
        assert!(
            output.contains(r#"file = "out/r.csv""#),
            "custom csv path should appear in output"
        );
        assert!(
            output.contains("[exporter.csv]"),
            "[exporter.csv] section should be active (not commented)"
        );
        assert!(
            output.contains("# [exporter.sqlite]"),
            "[exporter.sqlite] section should remain commented"
        );
    }

    #[test]
    fn test_apply_sqlite() {
        let answers = WizardAnswers {
            inputs: "x".to_owned(),
            exporter: ExporterChoice::Sqlite,
            sqlite_db: Some("d.db".to_owned()),
            sqlite_table: Some("t".to_owned()),
            csv_file: None,
        };
        let output = apply_wizard_answers_to_template(&answers);
        assert!(
            output.contains("[exporter.sqlite]"),
            "[exporter.sqlite] should be activated"
        );
        assert!(
            !output.contains("# [exporter.sqlite]"),
            "[exporter.sqlite] should not be commented"
        );
        assert!(
            output.contains(r#"database_url = "d.db""#),
            "database_url should use user value"
        );
        assert!(
            output.contains(r#"table_name = "t""#),
            "table_name should use user value"
        );
        assert!(
            output.contains("# [exporter.csv]"),
            "[exporter.csv] should be commented out"
        );
        assert!(
            output.contains(r#"# file = "outputs/sqllog.csv""#),
            "csv file line should be commented out"
        );
    }

    #[test]
    fn test_apply_does_not_corrupt_logging_file() {
        let answers_csv = WizardAnswers {
            inputs: "sqllogs".to_owned(),
            exporter: ExporterChoice::Csv,
            csv_file: Some("outputs/sqllog.csv".to_owned()),
            sqlite_db: None,
            sqlite_table: None,
        };
        let output_csv = apply_wizard_answers_to_template(&answers_csv);
        assert!(
            output_csv.contains(r#"file = "logs/sqllog2db.log""#),
            "logging.file must not be corrupted in CSV mode"
        );

        let answers_sqlite = WizardAnswers {
            inputs: "sqllogs".to_owned(),
            exporter: ExporterChoice::Sqlite,
            sqlite_db: Some("export/sqllog2db.db".to_owned()),
            sqlite_table: Some("sqllog_records".to_owned()),
            csv_file: None,
        };
        let output_sqlite = apply_wizard_answers_to_template(&answers_sqlite);
        assert!(
            output_sqlite.contains(r#"file = "logs/sqllog2db.log""#),
            "logging.file must not be corrupted in SQLite mode"
        );
    }

    #[test]
    fn test_apply_output_parses_as_config_csv() {
        let answers = WizardAnswers {
            inputs: "sqllogs".to_owned(),
            exporter: ExporterChoice::Csv,
            csv_file: Some("outputs/sqllog.csv".to_owned()),
            sqlite_db: None,
            sqlite_table: None,
        };
        let content = apply_wizard_answers_to_template(&answers);
        let cfg: crate::config::Config =
            toml::from_str(&content).expect("CSV output should parse as valid TOML Config");
        cfg.validate()
            .expect("CSV output should pass Config::validate()");
    }

    #[test]
    fn test_apply_output_parses_as_config_sqlite() {
        let answers = WizardAnswers {
            inputs: "sqllogs".to_owned(),
            exporter: ExporterChoice::Sqlite,
            sqlite_db: Some("export/sqllog2db.db".to_owned()),
            sqlite_table: Some("sqllog_records".to_owned()),
            csv_file: None,
        };
        let content = apply_wizard_answers_to_template(&answers);
        let cfg: crate::config::Config =
            toml::from_str(&content).expect("SQLite output should parse as valid TOML Config");
        cfg.validate()
            .expect("SQLite output should pass Config::validate()");
    }
}
