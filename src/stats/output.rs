//! 统计输出层：将聚合结果写入 CSV 文件或 `SQLite` 数据库（不复用 `Exporter` trait）。

use crate::error::{Error, ExportError, Result};
use crate::exporter::csv::writer::write_csv_escaped;
use crate::exporter::ensure_parent_dir;
use crate::stats::aggregate::{FrequentSqlRow, SlowSqlRow};
use rusqlite::Connection;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// 将慢 SQL 与高频 SQL 统计结果写入 CSV 文件。
///
/// 输出文件名硬编码为 `slow_sql.csv` 和 `frequent_sql.csv`，放在 `csv_dir` 目录下。
pub(crate) fn write_csv_stats(
    slow: &[SlowSqlRow],
    frequent: &[FrequentSqlRow],
    csv_dir: &Path,
) -> Result<()> {
    ensure_parent_dir(&csv_dir.join("slow_sql.csv"))?;
    write_slow_csv(&csv_dir.join("slow_sql.csv"), slow)?;
    write_frequent_csv(&csv_dir.join("frequent_sql.csv"), frequent)?;
    Ok(())
}

/// 将慢 SQL 行写入 `slow_sql.csv`。
fn write_slow_csv(path: &Path, rows: &[SlowSqlRow]) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "sql_text,elapsed_ms,timestamp")?;
    for row in rows {
        let mut line_buf: Vec<u8> = Vec::with_capacity(row.sql_text.len() + 64);
        line_buf.push(b'"');
        write_csv_escaped(&mut line_buf, row.sql_text.as_bytes());
        line_buf.push(b'"');
        line_buf.push(b',');
        line_buf.extend_from_slice(itoa::Buffer::new().format(row.elapsed_ms).as_bytes());
        line_buf.push(b',');
        line_buf.push(b'"');
        line_buf.extend_from_slice(row.timestamp.as_bytes());
        line_buf.push(b'"');
        line_buf.push(b'\n');
        writer.write_all(&line_buf)?;
    }
    writer.flush()?;
    Ok(())
}

/// 将高频 SQL 行写入 `frequent_sql.csv`。
fn write_frequent_csv(path: &Path, rows: &[FrequentSqlRow]) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    writeln!(
        writer,
        "normalized_sql,call_count,avg_elapsed_ms,max_elapsed_ms"
    )?;
    for row in rows {
        let mut line_buf: Vec<u8> = Vec::with_capacity(row.normalized_sql.len() + 64);
        line_buf.push(b'"');
        write_csv_escaped(&mut line_buf, row.normalized_sql.as_bytes());
        line_buf.push(b'"');
        line_buf.push(b',');
        line_buf.extend_from_slice(itoa::Buffer::new().format(row.call_count).as_bytes());
        line_buf.push(b',');
        line_buf.extend_from_slice(itoa::Buffer::new().format(row.avg_elapsed_ms).as_bytes());
        line_buf.push(b',');
        line_buf.extend_from_slice(itoa::Buffer::new().format(row.max_elapsed_ms).as_bytes());
        line_buf.push(b'\n');
        writer.write_all(&line_buf)?;
    }
    writer.flush()?;
    Ok(())
}

/// 将慢 SQL 与高频 SQL 统计结果写入 `SQLite` 数据库（DROP + CREATE，不累积历史）。
pub(crate) fn write_sqlite_stats(
    slow: &[SlowSqlRow],
    frequent: &[FrequentSqlRow],
    db_url: &str,
) -> Result<()> {
    let db_path = Path::new(db_url);
    ensure_parent_dir(db_path)?;
    let conn =
        Connection::open(db_url).map_err(|err| db_err(format!("open sqlite failed: {err}")))?;
    conn.execute_batch("BEGIN TRANSACTION;")
        .map_err(|err| db_err(format!("begin transaction failed: {err}")))?;
    let tx_result = run_sqlite_transaction(&conn, slow, frequent);
    if tx_result.is_err() {
        conn.execute_batch("ROLLBACK;").ok();
        return tx_result;
    }
    conn.execute_batch("COMMIT;")
        .map_err(|err| db_err(format!("commit failed: {err}")))?;
    Ok(())
}

/// 在事务内写入两张表。
fn run_sqlite_transaction(
    conn: &Connection,
    slow: &[SlowSqlRow],
    frequent: &[FrequentSqlRow],
) -> Result<()> {
    write_slow_table(conn, slow).map_err(|err| db_err(format!("write slow_sql table: {err}")))?;
    write_frequent_table(conn, frequent)
        .map_err(|err| db_err(format!("write frequent_sql table: {err}")))?;
    Ok(())
}

/// 创建并填充 `slow_sql` 表（DROP + CREATE）。
fn write_slow_table(conn: &Connection, rows: &[SlowSqlRow]) -> rusqlite::Result<()> {
    conn.execute_batch("DROP TABLE IF EXISTS slow_sql;")?;
    conn.execute_batch(
        "CREATE TABLE slow_sql (\
            sql_text TEXT NOT NULL, \
            elapsed_ms INTEGER NOT NULL, \
            timestamp TEXT NOT NULL\
        );",
    )?;
    let mut stmt =
        conn.prepare("INSERT INTO slow_sql (sql_text, elapsed_ms, timestamp) VALUES (?, ?, ?)")?;
    for row in rows {
        stmt.execute(rusqlite::params![
            row.sql_text,
            row.elapsed_ms,
            row.timestamp
        ])?;
    }
    Ok(())
}

/// 创建并填充 `frequent_sql` 表（DROP + CREATE）。
fn write_frequent_table(conn: &Connection, rows: &[FrequentSqlRow]) -> rusqlite::Result<()> {
    conn.execute_batch("DROP TABLE IF EXISTS frequent_sql;")?;
    conn.execute_batch(
        "CREATE TABLE frequent_sql (\
            normalized_sql TEXT NOT NULL, \
            call_count INTEGER NOT NULL, \
            avg_elapsed_ms INTEGER NOT NULL, \
            max_elapsed_ms INTEGER NOT NULL\
        );",
    )?;
    let mut stmt = conn.prepare(
        "INSERT INTO frequent_sql \
            (normalized_sql, call_count, avg_elapsed_ms, max_elapsed_ms) \
            VALUES (?, ?, ?, ?)",
    )?;
    for row in rows {
        // rusqlite 不支持 u64，cast 到 i64（call_count 实际不会超过 i64::MAX）
        #[expect(
            clippy::cast_possible_wrap,
            reason = "call_count is bounded by log record count, well within i64 range"
        )]
        let call_count_i64 = row.call_count as i64;
        stmt.execute(rusqlite::params![
            row.normalized_sql,
            call_count_i64,
            row.avg_elapsed_ms,
            row.max_elapsed_ms
        ])?;
    }
    Ok(())
}

/// 将 `rusqlite` 错误或描述字符串转换为 `Error::Export`。
fn db_err(reason: String) -> Error {
    Error::Export(ExportError::DatabaseFailed { reason })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::aggregate::{FrequentSqlRow, SlowSqlRow};

    fn make_slow_rows(count: usize) -> Vec<SlowSqlRow> {
        (0..count)
            .map(|idx| {
                let idx_i64 = i64::try_from(idx).unwrap_or(0);
                SlowSqlRow {
                    sql_text: format!("SELECT * FROM t WHERE id = {idx}"),
                    elapsed_ms: (idx_i64 + 1) * 10,
                    timestamp: format!("2025-01-{:02}", idx + 1),
                }
            })
            .collect()
    }

    fn make_frequent_rows(count: usize) -> Vec<FrequentSqlRow> {
        (0..count)
            .map(|idx| {
                let count_seed = idx as u64;
                let ms_seed = i64::try_from(idx).unwrap_or(0);
                FrequentSqlRow {
                    normalized_sql: format!("SELECT * FROM t{idx}"),
                    call_count: (count_seed + 1) * 5,
                    avg_elapsed_ms: (ms_seed + 1) * 2,
                    max_elapsed_ms: (ms_seed + 1) * 10,
                }
            })
            .collect()
    }

    #[test]
    fn test_write_csv_stats_creates_two_files() {
        let dir = tempfile::TempDir::new().unwrap();
        let slow = make_slow_rows(2);
        let frequent = make_frequent_rows(2);
        write_csv_stats(&slow, &frequent, dir.path()).unwrap();
        assert!(dir.path().join("slow_sql.csv").exists());
        assert!(dir.path().join("frequent_sql.csv").exists());
    }

    #[test]
    fn test_write_csv_stats_headers_and_rows() {
        let dir = tempfile::TempDir::new().unwrap();
        let slow = make_slow_rows(2);
        let frequent = make_frequent_rows(3);
        write_csv_stats(&slow, &frequent, dir.path()).unwrap();

        let slow_content = std::fs::read_to_string(dir.path().join("slow_sql.csv")).unwrap();
        let slow_lines: Vec<&str> = slow_content.lines().collect();
        assert_eq!(slow_lines[0], "sql_text,elapsed_ms,timestamp");
        assert_eq!(slow_lines.len(), 3, "header + 2 data rows");

        let freq_content = std::fs::read_to_string(dir.path().join("frequent_sql.csv")).unwrap();
        let freq_lines: Vec<&str> = freq_content.lines().collect();
        assert_eq!(
            freq_lines[0],
            "normalized_sql,call_count,avg_elapsed_ms,max_elapsed_ms"
        );
        assert_eq!(freq_lines.len(), 4, "header + 3 data rows");
    }

    #[test]
    fn test_write_csv_stats_escapes_double_quotes() {
        let dir = tempfile::TempDir::new().unwrap();
        let slow = vec![SlowSqlRow {
            sql_text: r#"SELECT * FROM t WHERE name = "alice""#.to_string(),
            elapsed_ms: 5,
            timestamp: "2025-01-01".to_string(),
        }];
        write_csv_stats(&slow, &[], dir.path()).unwrap();
        let content = std::fs::read_to_string(dir.path().join("slow_sql.csv")).unwrap();
        let second_line = content.lines().nth(1).unwrap();
        // 字段以 " 开头，内嵌 " 变为 ""
        assert!(second_line.starts_with('"'), "should start with quote");
        assert!(
            second_line.contains(r#""""#),
            "inner quotes should be doubled"
        );
    }

    #[test]
    fn test_write_csv_stats_creates_parent_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let subdir = dir.path().join("sub").join("dir");
        write_csv_stats(&[], &[], &subdir).unwrap();
        assert!(subdir.exists());
    }

    #[test]
    fn test_write_csv_stats_empty_rows() {
        let dir = tempfile::TempDir::new().unwrap();
        write_csv_stats(&[], &[], dir.path()).unwrap();

        let slow_content = std::fs::read_to_string(dir.path().join("slow_sql.csv")).unwrap();
        assert_eq!(slow_content.lines().count(), 1, "only header, no data");
        let freq_content = std::fs::read_to_string(dir.path().join("frequent_sql.csv")).unwrap();
        assert_eq!(freq_content.lines().count(), 1, "only header, no data");
    }

    #[test]
    fn test_write_sqlite_stats_creates_two_tables() {
        let db_file = tempfile::NamedTempFile::new().unwrap();
        let db_url = db_file.path().to_str().unwrap().to_string();
        let slow = make_slow_rows(2);
        let frequent = make_frequent_rows(3);
        write_sqlite_stats(&slow, &frequent, &db_url).unwrap();

        let conn = Connection::open(&db_url).unwrap();
        let slow_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM slow_sql", [], |row| row.get(0))
            .unwrap();
        assert_eq!(slow_count, 2);
        let freq_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM frequent_sql", [], |row| row.get(0))
            .unwrap();
        assert_eq!(freq_count, 3);
    }

    #[test]
    fn test_write_sqlite_stats_schema() {
        let db_file = tempfile::NamedTempFile::new().unwrap();
        let db_url = db_file.path().to_str().unwrap().to_string();
        write_sqlite_stats(&make_slow_rows(1), &make_frequent_rows(1), &db_url).unwrap();

        let conn = Connection::open(&db_url).unwrap();

        let mut slow_cols: Vec<String> = Vec::new();
        let mut stmt = conn.prepare("PRAGMA table_info(slow_sql)").unwrap();
        let col_names = stmt.query_map([], |row| row.get::<_, String>(1)).unwrap();
        for name in col_names {
            slow_cols.push(name.unwrap());
        }
        assert_eq!(slow_cols, vec!["sql_text", "elapsed_ms", "timestamp"]);

        let mut freq_cols: Vec<String> = Vec::new();
        let mut stmt2 = conn.prepare("PRAGMA table_info(frequent_sql)").unwrap();
        let col_names2 = stmt2.query_map([], |row| row.get::<_, String>(1)).unwrap();
        for name in col_names2 {
            freq_cols.push(name.unwrap());
        }
        assert_eq!(
            freq_cols,
            vec![
                "normalized_sql",
                "call_count",
                "avg_elapsed_ms",
                "max_elapsed_ms"
            ]
        );
    }

    #[test]
    fn test_write_sqlite_stats_drop_recreates() {
        let db_file = tempfile::NamedTempFile::new().unwrap();
        let db_url = db_file.path().to_str().unwrap().to_string();
        // 第一次写 3 行
        write_sqlite_stats(&make_slow_rows(3), &make_frequent_rows(3), &db_url).unwrap();
        // 第二次写 1 行（DROP + CREATE 应覆盖）
        write_sqlite_stats(&make_slow_rows(1), &[], &db_url).unwrap();

        let conn = Connection::open(&db_url).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM slow_sql", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1, "DROP+CREATE should reset table, not accumulate");
        let freq_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM frequent_sql", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            freq_count, 0,
            "frequent_sql should have 0 rows after second write with empty frequent"
        );
    }

    #[test]
    fn test_write_sqlite_stats_creates_parent_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("nested").join("stats.db");
        let db_url = db_path.to_str().unwrap().to_string();
        write_sqlite_stats(&[], &[], &db_url).unwrap();
        assert!(db_path.parent().unwrap().exists());
    }
}
