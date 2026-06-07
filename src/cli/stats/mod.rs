//! Stats 子命令入口：合并 CLI 与配置选项后委托 `crate::stats::run_stats`。
mod handler;
#[cfg(test)]
mod tests;

pub use handler::handle_stats;
