//! Watch 领域模块：监听输入目录中 `.log` 文件的创建与追加，触发全量/增量导出。
//! 主入口 [`run`]（在 `handler.rs`）；命令级 arg 处理保留在 `main.rs`。
//!
//! - `handler` — 入口与主循环（目录收集、watcher 启动、事件分发、摘要）
//! - `events`  — notify watcher 创建、事件路由、防抖
//! - `trigger` — 全量/增量两条触发路径与共享的追加模式注入
//! - `state`   — 运行时状态（`WatchLoopState`）与状态行渲染
//! - `offsets` — `_watch_offsets` 辅助表读写（增量 offset 持久化）

mod events;
mod handler;
mod offsets;
mod state;
mod trigger;

#[cfg(test)]
mod tests;

pub use handler::run;

// 以下 pub use 为集成测试（tests/watch_incremental.rs）提供公开 API，
// binary 内部不直接引用，通过 allow 消除 unused_imports lint。
pub use state::WatchLoopState;
pub use trigger::{trigger_full_file, trigger_incremental};
