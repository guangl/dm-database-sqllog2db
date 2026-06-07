//! Run 子命令：主编排在 orchestrator.rs，各阶段子模块独立成文件。

mod collector;
mod error_log;
mod filter_processor;
mod input;
mod orchestrator;
mod parallel;
mod prescan;
mod processor;
mod sequential;
mod sqlite_parallel;
mod summary;

#[cfg(test)]
mod tests;

pub use orchestrator::handle_run;
