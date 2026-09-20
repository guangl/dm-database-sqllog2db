//! Adapter for the traditional DM SQL log parser.

use super::{InputParseError, RecordIterator};
use crate::model::LogRecord;
use dm_database_parser_sqllog::{FileEncodingHint, LogParserBuilder, ParseError, Sqllog};
use std::path::Path;

pub(super) fn open(path: &Path) -> Result<RecordIterator, InputParseError> {
    let records = LogParserBuilder::new(path)
        .encoding_hint(FileEncodingHint::Auto)
        .build()?
        .iter()?;
    Ok(Box::new(records.map(|result| {
        result
            .map(sql_record_to_model)
            .map_err(InputParseError::from)
    })))
}

fn sql_record_to_model(record: Sqllog) -> LogRecord {
    LogRecord {
        ts: record.ts,
        tag: record.tag,
        ep: record.ep,
        sess_id: record.sess_id,
        thrd_id: record.thrd_id,
        username: record.username,
        trxid: record.trxid,
        statement: record.statement,
        appname: record.appname,
        client_ip: record.client_ip,
        sql: record.sql,
        exectime: record.exectime,
        rowcount: record.rowcount,
        exec_id: record.exec_id,
    }
}

impl From<ParseError> for InputParseError {
    fn from(error: ParseError) -> Self {
        Self::SqlLog(error)
    }
}
