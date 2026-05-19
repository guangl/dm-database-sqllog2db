use crate::error::{Error, ExportError, Result};
use crate::exporter::ensure_parent_dir;
use crate::pipeline::TemplateStats;
use rusqlite::Connection;
use std::path::Path;

/// 独立模板报告写入器——从 Exporter trait 中解耦
#[allow(dead_code)]
pub(crate) struct TemplateReporter;

#[allow(dead_code)]
impl TemplateReporter {
    /// 将模板统计写入 CSV 报告文件（D-02）——委托 `write_companion_rows`
    pub(crate) fn write_csv(path: &Path, stats: &[TemplateStats]) -> Result<()> {
        crate::exporter::csv::write_companion_rows(path, stats)
    }

    /// 将模板统计写入 `SQLite` 三表范式化报告文件（D-03）
    pub(crate) fn write_sqlite(path: &Path, stats: &[TemplateStats]) -> Result<()> {
        ensure_parent_dir(path).map_err(|e| {
            Error::Export(ExportError::WriteFailed {
                path: path.to_path_buf(),
                reason: format!("create dir failed: {e}"),
            })
        })?;

        let conn = Connection::open(path).map_err(|e| {
            Error::Export(ExportError::DatabaseFailed {
                reason: format!("open template db failed: {e}"),
            })
        })?;

        conn.execute_batch(
            "PRAGMA journal_mode = OFF;
             PRAGMA synchronous = OFF;
             PRAGMA cache_size = 1000000;
             PRAGMA temp_store = MEMORY;
             PRAGMA page_size = 65536;
             PRAGMA foreign_keys = ON;",
        )
        .map_err(|e| {
            Error::Export(ExportError::DatabaseFailed {
                reason: format!("template db pragma failed: {e}"),
            })
        })?;

        conn.execute_batch("BEGIN;").map_err(|e| {
            Error::Export(ExportError::DatabaseFailed {
                reason: format!("template db begin failed: {e}"),
            })
        })?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS template_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                template_key TEXT NOT NULL UNIQUE
            );
            CREATE TABLE IF NOT EXISTS template_stats (
                template_key_id INTEGER NOT NULL PRIMARY KEY REFERENCES template_keys(id),
                count INTEGER NOT NULL,
                avg_us INTEGER NOT NULL,
                min_us INTEGER NOT NULL,
                max_us INTEGER NOT NULL,
                first_seen TEXT NOT NULL,
                last_seen TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS latency_percentiles (
                template_key_id INTEGER NOT NULL REFERENCES template_keys(id),
                percentile_name TEXT NOT NULL,
                value_us INTEGER NOT NULL,
                PRIMARY KEY (template_key_id, percentile_name)
            );
            DELETE FROM latency_percentiles;
            DELETE FROM template_stats;
            DELETE FROM template_keys;",
        )
        .map_err(|e| {
            Error::Export(ExportError::DatabaseFailed {
                reason: format!("template db create tables failed: {e}"),
            })
        })?;

        for s in stats {
            conn.execute(
                "INSERT INTO template_keys (template_key) VALUES (?1)",
                rusqlite::params![s.template_key],
            )
            .map_err(|e| {
                Error::Export(ExportError::DatabaseFailed {
                    reason: format!("insert template_key failed: {e}"),
                })
            })?;

            let key_id = conn.last_insert_rowid();

            #[allow(clippy::cast_possible_wrap)]
            {
                conn.execute(
                    "INSERT INTO template_stats (template_key_id, count, avg_us, min_us, max_us, first_seen, last_seen) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![
                        key_id,
                        s.count as i64,
                        s.avg_us as i64,
                        s.min_us as i64,
                        s.max_us as i64,
                        s.first_seen,
                        s.last_seen,
                    ],
                )
                .map_err(|e| Error::Export(ExportError::DatabaseFailed {
                    reason: format!("insert template_stats failed: {e}"),
                }))?;
            }

            #[allow(clippy::cast_possible_wrap)]
            {
                conn.execute(
                    "INSERT INTO latency_percentiles (template_key_id, percentile_name, value_us) VALUES (?1, 'p50', ?2)",
                    rusqlite::params![key_id, s.p50_us as i64],
                )
                .map_err(|e| Error::Export(ExportError::DatabaseFailed {
                    reason: format!("insert latency p50 failed: {e}"),
                }))?;

                conn.execute(
                    "INSERT INTO latency_percentiles (template_key_id, percentile_name, value_us) VALUES (?1, 'p95', ?2)",
                    rusqlite::params![key_id, s.p95_us as i64],
                )
                .map_err(|e| Error::Export(ExportError::DatabaseFailed {
                    reason: format!("insert latency p95 failed: {e}"),
                }))?;

                conn.execute(
                    "INSERT INTO latency_percentiles (template_key_id, percentile_name, value_us) VALUES (?1, 'p99', ?2)",
                    rusqlite::params![key_id, s.p99_us as i64],
                )
                .map_err(|e| Error::Export(ExportError::DatabaseFailed {
                    reason: format!("insert latency p99 failed: {e}"),
                }))?;
            }
        }

        conn.execute_batch("COMMIT;").map_err(|e| {
            Error::Export(ExportError::DatabaseFailed {
                reason: format!("template db commit failed: {e}"),
            })
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::TemplateStats;
    use rusqlite::Connection;

    fn make_template_stats(key: &str, count: u64) -> TemplateStats {
        TemplateStats {
            template_key: key.to_string(),
            count,
            avg_us: 100,
            min_us: 10,
            max_us: 1000,
            p50_us: 90,
            p95_us: 500,
            p99_us: 900,
            first_seen: "2025-01-01 00:00:00".to_string(),
            last_seen: "2025-01-02 00:00:00".to_string(),
        }
    }

    #[test]
    fn test_write_sqlite_creates_three_tables() {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("templates.db");
        let stats = vec![make_template_stats("SELECT * FROM t WHERE id = ?", 100)];
        TemplateReporter::write_sqlite(&db_path, &stats).unwrap();
        let conn = Connection::open(&db_path).unwrap();
        let mut tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(std::result::Result::ok)
            .collect();
        tables.sort();
        assert_eq!(
            tables,
            vec![
                "latency_percentiles".to_string(),
                "template_keys".to_string(),
                "template_stats".to_string(),
            ]
        );
    }

    #[test]
    fn test_write_sqlite_data_correctness() {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("templates.db");
        let stats = vec![
            make_template_stats("SELECT * FROM a WHERE x = ?", 50),
            make_template_stats("INSERT INTO b VALUES (?)", 30),
        ];
        TemplateReporter::write_sqlite(&db_path, &stats).unwrap();
        let conn = Connection::open(&db_path).unwrap();

        let key_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM template_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(key_count, 2);

        let stats_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM template_stats", [], |r| r.get(0))
            .unwrap();
        assert_eq!(stats_count, 2);

        let pct_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM latency_percentiles", [], |r| r.get(0))
            .unwrap();
        assert_eq!(pct_count, 6); // 2 templates * 3 percentiles

        // Verify FK joins work
        let joined: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM template_keys k
                 JOIN template_stats s ON k.id = s.template_key_id
                 JOIN latency_percentiles p ON k.id = p.template_key_id
                 WHERE k.template_key = ?1",
                rusqlite::params!["SELECT * FROM a WHERE x = ?"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(joined, 3); // 3 percentile rows for this key
    }

    #[test]
    fn test_write_sqlite_empty_stats() {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("templates.db");
        TemplateReporter::write_sqlite(&db_path, &[]).unwrap();
        assert!(db_path.exists());
        let conn = Connection::open(&db_path).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM template_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_write_sqlite_overwrite() {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("templates.db");

        // First write: 2 templates
        let stats1 = vec![
            make_template_stats("SELECT * FROM a", 10),
            make_template_stats("SELECT * FROM b", 20),
        ];
        TemplateReporter::write_sqlite(&db_path, &stats1).unwrap();

        // Second write: 1 different template (overwrites)
        let stats2 = vec![make_template_stats("SELECT * FROM c", 30)];
        TemplateReporter::write_sqlite(&db_path, &stats2).unwrap();

        let conn = Connection::open(&db_path).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM template_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);

        let key: String = conn
            .query_row("SELECT template_key FROM template_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(key, "SELECT * FROM c");
    }
}
