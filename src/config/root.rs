use super::error_log::ErrorLogConfig;
use super::exporter::ExporterConfig;
use super::logging::LoggingConfig;
use super::sqllog::SqllogConfig;
use crate::error::{ConfigError, Error, Result};
use crate::pipeline::{FiltersFeature, NormalizeConfig, OutputConfig};
use crate::stats::config::StatsConfig;
use serde::Deserialize;
use std::io;
use std::path::Path;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub sqllog: SqllogConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
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
    /// watch 触发时设为 true，使 `write_error_log` 以追加模式打开文件。run 路径默认 false（覆盖写）。
    #[serde(skip)]
    pub append_error_log: bool,
}

impl Config {
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
