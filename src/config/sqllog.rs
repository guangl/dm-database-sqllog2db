use crate::error::{ConfigError, Error, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct SqllogConfig {
    /// 输入路径列表，支持目录、单文件或 glob 模式（如 `sqllogs/*.log`）
    #[serde(default)]
    pub inputs: Vec<String>,
    /// 旧键检测：捕获 `[sqllog] path = "..."` 旧格式。
    /// 非 None 时 validate() 会返回迁移错误，用户不应直接使用此字段。
    #[doc(hidden)]
    #[serde(rename = "path", default)]
    pub path_deprecated: Option<toml::Value>,
}

impl Default for SqllogConfig {
    fn default() -> Self {
        Self {
            inputs: vec!["sqllogs".to_string()],
            path_deprecated: None,
        }
    }
}

impl SqllogConfig {
    pub fn validate(&self) -> Result<()> {
        if let Some(ref deprecated_val) = self.path_deprecated {
            let raw = deprecated_val.to_string();
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "sqllog.path".to_string(),
                value: raw,
                reason: "字段 [sqllog].path 已移除，请改用 inputs = [\"...\"]（数组），例如 inputs = [\"sqllogs/*.log\"] 或 inputs = [\"sqllogs\"]".to_string(),
            }));
        }

        if self.inputs.is_empty() {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "sqllog.inputs".to_string(),
                value: "[]".to_string(),
                reason:
                    "inputs cannot be an empty array; provide at least one path, directory or glob"
                        .to_string(),
            }));
        }

        if self.inputs.iter().any(|s| s.trim().is_empty()) {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "sqllog.inputs".to_string(),
                value: format!("{:?}", self.inputs),
                reason: "inputs entries cannot be empty or whitespace-only".to_string(),
            }));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_inputs_is_sqllogs() {
        let cfg = SqllogConfig::default();
        assert_eq!(cfg.inputs, vec!["sqllogs".to_string()]);
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
}
