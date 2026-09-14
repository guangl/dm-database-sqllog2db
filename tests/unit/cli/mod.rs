use super::*;
use crate::cli::runtime::apply_verbosity_to_config;
use crate::error::{ConfigError, ExportError, ParserError};

#[test]
fn test_exit_code_clean() {
    let stats = ErrorStats::default();
    assert!(!stats.has_errors());
    assert!(!stats.has_fatal());
}

#[test]
fn test_exit_code_partial_errors() {
    let mut stats = ErrorStats::default();
    stats.add_parse_error();
    assert!(stats.has_errors());
    assert!(!stats.has_fatal());
}

#[test]
fn test_exit_code_fatal_error() {
    let mut stats = ErrorStats::default();
    stats.set_fatal("test fatal".into());
    assert!(stats.has_fatal());
}

#[test]
fn test_error_is_fatal_for_config() {
    let e = Error::Config(ConfigError::NoExporters);
    assert!(e.is_fatal());
    assert_eq!(e.severity(), crate::error::ErrorSeverity::Critical);
}

#[test]
fn test_error_is_fatal_for_parse_error() {
    let e = Error::Parser(ParserError::PathNotFound {
        path: "/tmp".into(),
    });
    assert!(!e.is_fatal());
    assert_eq!(e.severity(), crate::error::ErrorSeverity::Warning);
}

#[test]
fn test_error_suggestion_for_config_not_found() {
    let e = Error::Config(ConfigError::NotFound("/tmp/config.toml".into()));
    assert!(e.suggestion().contains("sqllog2db init"));
}

#[test]
fn test_error_suggestion_for_config_parse_failed() {
    let e = Error::Config(ConfigError::ParseFailed {
        path: "/tmp/bad.toml".into(),
        reason: "unexpected EOF".into(),
    });
    let s = e.suggestion();
    assert!(
        !s.is_empty(),
        "ParseFailed should have a non-empty suggestion, got empty"
    );
    assert!(
        s.contains("TOML") || s.contains("syntax"),
        "ParseFailed suggestion should mention TOML syntax; got: {s}"
    );
}

#[test]
fn test_error_suggestion_for_export_write_failed() {
    let e = Error::Export(ExportError::WriteFailed {
        path: "/tmp/out.csv".into(),
        reason: "disk full".into(),
    });
    assert!(!e.is_fatal());
    assert!(!e.suggestion().is_empty());
}

#[test]
fn verbosity_does_not_enable_file_logging() {
    let mut cfg = Config::default();
    apply_verbosity_to_config(&mut cfg, true, false);
    assert!(cfg.logging.is_none());
}

#[test]
fn test_apply_verbosity_quiet() {
    let mut cfg = Config {
        logging: Some(crate::config::LoggingConfig::default()),
        ..Config::default()
    };
    apply_verbosity_to_config(&mut cfg, false, true);
    assert_eq!(cfg.logging.as_ref().unwrap().level, "error");
}

#[test]
fn test_apply_verbosity_not_quiet() {
    let mut cfg = Config {
        logging: Some(crate::config::LoggingConfig::default()),
        ..Config::default()
    };
    let original = cfg.logging.as_ref().unwrap().level.clone();
    apply_verbosity_to_config(&mut cfg, false, false);
    assert_eq!(cfg.logging.as_ref().unwrap().level, original);
}

#[test]
fn test_apply_verbosity_verbose_sets_debug() {
    let mut cfg = Config {
        logging: Some(crate::config::LoggingConfig::default()),
        ..Config::default()
    };
    apply_verbosity_to_config(&mut cfg, true, false);
    assert_eq!(cfg.logging.as_ref().unwrap().level, "debug");
}

#[test]
fn test_load_config_not_found_returns_default() {
    let result = load_config("/nonexistent/path/config.toml");
    assert!(result.is_ok());
}

#[test]
fn test_load_config_invalid_toml_returns_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("bad.toml");
    std::fs::write(&path, "not valid toml ][[[").unwrap();
    let result = load_config(path.to_str().unwrap());
    assert!(result.is_err());
}

#[test]
fn test_apply_cli_inputs_none_keeps_config() {
    let mut cfg = Config::default();
    cfg.sqllog.inputs = vec!["sqllogs".to_string()];
    apply_cli_inputs_to_config(&mut cfg, None);
    assert_eq!(
        cfg.sqllog.inputs,
        vec!["sqllogs".to_string()],
        "None should not change config inputs"
    );
}

#[test]
fn test_apply_cli_inputs_some_replaces() {
    let mut cfg = Config::default();
    cfg.sqllog.inputs = vec!["a".to_string()];
    apply_cli_inputs_to_config(&mut cfg, Some(vec!["b".to_string(), "c".to_string()]));
    assert_eq!(
        cfg.sqllog.inputs,
        vec!["b".to_string(), "c".to_string()],
        "Some(non-empty) should completely replace config inputs"
    );
}

#[test]
fn test_apply_cli_inputs_empty_vec_keeps_config() {
    let mut cfg = Config::default();
    cfg.sqllog.inputs = vec!["x".to_string()];
    apply_cli_inputs_to_config(&mut cfg, Some(vec![]));
    assert_eq!(
        cfg.sqllog.inputs,
        vec!["x".to_string()],
        "Some(empty vec) should not change config inputs"
    );
}

#[test]
fn test_error_io_suggestion_non_empty() {
    let e = Error::Io(std::io::Error::other("disk full"));
    let suggestion = e.suggestion();
    assert!(!suggestion.is_empty(), "Io suggestion should not be empty");
    assert!(
        suggestion.contains("filesystem"),
        "Io suggestion should mention filesystem, got: {suggestion}"
    );
}

#[test]
fn test_error_print_format_uses_hint_prefix() {
    let e = Error::Export(ExportError::WriteFailed {
        path: "/tmp/out.csv".into(),
        reason: "disk full".into(),
    });
    let formatted = format_error_output(&e);
    assert!(
        formatted.contains("\n  hint: "),
        "formatted output should contain hint prefix, got: {formatted}"
    );
    assert!(
        !formatted.contains("Suggestion:"),
        "formatted output should not contain old Suggestion: prefix, got: {formatted}"
    );
    assert!(
        formatted.starts_with("[ERROR]"),
        "first line should start with [ERROR], got: {formatted}"
    );
}

// IN-02: Interrupted is excluded from format_error_output by a matches! guard in main().
// This test documents that the guard fires (i.e. the condition is true for Interrupted)
// and that if format_error_output were called it would emit a [CRITICAL] hint line —
// confirming that the guard is necessary to suppress it.
#[test]
fn test_interrupted_matches_guard_is_true() {
    let e = Error::Interrupted;
    assert!(
        matches!(e, Error::Interrupted),
        "Interrupted variant must match the guard used in main()"
    );
    // If the guard were bypassed, format_error_output would produce a hint:
    let formatted = format_error_output(&e);
    assert!(
        formatted.starts_with("[CRITICAL]"),
        "format_error_output for Interrupted would produce [CRITICAL], got: {formatted}"
    );
    assert!(
        formatted.contains("\n  hint: "),
        "format_error_output for Interrupted would include hint line, got: {formatted}"
    );
}

#[test]
fn test_format_error_output_config_parse_failed_is_critical() {
    let e = Error::Config(ConfigError::ParseFailed {
        path: "/tmp/bad.toml".into(),
        reason: "unexpected EOF".into(),
    });
    let formatted = format_error_output(&e);
    assert!(
        formatted.starts_with("[CRITICAL]"),
        "ParseFailed should produce [CRITICAL] prefix, got: {formatted}"
    );
    assert!(
        formatted.contains("\n  hint: "),
        "ParseFailed should include hint line, got: {formatted}"
    );
    assert!(
        !formatted.contains("Suggestion:"),
        "should not use old Suggestion: prefix, got: {formatted}"
    );
}
