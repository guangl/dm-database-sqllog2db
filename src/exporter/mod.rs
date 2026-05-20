use crate::config::Config;
use crate::error::{ConfigError, Error, Result};
use dm_database_parser_sqllog::{MetaParts, PerformanceMetrics, Sqllog};
use log::info;

pub mod csv;
pub(crate) mod projection;
pub mod sqlite;
pub(crate) use csv::CsvExporter;
pub(crate) use sqlite::SqliteExporter;

/// 所有导出器必须实现的接口
pub trait Exporter {
    fn initialize(&mut self) -> Result<()>;
    fn export(&mut self, sqllog: &Sqllog<'_>) -> Result<()>;

    /// 流式导出单条记录，同时附带 `normalized_sql`（流式路径，无需 batch）。
    /// 默认实现忽略 normalized，调用 `export`。
    fn export_one_normalized(
        &mut self,
        sqllog: &Sqllog<'_>,
        normalized: Option<&str>,
    ) -> Result<()> {
        let _ = normalized;
        self.export(sqllog)
    }

    /// 热路径：接收调用方已预解析的 `MetaParts` 和 `PerformanceMetrics`，
    /// 避免在导出器内部重复调用 `parse_meta()` / `parse_performance_metrics()`。
    /// 默认实现退化为 `export_one_normalized`（不使用预解析数据）。
    fn export_one_preparsed(
        &mut self,
        sqllog: &Sqllog<'_>,
        meta: &MetaParts<'_>,
        pm: &PerformanceMetrics<'_>,
        normalized: Option<&str>,
    ) -> Result<()> {
        let _ = (meta, pm);
        self.export_one_normalized(sqllog, normalized)
    }

    fn finalize(&mut self) -> Result<()>;

    fn stats_snapshot(&self) -> Option<ExportStats> {
        None
    }
}

/// 具体导出器的枚举包装，消除 `Box<dyn Exporter>` 的虚表分发开销，
/// 使编译器能够内联热路径（`export_one_preparsed` → `write_record_preparsed`）。
#[derive(Debug)]
pub(crate) enum ExporterKind {
    Csv(CsvExporter),
    Sqlite(SqliteExporter),
}

impl ExporterKind {
    fn kind_name(&self) -> &'static str {
        match self {
            Self::Csv(_) => "CSV",
            Self::Sqlite(_) => "SQLite",
        }
    }

    /// 当前 active exporter 是否应包含性能指标列（仅 CSV 路径有意义）。
    /// 用于 `cli/run.rs` 热循环判断是否需要调用 `record.parse_performance_metrics()`。
    pub fn csv_include_performance_metrics(&self) -> bool {
        match self {
            Self::Csv(exporter) => exporter.include_performance_metrics,
            // SQLite 永远需要完整 pm（schema 固定）
            Self::Sqlite(_) => true,
        }
    }

    fn initialize(&mut self) -> Result<()> {
        match self {
            Self::Csv(e) => e.initialize(),
            Self::Sqlite(e) => e.initialize(),
        }
    }

    #[inline]
    fn export_one_preparsed(
        &mut self,
        sqllog: &Sqllog<'_>,
        meta: &MetaParts<'_>,
        pm: &PerformanceMetrics<'_>,
        normalized: Option<&str>,
    ) -> Result<()> {
        match self {
            Self::Csv(e) => e.export_one_preparsed(sqllog, meta, pm, normalized),
            Self::Sqlite(e) => e.export_one_preparsed(sqllog, meta, pm, normalized),
        }
    }

    fn finalize(&mut self) -> Result<()> {
        match self {
            Self::Csv(e) => e.finalize(),
            Self::Sqlite(e) => e.finalize(),
        }
    }

    fn stats_snapshot(&self) -> Option<ExportStats> {
        match self {
            Self::Csv(e) => e.stats_snapshot(),
            Self::Sqlite(e) => e.stats_snapshot(),
        }
    }
}

/// 导出统计
#[derive(Debug, Default, Clone, Copy)]
pub struct ExportStats {
    pub exported: usize,
    pub skipped: usize,
    pub failed: usize,
    pub flush_operations: usize,
    pub last_flush_size: usize,
}

impl ExportStats {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_success(&mut self) {
        self.exported += 1;
    }

    #[must_use]
    pub fn total(&self) -> usize {
        self.exported + self.skipped + self.failed
    }
}

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
    /// CSV 路径根据配置返回；其他路径固定返回 true。
    pub(crate) fn csv_include_performance_metrics(&self) -> bool {
        self.exporter.csv_include_performance_metrics()
    }

    pub(crate) fn initialize(&mut self) -> Result<()> {
        info!("Initializing exporters...");
        self.exporter.initialize()?;
        info!("Exporters initialized");
        Ok(())
    }

    /// 热路径：使用预解析的 meta/pm，避免导出器内部重复解析。
    #[inline]
    pub(crate) fn export_one_preparsed(
        &mut self,
        sqllog: &Sqllog<'_>,
        meta: &MetaParts<'_>,
        pm: &PerformanceMetrics<'_>,
        normalized: Option<&str>,
    ) -> Result<()> {
        self.exporter
            .export_one_preparsed(sqllog, meta, pm, normalized)
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

/// 去除 IPv4-mapped IPv6 地址前缀（如 `::ffff:192.168.1.1` → `192.168.1.1`）
#[inline]
#[must_use]
pub(super) fn strip_ip_prefix(ip: &str) -> &str {
    const PREFIX: &str = "::ffff:";
    // 快速路径：IPv4 地址以数字开头，不以 ':' 开头，直接返回
    if ip.as_bytes().first() != Some(&b':') {
        return ip;
    }
    if ip.len() > PREFIX.len() && ip[..PREFIX.len()].eq_ignore_ascii_case(PREFIX) {
        &ip[PREFIX.len()..]
    } else {
        ip
    }
}

/// Saturating cast from f32 milliseconds to i64 milliseconds without precision-loss warnings
#[inline]
#[must_use]
pub(super) fn f32_ms_to_i64(ms: f32) -> i64 {
    if !ms.is_finite() {
        return 0;
    }

    const MAX_I64_F64: f64 = 9_223_372_036_854_775_807.0; // i64::MAX as f64
    const MIN_I64_F64: f64 = -9_223_372_036_854_775_808.0; // i64::MIN as f64

    let ms_f64 = f64::from(ms);
    if ms_f64 > MAX_I64_F64 {
        i64::MAX
    } else if ms_f64 < MIN_I64_F64 {
        i64::MIN
    } else {
        let clamped = ms_f64.trunc();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "value already clamped to i64 range"
        )]
        {
            clamped as i64
        }
    }
}

/// 确保输出文件的父目录存在
pub(super) fn ensure_parent_dir(path: &std::path::Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.exists()) {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
