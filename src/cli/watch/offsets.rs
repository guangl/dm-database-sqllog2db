//! Phase 70: `_watch_offsets` 辅助表读写。每次调用打开新连接，与 `SqliteExporter` 的 EXCLUSIVE 锁隔离（per D-05）。
