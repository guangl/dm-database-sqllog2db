//! Adapter for the feature-selected DM driver log formats.

use super::{InputParseError, RecordIterator};
use crate::model::LogRecord;
#[cfg(feature = "driver-jdbc")]
use dm_database_driver_log::JdbcEvent;
use dm_database_driver_log::{LogEvent, LogParserBuilder, ParseError};
use std::path::Path;

pub(super) fn open(path: &Path) -> Result<RecordIterator, InputParseError> {
    let records = LogParserBuilder::new(path)
        .encoding_hint(dm_database_driver_log::FileEncodingHint::Auto)
        .build()?
        .iter()?;
    Ok(Box::new(records.map(|result| {
        result
            .map(|event| driver_event_to_model(&event))
            .map_err(InputParseError::from)
    })))
}

fn driver_event_to_model(event: &LogEvent) -> LogRecord {
    match event {
        #[cfg(feature = "driver-jdbc")]
        LogEvent::Jdbc(jdbc) => jdbc_to_model(jdbc),
        #[cfg(feature = "driver-dm-provider")]
        LogEvent::DmProvider(provider) => provider_to_model(provider),
    }
}

#[cfg(feature = "driver-jdbc")]
fn jdbc_to_model(event: &JdbcEvent) -> LogRecord {
    let session_id = event
        .session_id_hex
        .clone()
        .or_else(|| event.conn_id.map(|id| id.to_string()))
        .unwrap_or_default();
    let exec_id = event.exec_id.unwrap_or_default();

    LogRecord {
        ts: event.event_time_text.clone(),
        tag: Some(event.category.to_owned()),
        ep: 0,
        sess_id: session_id,
        thrd_id: event.tid.map_or_else(String::new, |id| id.to_string()),
        username: String::new(),
        trxid: exec_id.to_string(),
        statement: event.method.clone(),
        appname: "dm-jdbc".to_owned(),
        client_ip: String::new(),
        sql: event.raw.clone(),
        exectime: f64_to_f32_ms(event.used_time_ms),
        rowcount: 0,
        exec_id,
    }
}

#[cfg(feature = "driver-dm-provider")]
fn provider_to_model(event: &dm_database_driver_log::DmProviderEvent) -> LogRecord {
    let session_id = event
        .session_id
        .map(|id| id.to_string())
        .or_else(|| event.object_id.clone())
        .or_else(|| event.conn_id.map(|id| id.to_string()))
        .unwrap_or_default();
    let exec_id = event.exec_id.unwrap_or_default();

    LogRecord {
        ts: event.event_time_text.clone(),
        tag: Some(event.category.to_owned()),
        ep: 0,
        sess_id: session_id,
        thrd_id: event.tid.map_or_else(String::new, |id| id.to_string()),
        username: String::new(),
        trxid: exec_id.to_string(),
        statement: event.method.clone(),
        appname: "dm-provider".to_owned(),
        client_ip: String::new(),
        sql: event.sql.clone().unwrap_or_else(|| event.raw.clone()),
        exectime: f64_to_f32_ms(event.used_time_ms),
        rowcount: 0,
        exec_id,
    }
}

fn f64_to_f32_ms(value: Option<f64>) -> f32 {
    let Some(value) = value.filter(|value| value.is_finite()) else {
        return 0.0;
    };
    let clamped = value.clamp(f64::from(f32::MIN), f64::from(f32::MAX));
    #[expect(
        clippy::cast_possible_truncation,
        reason = "value is clamped to the representable f32 range"
    )]
    {
        clamped as f32
    }
}

impl From<ParseError> for InputParseError {
    fn from(error: ParseError) -> Self {
        Self::DriverLog(error)
    }
}
