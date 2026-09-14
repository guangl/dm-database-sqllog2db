use crate::error::{ConfigError, Error, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct ExporterConfig {
    pub parquet: Option<ParquetExporterConfig>,
    pub csv: Option<CsvExporterConfig>,
}

/// The selected output backend; Parquet takes precedence when both are configured.
#[derive(Clone, Copy)]
pub(crate) enum ActiveExporter<'a> {
    Parquet(&'a ParquetExporterConfig),
    Csv(&'a CsvExporterConfig),
}

impl ExporterConfig {
    pub(crate) fn active(&self) -> Option<ActiveExporter<'_>> {
        self.parquet
            .as_ref()
            .map(ActiveExporter::Parquet)
            .or_else(|| self.csv.as_ref().map(ActiveExporter::Csv))
    }

    pub(super) fn has_any(&self) -> bool {
        self.active().is_some()
    }

    /// 校验导出器配置组合。
    ///
    /// # Errors
    ///
    /// 未配置任何导出器，或已配置导出器的字段校验失败时返回错误。
    pub fn validate(&self) -> Result<()> {
        if !self.has_any() {
            return Err(Error::Config(ConfigError::NoExporters));
        }
        if let Some(parquet) = &self.parquet {
            parquet.validate()?;
        }
        if let Some(csv) = &self.csv {
            csv.validate()?;
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ParquetCompression {
    #[default]
    Zstd,
    Snappy,
    Uncompressed,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ParquetExporterConfig {
    pub file: String,
    #[serde(default = "default_true")]
    pub overwrite: bool,
    #[serde(default)]
    pub compression: ParquetCompression,
    #[serde(default = "default_row_group_rows")]
    pub row_group_rows: usize,
}

fn default_row_group_rows() -> usize {
    65_536
}

impl Default for ParquetExporterConfig {
    fn default() -> Self {
        Self {
            file: "outputs/sqllog.parquet".to_string(),
            overwrite: true,
            compression: ParquetCompression::Zstd,
            row_group_rows: default_row_group_rows(),
        }
    }
}

impl ParquetExporterConfig {
    /// 校验 Parquet 输出路径和 row group 大小。
    ///
    /// # Errors
    ///
    /// 输出路径为空，或 `row_group_rows` 为 0 时返回配置错误。
    pub fn validate(&self) -> Result<()> {
        if self.file.trim().is_empty() {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "exporter.parquet.file".to_string(),
                value: self.file.clone(),
                reason: "Parquet output file path cannot be empty".to_string(),
            }));
        }
        if self.row_group_rows == 0 {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "exporter.parquet.row_group_rows".to_string(),
                value: "0".to_string(),
                reason: "row_group_rows must be greater than 0".to_string(),
            }));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct CsvExporterConfig {
    pub file: String,
    #[serde(default = "default_true")]
    pub overwrite: bool,
    #[serde(default)]
    pub append: bool,
    /// 关闭时跳过 `parse_performance_metrics()`，CSV 省略 `exectime/rowcount/exec_id` 三列。
    #[serde(default = "default_true")]
    pub include_performance_metrics: bool,
}

impl Default for CsvExporterConfig {
    fn default() -> Self {
        Self {
            file: "outputs/sqllog.csv".to_string(),
            overwrite: true,
            append: false,
            include_performance_metrics: true,
        }
    }
}

impl CsvExporterConfig {
    /// 校验 CSV 导出器配置。
    ///
    /// # Errors
    ///
    /// 输出文件路径为空白，或其他字段值非法时返回错误。
    pub fn validate(&self) -> Result<()> {
        if self.file.trim().is_empty() {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "exporter.csv.file".to_string(),
                value: self.file.clone(),
                reason: "CSV output file path cannot be empty".to_string(),
            }));
        }
        if !self.append && !self.overwrite {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "exporter.csv".to_string(),
                value: "overwrite=false, append=false".to_string(),
                reason: "at least one of overwrite or append must be true; \
                    both false would silently truncate an existing file"
                    .to_string(),
            }));
        }
        Ok(())
    }
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
#[path = "../../tests/unit/config/exporter.rs"]
mod tests;
