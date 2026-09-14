use super::*;

#[test]
fn test_default_inputs_are_empty() {
    let cfg = SqllogConfig::default();
    assert!(cfg.inputs.is_empty());
}

#[test]
fn test_validate_rejects_empty_inputs() {
    let cfg = SqllogConfig {
        inputs: vec![],
        path_deprecated: None,
    };
    let result = cfg.validate();
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("sqllog.inputs"),
        "Expected error message to contain 'sqllog.inputs', got: {err_msg}"
    );
}

#[test]
fn test_validate_rejects_whitespace_entry() {
    let cfg = SqllogConfig {
        inputs: vec!["  ".to_string()],
        path_deprecated: None,
    };
    let result = cfg.validate();
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("whitespace-only"),
        "Expected error message to contain 'whitespace-only', got: {err_msg}"
    );
}

#[test]
fn test_validate_rejects_legacy_path_key() {
    let cfg = SqllogConfig {
        inputs: vec!["sqllogs".to_string()],
        path_deprecated: Some(toml::Value::String("old".to_string())),
    };
    let result = cfg.validate();
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("sqllog.path"),
        "Expected error message to contain 'sqllog.path', got: {err_msg}"
    );
    assert!(
        err_msg.contains("inputs = [\""),
        "Expected error message to contain 'inputs = [\"', got: {err_msg}"
    );
}
