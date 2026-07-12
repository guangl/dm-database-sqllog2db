//! Phase 70: `_watch_offsets` 辅助表读写。每次调用打开新连接，与 `SqliteExporter` 的 EXCLUSIVE 锁隔离（per D-05）。

use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 确保 `_watch_offsets` 辅助表存在（幂等）。
/// 每次调用打开独立连接，与 `SqliteExporter` 的 EXCLUSIVE 模式互不干扰（per D-05）。
pub(super) fn ensure_offset_table(database_url: &str) -> rusqlite::Result<()> {
    let connection = Connection::open(database_url)?;
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS _watch_offsets \
         (path TEXT NOT NULL PRIMARY KEY, byte_offset INTEGER NOT NULL);",
    )?;
    Ok(())
}

/// 从 `_watch_offsets` 表加载所有路径→字节偏移映射。
/// 表不存在（首次运行）或打开失败时返回空 `HashMap`，不 panic。
pub(super) fn load_offsets(database_url: &str) -> HashMap<PathBuf, u64> {
    let connection = match Connection::open(database_url) {
        Ok(conn) => conn,
        Err(e) => {
            log::warn!("watch::offsets::load_offsets open: {e}");
            return HashMap::new();
        }
    };
    let Ok(mut statement) = connection.prepare("SELECT path, byte_offset FROM _watch_offsets")
    else {
        // 表不存在是首次运行的正常状态，静默返回空 map（Pitfall：首次运行）
        return HashMap::new();
    };
    let Ok(pairs) = statement.query_map([], |row| {
        let path_str: String = row.get(0)?;
        let byte_offset: i64 = row.get(1)?;
        Ok((path_str, byte_offset))
    }) else {
        return HashMap::new();
    };
    pairs
        .filter_map(|result| {
            let (path_str, offset) = result.ok()?;
            // try_from 拒绝负值，防止 i64 → u64 产生极大值（per T-70-02 / Pitfall 2）
            let offset = u64::try_from(offset).ok()?;
            Some((PathBuf::from(path_str), offset))
        })
        .collect()
}

/// 持久化单条路径→字节偏移记录（INSERT OR REPLACE 语义）。
/// 失败时 `log::warn!` 但不中断 watch（per D-07：持久化失败不致命）。
pub(super) fn save_offset(database_url: &str, path: &Path, offset: u64) {
    let connection = match Connection::open(database_url) {
        Ok(conn) => conn,
        Err(e) => {
            log::warn!("watch::offsets::save_offset open: {e}");
            return;
        }
    };
    // u64 文件大小现实中不会溢出 i64（文件最大 ~8EB，i64::MAX ~9.2EB）；超出则饱和。
    let byte_offset = i64::try_from(offset).unwrap_or(i64::MAX);
    if let Err(e) = connection.execute(
        "INSERT OR REPLACE INTO _watch_offsets (path, byte_offset) VALUES (?1, ?2)",
        params![path.to_string_lossy().as_ref(), byte_offset],
    ) {
        log::warn!("watch::offsets::save_offset write: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_ensure_offset_table_idempotent() {
        let tmp = NamedTempFile::new().unwrap();
        let url = tmp.path().to_str().unwrap();
        // 调用两次均应返回 Ok(())
        assert!(
            ensure_offset_table(url).is_ok(),
            "第一次 ensure_offset_table 应返回 Ok(())"
        );
        assert!(
            ensure_offset_table(url).is_ok(),
            "第二次 ensure_offset_table 应返回 Ok(())（幂等）"
        );
    }

    #[test]
    fn test_save_and_load_offset_roundtrip() {
        let tmp = NamedTempFile::new().unwrap();
        let url = tmp.path().to_str().unwrap();
        ensure_offset_table(url).unwrap();
        let test_path = Path::new("/tmp/a.log");
        save_offset(url, test_path, 12345);
        let offsets = load_offsets(url);
        assert_eq!(
            offsets.get(&PathBuf::from("/tmp/a.log")),
            Some(&12345u64),
            "load_offsets 应返回刚写入的 offset=12345"
        );
    }

    #[test]
    fn test_load_offsets_missing_table_returns_empty() {
        let tmp = NamedTempFile::new().unwrap();
        let url = tmp.path().to_str().unwrap();
        // 未建表，直接调用 load_offsets
        let offsets = load_offsets(url);
        assert!(
            offsets.is_empty(),
            "未建表时 load_offsets 应返回空 HashMap，got: {offsets:?}"
        );
    }

    #[test]
    fn test_save_offset_replaces_existing() {
        let tmp = NamedTempFile::new().unwrap();
        let url = tmp.path().to_str().unwrap();
        ensure_offset_table(url).unwrap();
        let test_path = Path::new("/tmp/replace.log");
        save_offset(url, test_path, 100);
        save_offset(url, test_path, 200);
        let offsets = load_offsets(url);
        assert_eq!(
            offsets.get(&PathBuf::from("/tmp/replace.log")),
            Some(&200u64),
            "同路径第二次 save_offset 应替换为 200（INSERT OR REPLACE）"
        );
    }

    #[test]
    fn test_load_offsets_filters_negative() {
        let tmp = NamedTempFile::new().unwrap();
        let url = tmp.path().to_str().unwrap();
        ensure_offset_table(url).unwrap();
        // 直接插入负值行
        let connection = rusqlite::Connection::open(url).unwrap();
        connection
            .execute(
                "INSERT INTO _watch_offsets (path, byte_offset) VALUES (?1, ?2)",
                params!["/tmp/neg.log", -1i64],
            )
            .unwrap();
        drop(connection);
        let offsets = load_offsets(url);
        assert!(
            !offsets.contains_key(&PathBuf::from("/tmp/neg.log")),
            "负数 byte_offset=-1 应被过滤掉，不出现在结果中"
        );
        assert!(
            offsets.is_empty(),
            "过滤负值后 HashMap 应为空，got: {offsets:?}"
        );
    }
}
