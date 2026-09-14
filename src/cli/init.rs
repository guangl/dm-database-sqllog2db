//! Configuration creation: file handling, interactive questions and rendering.
use crate::error::{ConfigError, Error, FileError, Result};
use log::{error, info, warn};
use std::fs;
use std::path::Path;

/// 生成默认配置文件
///
/// # Errors
///
/// 目标文件已存在且未指定 `--force`、父目录创建失败或文件写入失败时返回错误。
pub fn handle_init(output_path: &str, force: bool) -> Result<()> {
    let path = Path::new(output_path);
    let content = build_parquet_template();
    write_config_file(path, &content, force)?;
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

/// 交互式配置向导入口
///
/// # Errors
///
/// 目标文件已存在且未指定 `--force`、向导交互 IO 失败或配置文件写入失败时返回错误。
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

#[cfg(test)]
#[path = "../../tests/unit/cli/init.rs"]
mod tests;

use crate::config::template::CONFIG_TEMPLATE_CSV;

const EXPORTER_MARKER: &str = "# ===================== 导出器配置 =====================";

fn build_parquet_template() -> String {
    let prefix = CONFIG_TEMPLATE_CSV
        .split_once(EXPORTER_MARKER)
        .map_or(CONFIG_TEMPLATE_CSV, |(prefix, _)| prefix);
    format!(
        "{prefix}{EXPORTER_MARKER}\n\
# 同一时刻只能启用一个导出器。优先级：parquet > csv\n\n\
# 方案 1：Parquet 导出（默认）\n\
[exporter.parquet]\n\
file = \"outputs/sqllog.parquet\"\n\
overwrite = true\n\
# zstd | snappy | uncompressed\n\
compression = \"zstd\"\n\
# 每个 row group 的最大记录数\n\
row_group_rows = 65536\n\n\
# 方案 2：CSV 导出\n\
# [exporter.csv]\n\
# CSV 输出文件路径\n\
# file = \"outputs/sqllog.csv\"\n\
# 写入前删除并重建文件（true/false）\n\
# overwrite = true\n\
# 追加到已有 CSV 文件而非覆盖（true/false）\n\
# append = false\n\n\
"
    )
}

fn apply_parquet_substitutions(content: &str, answers: &WizardAnswers) -> String {
    let file = answers
        .parquet_file
        .as_deref()
        .unwrap_or("outputs/sqllog.parquet");
    content.replace(
        r#"file = "outputs/sqllog.parquet""#,
        &format!(r#"file = "{}""#, toml_escape(file)),
    )
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

fn apply_wizard_answers_to_template(answers: &WizardAnswers) -> String {
    let escaped_inputs = toml_escape(&answers.inputs);
    let template = match answers.exporter {
        ExporterChoice::Parquet => {
            return apply_parquet_substitutions(
                &build_parquet_template().replace(
                    r#"inputs = ["sqllogs"]"#,
                    &format!(r#"inputs = ["{escaped_inputs}"]"#),
                ),
                answers,
            );
        }
        ExporterChoice::Csv => CONFIG_TEMPLATE_CSV,
    };
    let content = template.replace(
        r#"inputs = ["sqllogs"]"#,
        &format!(r#"inputs = ["{escaped_inputs}"]"#),
    );
    match answers.exporter {
        ExporterChoice::Parquet => unreachable!("handled above"),
        ExporterChoice::Csv => apply_csv_substitutions(&content, answers),
    }
}

use std::io::{BufRead, Write};

// ── Wizard types ─────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Eq)]
pub enum ExporterChoice {
    Parquet,
    Csv,
}

#[derive(Debug, PartialEq, Eq)]
pub struct WizardAnswers {
    pub inputs: String,
    pub exporter: ExporterChoice,
    pub parquet_file: Option<String>,
    pub csv_file: Option<String>,
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
    write!(writer, "导出格式 (parquet/csv) [default: parquet]: ")?;
    writer.flush()?;
    let mut last_input = String::new();
    for _ in 0..3 {
        buf.clear();
        reader.read_line(buf)?;
        buf.trim().clone_into(&mut last_input);
        match last_input.as_str() {
            "" | "parquet" => return Ok(ExporterChoice::Parquet),
            "csv" => return Ok(ExporterChoice::Csv),
            _ => {
                write!(writer, "无效格式\"{last_input}\"，请输入 parquet 或 csv: ")?;
                writer.flush()?;
            }
        }
    }
    Err(Error::Config(ConfigError::InvalidValue {
        field: "exporter".to_owned(),
        value: last_input,
        reason: "must be 'parquet' or 'csv'".to_owned(),
    }))
}

fn build_parquet_answers(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    inputs: String,
    buf: &mut String,
) -> Result<WizardAnswers> {
    let parquet_file = prompt_line(
        reader,
        writer,
        "Parquet 输出文件路径 [default: outputs/sqllog.parquet]: ",
        "outputs/sqllog.parquet",
        buf,
    )?;
    Ok(WizardAnswers {
        inputs,
        exporter: ExporterChoice::Parquet,
        parquet_file: Some(parquet_file),
        csv_file: None,
    })
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
        parquet_file: None,
        csv_file: Some(csv_file),
    })
}

/// 运行交互式向导，逐项读取用户输入并返回答案集合。
///
/// # Errors
///
/// 从 `reader` 读取或向 `writer` 写入提示失败（IO 错误）时返回错误。
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
        ExporterChoice::Parquet => build_parquet_answers(reader, writer, inputs, &mut buf),
        ExporterChoice::Csv => build_csv_answers(reader, writer, inputs, &mut buf),
    }
}
