use crate::error::{ConfigError, Error, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Default)]
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

impl SqllogConfig {
    /// 校验输入配置。
    ///
    /// # Errors
    ///
    /// 使用了已废弃的 `path` 字段或 `inputs` 含空白项时返回错误。
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
#[path = "../../tests/unit/config/sqllog.rs"]
mod tests;
