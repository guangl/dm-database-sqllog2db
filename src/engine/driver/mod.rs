//! 执行路径驱动：多文件并行（CSV / SQLite）、顺序流式、单文件切块。
//! 由 `engine::run` 按导出器类型与文件数路由选择。

pub(crate) mod chunk;
pub(crate) mod parallel;
pub(crate) mod sequential;
pub(crate) mod sqlite;
