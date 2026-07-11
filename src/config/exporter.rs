use crate::error::{ConfigError, Error, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ExporterConfig {
    pub csv: Option<CsvExporterConfig>,
    pub sqlite: Option<SqliteExporterConfig>,
}

impl ExporterConfig {
    pub(super) fn has_any(&self) -> bool {
        self.csv.is_some() || self.sqlite.is_some()
    }

    pub fn validate(&self) -> Result<()> {
        if !self.has_any() {
            return Err(Error::Config(ConfigError::NoExporters));
        }
        if let Some(csv) = &self.csv {
            csv.validate()?;
        }
        if let Some(sqlite) = &self.sqlite {
            sqlite.validate()?;
        }
        Ok(())
    }
}

impl Default for ExporterConfig {
    fn default() -> Self {
        Self {
            csv: Some(CsvExporterConfig::default()),
            sqlite: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct CsvExporterConfig {
    pub file: String,
    #[serde(default = "default_true")]
    pub overwrite: bool,
    #[serde(default)]
    pub append: bool,
    /// 关闭时跳过 `parse_performance_metrics()`，CSV 省略 `exectime/rowcount/exec_id` 三列。
    #[serde(default = "default_true")]
    pub include_performance_metrics: bool,
    /// 可选：每文件最大行数，达到后自动切分到新文件（如 `sqllog_1.csv`, `sqllog_2.csv`）。
    /// 默认不设置（单文件输出）。设为 0 或不配置表示不切分。
    /// 切分模式下仅支持 overwrite = true。
    #[serde(default)]
    pub max_rows_per_file: Option<usize>,
}

impl Default for CsvExporterConfig {
    fn default() -> Self {
        Self {
            file: "outputs/sqllog.csv".to_string(),
            overwrite: true,
            append: false,
            include_performance_metrics: true,
            max_rows_per_file: None,
        }
    }
}

impl CsvExporterConfig {
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
        if let Some(max_rows) = self.max_rows_per_file {
            if max_rows == 0 {
                return Err(Error::Config(ConfigError::InvalidValue {
                    field: "exporter.csv.max_rows_per_file".to_string(),
                    value: "0".to_string(),
                    reason: "max_rows_per_file must be greater than 0, or omit the field entirely"
                        .to_string(),
                }));
            }
            if self.append || !self.overwrite {
                return Err(Error::Config(ConfigError::InvalidValue {
                    field: "exporter.csv.max_rows_per_file".to_string(),
                    value: max_rows.to_string(),
                    reason: "max_rows_per_file requires overwrite=true and append=false; \
                        append mode is not supported with file splitting"
                        .to_string(),
                }));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct SqliteExporterConfig {
    pub database_url: String,
    #[serde(default = "default_table_name")]
    pub table_name: String,
    #[serde(default = "default_true")]
    pub overwrite: bool,
    #[serde(default)]
    pub append: bool,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_multi_row_batch_size")]
    pub multi_row_batch_size: usize,
}

fn default_table_name() -> String {
    "sqllog_records".to_string()
}

fn default_batch_size() -> usize {
    10_000
}

fn default_multi_row_batch_size() -> usize {
    64
}

impl Default for SqliteExporterConfig {
    fn default() -> Self {
        Self {
            database_url: "export/sqllog2db.db".to_string(),
            table_name: "sqllog_records".to_string(),
            overwrite: true,
            append: false,
            batch_size: 10_000,
            multi_row_batch_size: 64,
        }
    }
}

impl SqliteExporterConfig {
    pub fn validate(&self) -> Result<()> {
        if self.database_url.trim().is_empty() {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "exporter.sqlite.database_url".to_string(),
                value: self.database_url.clone(),
                reason: "SQLite database URL cannot be empty".to_string(),
            }));
        }
        if self.table_name.trim().is_empty() {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "exporter.sqlite.table_name".to_string(),
                value: self.table_name.clone(),
                reason: "SQLite table name cannot be empty".to_string(),
            }));
        }
        let is_valid_ident = {
            let mut chars = self.table_name.chars();
            chars
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        };
        if !is_valid_ident {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "exporter.sqlite.table_name".to_string(),
                value: self.table_name.clone(),
                reason: "table name must match ^[a-zA-Z_][a-zA-Z0-9_]*$ (ASCII identifiers only)"
                    .to_string(),
            }));
        }
        if self.batch_size == 0 {
            return Err(ConfigError::InvalidValue {
                field: "exporter.sqlite.batch_size".to_string(),
                value: "0".to_string(),
                reason: "batch_size must be greater than 0".to_string(),
            }
            .into());
        }
        if self.multi_row_batch_size == 0 || self.multi_row_batch_size > 64 {
            return Err(ConfigError::InvalidValue {
                field: "exporter.sqlite.multi_row_batch_size".to_string(),
                value: self.multi_row_batch_size.to_string(),
                reason:
                    "multi_row_batch_size must be between 1 and 64 (15 cols × 64 = 960 < SQLITE_LIMIT_VARIABLE_NUMBER 999)"
                        .to_string(),
            }
            .into());
        }
        Ok(())
    }
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_cfg() -> SqliteExporterConfig {
        SqliteExporterConfig {
            database_url: "test.db".to_string(),
            table_name: "t".to_string(),
            overwrite: true,
            append: false,
            batch_size: 1000,
            multi_row_batch_size: 64,
        }
    }

    #[test]
    fn test_default_multi_row_batch_size() {
        assert_eq!(SqliteExporterConfig::default().multi_row_batch_size, 64);
    }

    #[test]
    fn test_validate_rejects_zero() {
        let mut cfg = base_cfg();
        cfg.multi_row_batch_size = 0;
        let err = cfg.validate().unwrap_err().to_string();
        assert!(
            err.contains("exporter.sqlite.multi_row_batch_size"),
            "error should mention field name, got: {err}"
        );
    }

    #[test]
    fn test_validate_rejects_over_64() {
        let mut cfg = base_cfg();
        cfg.multi_row_batch_size = 65;
        let err = cfg.validate().unwrap_err().to_string();
        assert!(
            err.contains("exporter.sqlite.multi_row_batch_size"),
            "error should mention field name, got: {err}"
        );
    }

    #[test]
    fn test_validate_accepts_boundaries() {
        let mut cfg = base_cfg();
        cfg.multi_row_batch_size = 1;
        cfg.validate()
            .expect("multi_row_batch_size = 1 should be valid");
        cfg.multi_row_batch_size = 64;
        cfg.validate()
            .expect("multi_row_batch_size = 64 should be valid");
    }
}
