use super::{CsvExporter, ParquetExporter};
use super::{ExportStats, Exporter};
use crate::config::Config;
use crate::error::{ConfigError, Error, Result};
use dm_database_parser_sqllog::Sqllog;
use log::info;

use crate::config::exporter::ActiveExporter;

/// Owns the selected backend and dispatches export operations directly.
pub(crate) enum ExporterManager {
    Parquet(Box<ParquetExporter>),
    Csv(Box<CsvExporter>),
}

impl std::fmt::Debug for ExporterManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExporterManager")
            .field("exporter", &self.name())
            .finish()
    }
}

impl ExporterManager {
    /// 从已构建的 `CsvExporter` 创建管理器（并行处理时每个任务独立调用）。
    #[cfg(test)]
    #[must_use]
    pub(crate) fn from_csv(exporter: CsvExporter) -> Self {
        Self::Csv(Box::new(exporter))
    }

    pub(crate) fn from_config(config: &Config) -> Result<Self> {
        info!("Initializing exporter manager...");

        let normalize = config.replace_parameters.is_some();

        let field_mask = config.output.as_ref().map_or(
            crate::pipeline::FieldMask::ALL,
            crate::pipeline::OutputConfig::field_mask,
        );
        let ordered_indices = config.output.as_ref().map_or_else(
            || (0..crate::pipeline::FIELD_NAMES.len()).collect(),
            crate::pipeline::OutputConfig::ordered_field_indices,
        );

        match config.exporter.active() {
            Some(ActiveExporter::Parquet(cfg)) => {
                info!("Using Parquet exporter: {}", cfg.file);
                let mut exporter = ParquetExporter::from_config(cfg);
                exporter.normalize = normalize;
                exporter.field_mask = field_mask;
                exporter.ordered_indices = ordered_indices;
                Ok(Self::Parquet(Box::new(exporter)))
            }
            Some(ActiveExporter::Csv(cfg)) => {
                info!("Using CSV exporter: {}", cfg.file);
                let mut exporter = CsvExporter::from_config(cfg);
                exporter.normalize = normalize;
                exporter.field_mask = field_mask;
                exporter.ordered_indices = ordered_indices;
                Ok(Self::Csv(Box::new(exporter)))
            }
            None => Err(Error::Config(ConfigError::NoExporters)),
        }
    }

    pub(crate) fn log_stats(&self) {
        if let Some(s) = self.stats_snapshot() {
            info!(
                "Export stats: {} => success: {}, failed: {}, skipped: {} (total: {}){}",
                self.name(),
                s.exported,
                s.failed,
                s.skipped,
                s.total(),
                if s.flush_operations > 0 {
                    format!(
                        " | flushed: {} times (recent {} entries)",
                        s.flush_operations, s.last_flush_size
                    )
                } else {
                    String::new()
                }
            );
        }
    }
}

impl ExporterManager {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::Parquet(_) => "Parquet",
            Self::Csv(_) => "CSV",
        }
    }

    /// 当前 active exporter 是否应包含性能指标列（仅 CSV 路径有意义）。
    pub fn csv_include_performance_metrics(&self) -> bool {
        match self {
            Self::Csv(exporter) => exporter.include_performance_metrics,
            Self::Parquet(_) => true,
        }
    }

    pub(crate) fn initialize(&mut self) -> Result<()> {
        match self {
            Self::Parquet(e) => e.initialize(),
            Self::Csv(e) => e.initialize(),
        }
    }

    #[inline]
    pub(crate) fn export_one_preparsed(
        &mut self,
        sqllog: &Sqllog,
        include_pm: bool,
        normalized: Option<&str>,
    ) -> Result<()> {
        match self {
            Self::Parquet(e) => e.export_one_preparsed(sqllog, include_pm, normalized),
            Self::Csv(e) => e.export_one_preparsed(sqllog, include_pm, normalized),
        }
    }

    pub(crate) fn finalize(&mut self) -> Result<()> {
        match self {
            Self::Parquet(e) => e.finalize(),
            Self::Csv(e) => e.finalize(),
        }
    }

    pub(crate) fn stats_snapshot(&self) -> Option<ExportStats> {
        match self {
            Self::Parquet(e) => e.stats_snapshot(),
            Self::Csv(e) => e.stats_snapshot(),
        }
    }
}
