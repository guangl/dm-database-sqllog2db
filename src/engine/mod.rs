//! Run 引擎：解析日志文件 → pipeline 过滤/归一化 → 导出的完整处理管线。
//!
//! 主入口 [`run`]（在 `run.rs`）负责编排，按阶段分四个子模块：
//! - `prepare` — 输入文件解析、事务过滤器预扫描、内存预算并发控制
//! - `driver` — 并行（CSV/SQLite）、顺序、单文件切块四条执行路径
//! - `record` — 驱动路径共享的记录级"过滤 → 归一化 → 写出"循环
//! - `report` — 运行摘要与解析错误日志写出

mod driver;
mod prepare;
mod record;
mod report;
mod run;

#[cfg(test)]
mod tests;

pub use run::run;
