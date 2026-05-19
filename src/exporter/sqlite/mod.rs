use crate::error::{Error, ExportError, Result};
use dm_database_parser_sqllog::{MetaParts, PerformanceMetrics, Sqllog};
use log::info;
use rusqlite::Connection;
use std::path::Path;

use super::{ExportStats, Exporter};

mod sql_builder;
mod write;

use self::sql_builder::{build_create_sql, build_insert_sql};
use self::write::do_insert_preparsed;

pub(crate) struct SqliteExporter {
    database_url: String,
    table_name: String,
    insert_sql: String,
    overwrite: bool,
    append: bool,
    conn: Option<Connection>,
    stats: ExportStats,
    row_count: usize,
    batch_size: usize,
    pub(super) normalize: bool,
    pub(super) field_mask: crate::pipeline::FieldMask,
    pub(super) ordered_indices: Vec<usize>,
}

fn initialize_pragmas(conn: &Connection) -> std::result::Result<(), rusqlite::Error> {
    conn.execute_batch(
        "PRAGMA journal_mode = OFF;
         PRAGMA synchronous = OFF;
         PRAGMA cache_size = 1000000;
         PRAGMA locking_mode = EXCLUSIVE;
         PRAGMA temp_store = MEMORY;
         PRAGMA mmap_size = 30000000000;
         PRAGMA page_size = 65536;
         PRAGMA threads = 4;",
    )?;
    Ok(())
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
        exporter
    }

    fn db_err(reason: impl Into<String>) -> Error {
        Error::Export(ExportError::DatabaseFailed {
            reason: reason.into(),
        })
    }

    /// 获取数据库连接引用，未初始化时返回错误而非 panic
    fn conn_ref(&self) -> Result<&Connection> {
        self.conn
            .as_ref()
            .ok_or_else(|| Self::db_err("not initialized"))
    }

    /// 每 `batch_size` 行执行中间 COMMIT，将大事务拆分为小事务
    fn batch_commit_if_needed(&mut self) -> Result<()> {
        self.row_count += 1;
        if self.row_count % self.batch_size == 0 {
            let conn = self.conn_ref()?;
            conn.execute_batch("COMMIT; BEGIN")
                .map_err(|e| Self::db_err(format!("batch commit failed: {e}")))?;
        }
        Ok(())
    }

    /// 处理 DELETE FROM 的执行结果（"no such table" 静默忽略）
    fn handle_delete_clear_result(result: rusqlite::Result<usize>, table_name: &str) {
        if let Err(rusqlite::Error::SqliteFailure(_, Some(ref msg))) = result {
            if msg.contains("no such table") {
                return;
            }
        }
        if let Err(e) = result {
            log::warn!("sqlite clear failed for table {table_name}: {e}");
        }
    }

    /// 仅打开数据库连接并设置 pragmas，不创建主数据表。
    /// 用于并行 CSV 路径中写入模板统计的场景，避免创建空的主数据表。
    #[allow(dead_code)]
    pub(crate) fn open_connection_only(&mut self) -> Result<()> {
        let conn = Connection::open(&self.database_url)
            .map_err(|e| Self::db_err(format!("open failed: {e}")))?;
        initialize_pragmas(&conn).map_err(|e| Self::db_err(format!("set PRAGMAs failed: {e}")))?;
        self.conn = Some(conn);
        Ok(())
    }

    /// 根据 overwrite/append 模式准备目标表
    fn prepare_target_table(&self) -> Result<()> {
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

impl Exporter for SqliteExporter {
    fn initialize(&mut self) -> Result<()> {
        info!("Initializing SQLite exporter: {}", self.database_url);

        let path = Path::new(&self.database_url);
        if let Some(parent) = path.parent().filter(|p| !p.exists()) {
            std::fs::create_dir_all(parent)
                .map_err(|e| Self::db_err(format!("create dir failed: {e}")))?;
        }

        let conn = Connection::open(&self.database_url)
            .map_err(|e| Self::db_err(format!("open failed: {e}")))?;

        initialize_pragmas(&conn).map_err(|e| Self::db_err(format!("set PRAGMAs failed: {e}")))?;

        self.conn = Some(conn);
        self.row_count = 0;

        self.prepare_target_table()?;

        self.insert_sql = build_insert_sql(&self.table_name, &self.ordered_indices);

        let conn = self.conn_ref()?;
        let create_sql = build_create_sql(&self.table_name, &self.ordered_indices);
        conn.execute(&create_sql, [])
            .map_err(|e| Self::db_err(format!("create table failed: {e}")))?;

        conn.execute_batch("BEGIN TRANSACTION;")
            .map_err(|e| Self::db_err(format!("begin transaction failed: {e}")))?;

        info!("SQLite exporter initialized: {}", self.database_url);
        Ok(())
    }

    fn export(&mut self, sqllog: &Sqllog<'_>) -> Result<()> {
        self.export_one_normalized(sqllog, None)
    }

    fn export_one_normalized(
        &mut self,
        sqllog: &Sqllog<'_>,
        normalized: Option<&str>,
    ) -> Result<()> {
        let meta = sqllog.parse_meta();
        let pm = sqllog.parse_performance_metrics();
        self.export_one_preparsed(sqllog, &meta, &pm, normalized)
    }

    fn export_one_preparsed(
        &mut self,
        sqllog: &Sqllog<'_>,
        meta: &MetaParts<'_>,
        pm: &PerformanceMetrics<'_>,
        normalized: Option<&str>,
    ) -> Result<()> {
        {
            let conn = self
                .conn
                .as_ref()
                .ok_or_else(|| Self::db_err("not initialized"))?;
            let mut stmt = conn
                .prepare_cached(&self.insert_sql)
                .map_err(|e| Self::db_err(format!("prepare failed: {e}")))?;
            let ns_ref = if self.normalize { normalized } else { None };
            do_insert_preparsed(
                &mut stmt,
                sqllog,
                meta,
                pm,
                ns_ref,
                self.field_mask,
                &self.ordered_indices,
            )
            .map_err(|e| Self::db_err(format!("insert failed: {e}")))?;
        }
        self.stats.record_success();
        self.batch_commit_if_needed()?;
        Ok(())
    }

    fn finalize(&mut self) -> Result<()> {
        if let Some(conn) = &self.conn {
            conn.execute_batch("COMMIT;")
                .map_err(|e| Self::db_err(format!("commit failed: {e}")))?;
        }
        info!(
            "SQLite export finished: {} (success: {}, failed: {})",
            self.database_url, self.stats.exported, self.stats.failed
        );
        Ok(())
    }

    fn stats_snapshot(&self) -> Option<ExportStats> {
        Some(self.stats)
    }
}

#[cfg(test)]
mod tests;
