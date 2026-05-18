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

    fn write_template_stats(
        &mut self,
        stats: &[crate::pipeline::TemplateStats],
        _csv_output_path: Option<&str>,
        sqlite_table_name: Option<&str>,
    ) -> Result<()> {
        let Some(table_name) = sqlite_table_name else {
            return Ok(());
        };
        if table_name.trim().is_empty() {
            return Ok(());
        }
        let mut ident_chars = table_name.chars();
        let valid_ident = ident_chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && ident_chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !valid_ident {
            return Err(Error::Config(crate::error::ConfigError::InvalidValue {
                field: "template.output_sqlite_table".to_string(),
                value: table_name.to_string(),
                reason: "table name must start with a letter or underscore \
                         and contain only ASCII alphanumeric or underscore"
                    .to_string(),
            }));
        }
        let conn = self.conn_ref()?;
        conn.execute_batch("BEGIN;")
            .map_err(|e| Self::db_err(format!("begin failed: {e}")))?;
        if self.overwrite {
            conn.execute(&format!("DROP TABLE IF EXISTS \"{table_name}\""), [])
                .map_err(|e| Self::db_err(format!("drop {table_name} failed: {e}")))?;
        }
        conn.execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS \"{table_name}\" \
                 (template_key TEXT NOT NULL PRIMARY KEY, \
                  count INTEGER NOT NULL, \
                  avg_us INTEGER NOT NULL, \
                  min_us INTEGER NOT NULL, \
                  max_us INTEGER NOT NULL, \
                  p50_us INTEGER NOT NULL, \
                  p95_us INTEGER NOT NULL, \
                  p99_us INTEGER NOT NULL, \
                  first_seen TEXT NOT NULL, \
                  last_seen TEXT NOT NULL)"
            ),
            [],
        )
        .map_err(|e| Self::db_err(format!("create {table_name} failed: {e}")))?;
        #[allow(clippy::cast_possible_wrap)]
        for s in stats {
            #[rustfmt::skip]
            let p = rusqlite::params![s.template_key, s.count as i64, s.avg_us as i64, s.min_us as i64, s.max_us as i64, s.p50_us as i64, s.p95_us as i64, s.p99_us as i64, s.first_seen, s.last_seen];
            conn.execute(
                &format!("INSERT INTO \"{table_name}\" VALUES (?,?,?,?,?,?,?,?,?,?)"),
                p,
            )
            .map_err(|e| Self::db_err(format!("insert {table_name} failed: {e}")))?;
        }
        conn.execute_batch("COMMIT;")
            .map_err(|e| Self::db_err(format!("commit {table_name} failed: {e}")))?;
        info!(
            "{}: {} rows written to {}",
            table_name,
            stats.len(),
            self.database_url
        );
        Ok(())
    }

    fn stats_snapshot(&self) -> Option<ExportStats> {
        Some(self.stats)
    }
}

#[cfg(test)]
mod tests;
