use super::super::ExportStats;
use crate::error::{Error, ExportError, Result};
use log::info;
use rusqlite::Connection;

pub(crate) struct SqliteExporter {
    pub(super) database_url: String,
    pub(super) table_name: String,
    pub(super) insert_sql: String,
    pub(super) overwrite: bool,
    pub(super) append: bool,
    pub(super) conn: Option<Connection>,
    pub(super) stats: ExportStats,
    pub(super) row_count: usize,
    pub(super) batch_size: usize,
    pub(crate) normalize: bool,
    pub(crate) field_mask: crate::pipeline::FieldMask,
    pub(crate) ordered_indices: Vec<usize>,
    pub(super) multi_row_batch_size: usize,
    pub(super) row_buffer: Vec<Vec<rusqlite::types::Value>>,
    pub(super) sql_cache: std::collections::HashMap<usize, String>,
}

impl std::fmt::Debug for SqliteExporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteExporter")
            .field("database_url", &self.database_url)
            .field("table_name", &self.table_name)
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}

impl SqliteExporter {
    #[must_use]
    pub(crate) fn new(
        database_url: String,
        table_name: String,
        overwrite: bool,
        append: bool,
    ) -> Self {
        let insert_sql = format!(
            "INSERT INTO \"{table_name}\" VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        );
        Self {
            database_url,
            table_name,
            insert_sql,
            overwrite,
            append,
            conn: None,
            stats: ExportStats::new(),
            row_count: 0,
            batch_size: 10_000,
            normalize: true,
            field_mask: crate::pipeline::FieldMask::ALL,
            ordered_indices: (0..crate::pipeline::FIELD_NAMES.len()).collect(),
            multi_row_batch_size: 64,
            row_buffer: Vec::new(),
            sql_cache: std::collections::HashMap::new(),
        }
    }

    #[must_use]
    pub(crate) fn from_config(config: &crate::config::SqliteExporterConfig) -> Self {
        let mut exporter = Self::new(
            config.database_url.clone(),
            config.table_name.clone(),
            config.overwrite,
            config.append,
        );
        exporter.batch_size = config.batch_size;
        exporter.multi_row_batch_size = config.multi_row_batch_size;
        exporter
    }

    pub(super) fn db_err(reason: impl Into<String>) -> Error {
        Error::Export(ExportError::DatabaseFailed {
            reason: reason.into(),
        })
    }

    pub(super) fn conn_ref(&self) -> Result<&Connection> {
        self.conn
            .as_ref()
            .ok_or_else(|| Self::db_err("not initialized"))
    }

    /// 启用 WAL 模式（仅供并行路径在 `initialize` 之后调用）。
    /// 不修改 `initialize_pragmas`，避免影响 benchmark 路径的 OFF+OFF 配置。
    /// 必须先 COMMIT 关闭 `initialize()` 开启的事务，再切换模式，最后重新 BEGIN。
    /// `locking_mode` 也须切回 NORMAL，因为 EXCLUSIVE 模式与 WAL 不兼容。
    pub(crate) fn set_wal_mode(&self) -> Result<()> {
        let conn = self.conn_ref()?;
        conn.execute_batch(
            "COMMIT;
             PRAGMA locking_mode = NORMAL;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             BEGIN TRANSACTION;",
        )
        .map_err(|e| Self::db_err(format!("set WAL mode failed: {e}")))?;
        Ok(())
    }

    pub(super) fn batch_commit_if_needed(&mut self) -> Result<()> {
        self.row_count += 1;
        if self.row_count % self.batch_size == 0 {
            let conn = self.conn_ref()?;
            conn.execute_batch("COMMIT; BEGIN")
                .map_err(|e| Self::db_err(format!("batch commit failed: {e}")))?;
        }
        Ok(())
    }

    /// 将 `row_buffer` 中缓积的行批量写入数据库。
    /// 返回已写入的行数（即 buffer 原长度）；buffer 在返回前被清空。
    pub(super) fn flush_batch(&mut self) -> Result<usize> {
        if self.row_buffer.is_empty() {
            return Ok(0);
        }
        let flushed = self.row_buffer.len();
        // 从缓存获取 SQL；若缓存未命中则构建并存入缓存
        if !self.sql_cache.contains_key(&flushed) {
            let ordered_indices_snapshot = self.ordered_indices.clone();
            let built = super::sql_builder::build_multi_row_insert_sql(
                &self.table_name,
                &ordered_indices_snapshot,
                flushed,
            );
            self.sql_cache.insert(flushed, built);
        }
        let sql = self.sql_cache[&flushed].clone();
        #[cfg(debug_assertions)]
        {
            let expected_placeholder_count = self.ordered_indices.len() * flushed;
            let actual = sql.matches('?').count();
            debug_assert_eq!(
                actual, expected_placeholder_count,
                "sql_cache[{flushed}] was built for a different col_count"
            );
        }
        let flattened: Vec<rusqlite::types::Value> = self.row_buffer.drain(..).flatten().collect();
        let conn = self.conn_ref()?;
        conn.execute(&sql, rusqlite::params_from_iter(flattened.iter()))
            .map_err(|e| Self::db_err(format!("batch insert failed: {e}")))?;
        Ok(flushed)
    }

    pub(super) fn handle_delete_clear_result(result: rusqlite::Result<usize>, table_name: &str) {
        if let Err(rusqlite::Error::SqliteFailure(_, Some(ref msg))) = result {
            if msg.contains("no such table") {
                return;
            }
        }
        if let Err(e) = result {
            log::warn!("sqlite clear failed for table {table_name}: {e}");
        }
    }

    /// 处理 overwrite 模式：DROP TABLE（在事务外执行，DDL 不支持回滚）。
    /// DELETE 模式在 `initialize()` 的事务内处理，见 `clear_table_rows_in_txn`。
    pub(super) fn prepare_target_table(&self) -> Result<()> {
        if self.overwrite {
            let conn = self.conn_ref()?;
            conn.execute(&format!("DROP TABLE IF EXISTS \"{}\"", self.table_name), [])
                .map_err(|e| Self::db_err(format!("drop table failed: {e}")))?;
            info!("Dropped existing table: {}", self.table_name);
        }
        Ok(())
    }

    /// 在事务内清空表行（非 overwrite、非 append 模式时调用）。
    /// 调用方须确保此方法在 BEGIN TRANSACTION 之后、COMMIT 之前执行。
    pub(super) fn clear_table_rows_in_txn(&self) -> Result<()> {
        Self::handle_delete_clear_result(
            self.conn_ref()?
                .execute(&format!("DELETE FROM \"{}\"", self.table_name), []),
            &self.table_name,
        );
        Ok(())
    }
}
