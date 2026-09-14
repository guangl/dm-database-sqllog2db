use super::Config;
use crate::error::{ConfigError, Error, Result};

impl Config {
    /// 校验整份配置（logging、exporter、sqllog、stats、output 各节）。
    ///
    /// # Errors
    ///
    /// 任一子配置节校验失败时返回该节的错误。
    pub fn validate(&self) -> Result<()> {
        if let Some(logging) = &self.logging {
            logging.validate()?;
        }
        self.exporter.validate()?;
        self.sqllog.validate()?;
        self.validate_output_fields()?;
        self.validate_stats_time_fields()?;
        self.validate_error_log()?;
        Ok(())
    }

    /// 校验统计命令实际使用的配置，不要求配置任何导出器。
    ///
    /// # Errors
    ///
    /// logging、sqllog、stats 或 error 配置无效时返回错误。
    pub fn validate_for_stats(&self) -> Result<()> {
        if let Some(logging) = &self.logging {
            logging.validate()?;
        }
        self.sqllog.validate()?;
        self.validate_stats_time_fields()?;
        self.validate_error_log()?;
        Ok(())
    }

    fn validate_error_log(&self) -> Result<()> {
        if let Some(err_cfg) = &self.error
            && err_cfg.file.trim().is_empty()
        {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "error.file".to_string(),
                value: err_cfg.file.clone(),
                reason: "error log file path must not be empty or whitespace".to_string(),
            }));
        }
        Ok(())
    }

    fn validate_stats_time_fields(&self) -> Result<()> {
        crate::stats::config::validate_stats_time_range(&self.stats)
    }

    fn validate_output_fields(&self) -> Result<()> {
        if let Some(names) = self.output.as_ref().and_then(|o| o.fields.as_ref()) {
            for name in names {
                if !crate::pipeline::FIELD_NAMES.contains(&name.as_str()) {
                    return Err(Error::Config(ConfigError::InvalidValue {
                        field: "output.fields".to_string(),
                        value: name.clone(),
                        reason: format!(
                            "unknown field '{name}'; valid fields: {}",
                            crate::pipeline::FIELD_NAMES.join(", ")
                        ),
                    }));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../../tests/unit/config/validate.rs"]
mod tests;
