use super::stats::ExportStats;
use crate::error::Result;
use dm_database_parser_sqllog::Sqllog;

/// 所有导出器必须实现的接口
pub trait Exporter {
    fn initialize(&mut self) -> Result<()>;
    fn export(&mut self, sqllog: &Sqllog) -> Result<()>;

    /// 流式导出单条记录，同时附带 `normalized_sql`（流式路径，无需 batch）。
    /// 默认实现忽略 normalized，调用 `export`。
    fn export_one_normalized(&mut self, sqllog: &Sqllog, normalized: Option<&str>) -> Result<()> {
        let _ = normalized;
        self.export(sqllog)
    }

    /// 热路径：使用已解析的 `Sqllog` 直接导出。
    /// `include_pm` 控制是否写入性能指标列（仅 CSV 路径有意义）。
    /// parser 库已将所有字段物化到 `Sqllog`，`meta`/`pm` 参数取消。
    fn export_one_preparsed(
        &mut self,
        sqllog: &Sqllog,
        include_pm: bool,
        normalized: Option<&str>,
    ) -> Result<()> {
        let _ = include_pm;
        self.export_one_normalized(sqllog, normalized)
    }

    fn finalize(&mut self) -> Result<()>;

    fn stats_snapshot(&self) -> Option<ExportStats> {
        None
    }
}
