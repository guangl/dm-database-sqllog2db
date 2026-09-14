//! Config 模块：根 Config 结构、子模块（sqllog/exporter/logging）、validate。

pub mod exporter;
pub mod logging;
pub mod sqllog;
pub(crate) mod template;
mod validate;

#[cfg(test)]
#[path = "../../tests/unit/config/mod.rs"]
mod tests;

// 以下 pub use 为 crate::config 路径的公开 API 重导出，
// 部分类型仅在测试代码中通过 crate::config::X 路径访问，
// 因此在非测试编译上下文中可能触发 unused_imports 警告。
pub use crate::stats::config::StatsConfig;
pub use exporter::{CsvExporterConfig, ExporterConfig, ParquetCompression, ParquetExporterConfig};
pub use logging::{LOG_LEVELS, LoggingConfig};
pub use sqllog::SqllogConfig;

use crate::error::{ConfigError, Error, Result};
use crate::pipeline::{FiltersFeature, NormalizeConfig, OutputConfig};
use serde::Deserialize;
use std::io;
use std::path::Path;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub sqllog: SqllogConfig,
    #[serde(default)]
    pub logging: Option<LoggingConfig>,
    #[serde(default)]
    pub exporter: ExporterConfig,
    #[serde(default)]
    pub replace_parameters: Option<NormalizeConfig>,
    #[serde(default)]
    pub filter: Option<FiltersFeature>,
    #[serde(default)]
    pub output: Option<OutputConfig>,
    #[serde(default)]
    pub stats: StatsConfig,
    #[serde(default)]
    pub error: Option<ErrorLogConfig>,
}

impl Config {
    /// 从 TOML 文件加载配置。
    ///
    /// # Errors
    ///
    /// 文件不存在、读取失败或 TOML 反序列化失败时返回错误。
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                Error::Config(ConfigError::NotFound(path.to_path_buf()))
            } else {
                Error::Io(e)
            }
        })?;
        toml::from_str(&content).map_err(|e| {
            Error::Config(ConfigError::ParseFailed {
                path: path.to_path_buf(),
                reason: e.to_string(),
            })
        })
    }
}

/// error log 输出配置。
#[derive(Debug, Deserialize, Clone)]
pub struct ErrorLogConfig {
    pub file: String,
}
