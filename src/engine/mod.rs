//! Run 引擎：解析日志文件 → pipeline 过滤/归一化 → 导出的完整处理管线。
//!
//! 主入口 [`run`]（原 `cli::run::handle_run`）负责编排；解析/并行/顺序/切块等各阶段
//! 拆分为独立子模块。`cli::run` 命令层已溶解至此——命令级 arg 处理保留在 `main.rs`。

mod chunk;
#[cfg(test)]
mod collector;
mod error_log;
mod input;
mod memory_budget;
mod parallel;
mod prescan;
mod processor;
mod record_iter;
mod run;
mod sequential;
mod sqlite_parallel;
mod summary;

#[cfg(test)]
mod tests;

pub use run::run;
