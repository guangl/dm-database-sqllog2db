use rusqlite::Connection;

pub(super) fn initialize_pragmas(conn: &Connection) -> std::result::Result<(), rusqlite::Error> {
    // cache_size 的负值以 KiB 计，避免随 page_size 放大：旧的 100 万页在
    // 64 KiB 页下可缓存约 61 GiB。先设 page_size 再设缓存，确保预算按最终页大小计算。
    // 关闭 mmap，临时数据允许落盘。
    conn.execute_batch(
        "PRAGMA journal_mode = OFF;
         PRAGMA synchronous = OFF;
         PRAGMA page_size = 65536;
         PRAGMA cache_size = -16384;
         PRAGMA locking_mode = EXCLUSIVE;
         PRAGMA temp_store = FILE;
         PRAGMA mmap_size = 0;
         PRAGMA threads = 4;",
    )?;
    Ok(())
}
