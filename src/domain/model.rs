//! Domain records shared by every parser, filter and exporter.
//!
//! Parser crates are intentionally kept behind the streaming adapters. This prevents a new
//! input format from forcing changes through the processing and export layers.

/// A normalized log record used by the application pipeline.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LogRecord {
    /// Timestamp in the source log's display format.
    pub ts: String,
    /// Source-specific category, such as `SEL`, `ORA` or `execute`.
    pub tag: Option<String>,
    /// Execution point, when the source provides one.
    pub ep: u8,
    /// Session identifier.
    pub sess_id: String,
    /// Thread identifier.
    pub thrd_id: String,
    /// Database user, when available.
    pub username: String,
    /// Transaction or source execution identifier.
    pub trxid: String,
    /// Statement or method identifier.
    pub statement: String,
    /// Application or driver name.
    pub appname: String,
    /// Client IP address, when available.
    pub client_ip: String,
    /// SQL text or the best source representation of the event.
    pub sql: String,
    /// Execution time in milliseconds.
    pub exectime: f32,
    /// Affected row count, when available.
    pub rowcount: u32,
    /// Execution identifier, when available.
    pub exec_id: i64,
}
