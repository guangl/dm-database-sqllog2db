use super::api::Exporter;
use super::stats::ExportStats;
use super::{CsvExporter, SqliteExporter};
use crate::error::Result;
use dm_database_parser_sqllog::Sqllog;

/// 具体导出器的枚举包装，消除 `Box<dyn Exporter>` 的虚表分发开销，
/// 使编译器能够内联热路径。
#[derive(Debug)]
pub(crate) enum ExporterKind {
    Csv(CsvExporter),
    Sqlite(SqliteExporter),
}

impl ExporterKind {
    pub(crate) fn kind_name(&self) -> &'static str {
        match self {
            Self::Csv(_) => "CSV",
            Self::Sqlite(_) => "SQLite",
        }
    }

    /// 当前 active exporter 是否应包含性能指标列（仅 CSV 路径有意义）。
    pub fn csv_include_performance_metrics(&self) -> bool {
        match self {
            Self::Csv(exporter) => exporter.include_performance_metrics,
            Self::Sqlite(_) => true,
        }
    }

    pub(crate) fn initialize(&mut self) -> Result<()> {
        match self {
            Self::Csv(e) => e.initialize(),
            Self::Sqlite(e) => e.initialize(),
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
            Self::Csv(e) => e.export_one_preparsed(sqllog, include_pm, normalized),
            Self::Sqlite(e) => e.export_one_preparsed(sqllog, include_pm, normalized),
        }
    }

    pub(crate) fn finalize(&mut self) -> Result<()> {
        match self {
            Self::Csv(e) => e.finalize(),
            Self::Sqlite(e) => e.finalize(),
        }
    }

    pub(crate) fn stats_snapshot(&self) -> Option<ExportStats> {
        match self {
            Self::Csv(e) => e.stats_snapshot(),
            Self::Sqlite(e) => e.stats_snapshot(),
        }
    }
}
