use crate::error::{ConfigError, Error, Result};
use serde::Deserialize;

pub const LOG_LEVELS: &[&str] = &["trace", "debug", "info", "warn", "error"];

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    #[serde(default = "default_file")]
    pub file: String,
    #[serde(default = "default_level")]
    pub level: String,
    #[serde(default = "default_retention_days")]
    pub retention_days: usize,
}

fn default_file() -> String {
    "logs/sqllog2db.log".to_string()
}

fn default_level() -> String {
    "info".to_string()
}

fn default_retention_days() -> usize {
    7
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            file: "logs/sqllog2db.log".to_string(),
            level: "info".to_string(),
            retention_days: 7,
        }
    }
}

impl LoggingConfig {
    /// 校验日志配置。
    ///
    /// # Errors
    ///
    /// 日志文件路径为空白、日志级别不在合法集合内或 `retention_days` 越界时返回错误。
    pub fn validate(&self) -> Result<()> {
        if self.file.trim().is_empty() {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "logging.file".to_string(),
                value: self.file.clone(),
                reason: "Log file path cannot be empty".to_string(),
            }));
        }
        if !LOG_LEVELS
            .iter()
            .any(|&l| l.eq_ignore_ascii_case(&self.level))
        {
            return Err(Error::Config(ConfigError::InvalidLogLevel {
                level: self.level.clone(),
                valid_levels: LOG_LEVELS.iter().map(|s| (*s).to_string()).collect(),
            }));
        }
        if self.retention_days == 0 || self.retention_days > 365 {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "logging.retention_days".to_string(),
                value: self.retention_days.to_string(),
                reason: "Retention days must be between 1 and 365".to_string(),
            }));
        }
        Ok(())
    }
}
