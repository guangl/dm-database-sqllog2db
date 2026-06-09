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
    // row_buffer and sql_cache are used in flush_batch (impls.rs Task 2)
    #[allow(dead_code)]
    pub(super) row_buffer: Vec<Vec<rusqlite::types::Value>>,
    #[allow(dead_code)]
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

    pub(super) fn prepare_target_table(&self) -> Result<()> {
        if self.overwrite {
            let conn = self.conn_ref()?;
            conn.execute(&format!("DROP TABLE IF EXISTS \"{}\"", self.table_name), [])
                .map_err(|e| Self::db_err(format!("drop table failed: {e}")))?;
            info!("Dropped existing table: {}", self.table_name);
        } else if !self.append {
            Self::handle_delete_clear_result(
                self.conn_ref()?
                    .execute(&format!("DELETE FROM \"{}\"", self.table_name), []),
                &self.table_name,
            );
        }
        Ok(())
    }
}
