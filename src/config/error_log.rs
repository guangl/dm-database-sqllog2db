use serde::Deserialize;

/// error log 输出配置。
#[derive(Debug, Deserialize, Clone)]
pub struct ErrorLogConfig {
    pub file: String,
}
