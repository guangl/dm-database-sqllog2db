//! Parser-independent predicates used by the filter pipeline.

use crate::model::LogRecord;

#[derive(Debug, Clone)]
pub(super) enum Predicate {
    Username(String),
    ClientIp(String),
    Session(String),
    Thread(String),
    App(String),
    Tag(String),
    TimestampAtLeast(String),
    TimestampAtMost(String),
    SqlContains(String),
    ExecId(i64),
    RuntimeAtLeast(f32),
    RowCountAtLeast(u32),
}

impl Predicate {
    pub(super) fn matches(&self, record: &LogRecord) -> bool {
        match self {
            Self::Username(expected) => record.username == *expected,
            Self::ClientIp(expected) => record.client_ip == *expected,
            Self::Session(expected) => record.sess_id == *expected,
            Self::Thread(expected) => record.thrd_id == *expected,
            Self::App(expected) => record.appname == *expected,
            Self::Tag(expected) => record.tag.as_deref() == Some(expected.as_str()),
            Self::TimestampAtLeast(expected) => record.ts >= *expected,
            Self::TimestampAtMost(expected) => record.ts <= *expected,
            Self::SqlContains(expected) => record.sql.contains(expected),
            Self::ExecId(expected) => record.exec_id == *expected,
            Self::RuntimeAtLeast(expected) => record.exectime >= *expected,
            Self::RowCountAtLeast(expected) => record.rowcount >= *expected,
        }
    }
}
