use crate::error::{ConfigError, Error, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct SqllogConfig {
    /// 日志文件路径：目录、单文件或 glob 模式（e.g. `sqllogs/*.log`）
    /// 旧配置中的 `directory` 键仍被接受。
    #[serde(alias = "directory")]
    pub path: String,
}

impl Default for SqllogConfig {
    fn default() -> Self {
        Self {
            path: "sqllogs".to_string(),
        }
    }
}

impl SqllogConfig {
    pub fn validate(&self) -> Result<()> {
        if self.path.trim().is_empty() {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "sqllog.path".to_string(),
                value: self.path.clone(),
                reason: "Input path cannot be empty".to_string(),
            }));
        }
        Ok(())
    }
}
