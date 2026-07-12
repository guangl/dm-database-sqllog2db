use super::stats::ExportStats;
use crate::error::Result;
use dm_database_parser_sqllog::Sqllog;

/// 所有导出器必须实现的接口
pub trait Exporter {
    /// 初始化导出目标（创建文件/表等）。
    ///
    /// # Errors
    ///
    /// 输出目标创建或打开失败时返回错误（由具体实现决定，如文件 IO、数据库连接）。
    fn initialize(&mut self) -> Result<()>;

    /// 导出单条记录。
    ///
    /// # Errors
    ///
    /// 写出失败时返回错误（如磁盘写入失败、数据库执行失败）。
    fn export(&mut self, sqllog: &Sqllog) -> Result<()>;

    /// 流式导出单条记录，同时附带 `normalized_sql`（流式路径，无需 batch）。
    /// 默认实现忽略 normalized，调用 `export`。
    ///
    /// # Errors
    ///
    /// 同 [`Exporter::export`]：写出失败时返回错误。
    fn export_one_normalized(&mut self, sqllog: &Sqllog, normalized: Option<&str>) -> Result<()> {
        let _ = normalized;
        self.export(sqllog)
    }

    /// 热路径：使用已解析的 `Sqllog` 直接导出。
    /// `include_pm` 控制是否写入性能指标列（仅 CSV 路径有意义）。
    /// parser 库已将所有字段物化到 `Sqllog`，`meta`/`pm` 参数取消。
    ///
    /// # Errors
    ///
    /// 同 [`Exporter::export`]：写出失败时返回错误。
    fn export_one_preparsed(
        &mut self,
        sqllog: &Sqllog,
        include_pm: bool,
        normalized: Option<&str>,
    ) -> Result<()> {
        let _ = include_pm;
        self.export_one_normalized(sqllog, normalized)
    }

    /// 结束导出：flush 缓冲并关闭输出目标。
    ///
    /// # Errors
    ///
    /// 缓冲落盘或事务提交失败时返回错误。
    fn finalize(&mut self) -> Result<()>;

    fn stats_snapshot(&self) -> Option<ExportStats> {
        None
    }
}
