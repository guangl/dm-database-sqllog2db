use super::*;
use crate::config::template::CONFIG_TEMPLATE_CSV;
use crate::error::ConfigError;

#[test]
fn test_wizard_all_defaults() {
    let input = b"\n\n\n";
    let mut reader = std::io::Cursor::new(input.as_ref());
    let mut writer = Vec::<u8>::new();
    let answers = run_wizard(&mut reader, &mut writer).unwrap();
    assert_eq!(answers.inputs, "sqllogs");
    assert!(matches!(answers.exporter, ExporterChoice::Parquet));
    assert_eq!(
        answers.parquet_file.as_deref(),
        Some("outputs/sqllog.parquet")
    );
    assert!(answers.csv_file.is_none());
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
        output.contains("导出格式 (parquet/csv)"),
        "prompt should contain all exporter choices"
    );
    assert!(
        output.contains("Parquet 输出文件路径"),
        "default wizard should prompt for a Parquet path"
    );
}

#[test]
fn test_apply_csv_default() {
    let answers = WizardAnswers {
        inputs: "sqllogs".to_owned(),
        exporter: ExporterChoice::Csv,
        parquet_file: None,
        csv_file: Some("outputs/sqllog.csv".to_owned()),
    };
    let output = apply_wizard_answers_to_template(&answers);
    assert_eq!(
        output, CONFIG_TEMPLATE_CSV,
        "default CSV path should produce identical output to template"
    );
}

#[test]
fn test_apply_csv_custom() {
    let answers = WizardAnswers {
        inputs: "my/dir".to_owned(),
        exporter: ExporterChoice::Csv,
        parquet_file: None,
        csv_file: Some("out/r.csv".to_owned()),
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
        !output.contains("sqlite"),
        "template should only contain file exporters"
    );
}

#[test]
fn test_apply_does_not_corrupt_logging_file() {
    let answers_csv = WizardAnswers {
        inputs: "sqllogs".to_owned(),
        exporter: ExporterChoice::Csv,
        parquet_file: None,
        csv_file: Some("outputs/sqllog.csv".to_owned()),
    };
    let output_csv = apply_wizard_answers_to_template(&answers_csv);
    assert!(
        output_csv.contains(r#"file = "logs/sqllog2db.log""#),
        "logging.file must not be corrupted in CSV mode"
    );
}

#[test]
fn test_apply_output_parses_as_config_csv() {
    let answers = WizardAnswers {
        inputs: "sqllogs".to_owned(),
        exporter: ExporterChoice::Csv,
        parquet_file: None,
        csv_file: Some("outputs/sqllog.csv".to_owned()),
    };
    let content = apply_wizard_answers_to_template(&answers);
    let cfg: crate::config::Config =
        toml::from_str(&content).expect("CSV output should parse as valid TOML Config");
    cfg.validate()
        .expect("CSV output should pass Config::validate()");
}

#[test]
fn wizard_rejects_removed_exporter() {
    let mut reader = std::io::Cursor::new(b"\nsqlite\nsqlite\nsqlite\n");
    let mut writer = Vec::new();
    let error = run_wizard(&mut reader, &mut writer).unwrap_err();
    assert!(error.to_string().contains("must be 'parquet' or 'csv'"));
    assert!(!build_parquet_template().contains("sqlite"));
    assert!(!CONFIG_TEMPLATE_CSV.contains("sqlite"));
}
