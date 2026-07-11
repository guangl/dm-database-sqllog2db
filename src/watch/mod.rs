//! Watch 领域模块：主入口 `run` 在 `handler.rs`，按 watch 流程职责分文件组织。
//! 命令级 arg 处理保留在 `main.rs`；对外沿用 `dm_database_sqllog2db::watch::run`。

pub(super) mod offsets;

mod append;
mod debounce;
mod dirs;
mod event;
mod handler;
mod state;
mod status;
mod trigger_full;
mod trigger_incremental;
mod watcher;

#[cfg(test)]
mod tests;

pub use handler::run;

// 以下 pub use 为集成测试（tests/watch_incremental.rs）提供公开 API，
// binary 内部不直接引用，通过 allow 消除 unused_imports lint。
#[allow(unused_imports)]
pub use state::WatchLoopState;
#[allow(unused_imports)]
pub use trigger_full::trigger_full_file;
#[allow(unused_imports)]
pub use trigger_incremental::trigger_incremental;
