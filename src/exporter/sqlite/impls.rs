use super::super::{ExportStats, Exporter};
use super::exporter::SqliteExporter;
use super::pragma::initialize_pragmas;
use super::sql_builder::{build_create_sql, build_insert_sql};
use super::write::do_insert_preparsed;
use crate::error::Result;
use dm_database_parser_sqllog::Sqllog;
use log::info;
use rusqlite::Connection;
use std::path::Path;

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

    fn export(&mut self, sqllog: &Sqllog) -> Result<()> {
        self.export_one_normalized(sqllog, None)
    }

    fn export_one_normalized(&mut self, sqllog: &Sqllog, normalized: Option<&str>) -> Result<()> {
        self.export_one_preparsed(sqllog, true, normalized)
    }

    fn export_one_preparsed(
        &mut self,
        sqllog: &Sqllog,
        _include_pm: bool,
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
