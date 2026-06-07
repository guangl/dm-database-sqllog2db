use super::kind::ExporterKind;
use super::{CsvExporter, SqliteExporter};
use crate::config::Config;
use crate::error::{ConfigError, Error, Result};
use dm_database_parser_sqllog::Sqllog;
use log::info;

/// 导出器管理器
pub(crate) struct ExporterManager {
    exporter: ExporterKind,
}

impl std::fmt::Debug for ExporterManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExporterManager")
            .field("exporter", &self.exporter.kind_name())
            .finish()
    }
}

impl ExporterManager {
    /// 从已构建的 `CsvExporter` 创建管理器（并行处理时每个任务独立调用）。
    #[must_use]
    pub(crate) fn from_csv(exporter: CsvExporter) -> Self {
        Self {
            exporter: ExporterKind::Csv(exporter),
        }
    }

    pub(crate) fn from_config(config: &Config) -> Result<Self> {
        info!("Initializing exporter manager...");

        let normalize = config.replace_parameters.as_ref().is_none_or(|r| r.enable);

        let field_mask = config.output.as_ref().map_or(
            crate::pipeline::FieldMask::ALL,
            crate::pipeline::OutputConfig::field_mask,
        );
        let ordered_indices = config.output.as_ref().map_or_else(
            || (0..crate::pipeline::FIELD_NAMES.len()).collect(),
            crate::pipeline::OutputConfig::ordered_field_indices,
        );

        if let Some(cfg) = &config.exporter.csv {
            info!("Using CSV exporter: {}", cfg.file);
            let mut exporter = CsvExporter::from_config(cfg);
            exporter.normalize = normalize;
            exporter.field_mask = field_mask;
            exporter.ordered_indices.clone_from(&ordered_indices);
            return Ok(Self {
                exporter: ExporterKind::Csv(exporter),
            });
        }

        if let Some(cfg) = &config.exporter.sqlite {
            info!("Using SQLite exporter: {}", cfg.database_url);
            let mut exporter = SqliteExporter::from_config(cfg);
            exporter.normalize = normalize;
            exporter.field_mask = field_mask;
            exporter.ordered_indices = ordered_indices;
            return Ok(Self {
                exporter: ExporterKind::Sqlite(exporter),
            });
        }

        Err(Error::Config(ConfigError::NoExporters))
    }

    /// 返回当前 active exporter 是否应包含性能指标列。
    pub(crate) fn csv_include_performance_metrics(&self) -> bool {
        self.exporter.csv_include_performance_metrics()
    }

    pub(crate) fn initialize(&mut self) -> Result<()> {
        info!("Initializing exporters...");
        self.exporter.initialize()?;
        info!("Exporters initialized");
        Ok(())
    }

    /// 仅对 `SQLite` exporter 生效，启用 WAL 模式。
    /// 非 `SQLite` exporter 时静默 no-op（per D-02：并行场景下启用 WAL）。
    pub(crate) fn set_sqlite_wal_mode(&self) -> Result<()> {
        match &self.exporter {
            ExporterKind::Sqlite(e) => e.set_wal_mode(),
            ExporterKind::Csv(_) => Ok(()),
        }
    }

    /// 热路径：使用已解析的 `Sqllog` 直接导出。
    #[inline]
    pub(crate) fn export_one_preparsed(
        &mut self,
        sqllog: &Sqllog,
        include_pm: bool,
        normalized: Option<&str>,
    ) -> Result<()> {
        self.exporter
            .export_one_preparsed(sqllog, include_pm, normalized)
    }

    pub(crate) fn finalize(&mut self) -> Result<()> {
        info!("Finalizing exporters...");
        self.exporter.finalize()?;
        info!("Exporters finished");
        Ok(())
    }

    #[must_use]
    pub(crate) fn name(&self) -> &str {
        self.exporter.kind_name()
    }

    pub(crate) fn log_stats(&self) {
        if let Some(s) = self.exporter.stats_snapshot() {
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
