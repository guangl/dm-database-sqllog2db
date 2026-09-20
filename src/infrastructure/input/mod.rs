//! Streaming input boundary and bounded prefetching.
//!
//! Parser implementations live behind feature-gated adapters. The rest of the application only
//! sees [`crate::model::LogRecord`], so adding another log format does not change filters,
//! normalization, statistics or exporters.

#[cfg(any(feature = "driver-jdbc", feature = "driver-dm-provider"))]
#[path = "adapters/driver_log.rs"]
mod driver_log;
#[cfg(feature = "sqllog")]
#[path = "adapters/sql_log.rs"]
mod sql_log;

mod resolver;

pub(crate) use resolver::InputResolver;

use crate::model::LogRecord;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Errors emitted at the input boundary.
#[derive(Debug)]
pub(crate) enum InputParseError {
    Io(std::io::Error),
    #[cfg(feature = "sqllog")]
    SqlLog(dm_database_parser_sqllog::ParseError),
    #[cfg(any(feature = "driver-jdbc", feature = "driver-dm-provider"))]
    DriverLog(dm_database_driver_log::ParseError),
    #[allow(dead_code)]
    Unsupported {
        format: &'static str,
        feature: &'static str,
    },
}

impl fmt::Display for InputParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            #[cfg(feature = "sqllog")]
            Self::SqlLog(error) => write!(f, "SQL log: {error}"),
            #[cfg(any(feature = "driver-jdbc", feature = "driver-dm-provider"))]
            Self::DriverLog(error) => write!(f, "driver log: {error}"),
            Self::Unsupported { format, feature } => {
                write!(
                    f,
                    "{format} input support is disabled; enable the `{feature}` feature"
                )
            }
        }
    }
}

impl std::error::Error for InputParseError {}

type Record = Result<LogRecord, InputParseError>;
type RecordIterator = Box<dyn Iterator<Item = Record> + Send>;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputFormat {
    SqlLog,
    DriverLog,
}

/// Open one input through the adapter selected by the first non-empty record.
pub(crate) fn open_log_file(path: &Path) -> Result<RecordIterator, InputParseError> {
    match detect_input_format(path)? {
        InputFormat::SqlLog => {
            #[cfg(feature = "sqllog")]
            {
                sql_log::open(path)
            }
            #[cfg(not(feature = "sqllog"))]
            {
                Err(InputParseError::Unsupported {
                    format: "SQL log",
                    feature: "sqllog",
                })
            }
        }
        InputFormat::DriverLog => {
            #[cfg(any(feature = "driver-jdbc", feature = "driver-dm-provider"))]
            {
                driver_log::open(path)
            }
            #[cfg(not(any(feature = "driver-jdbc", feature = "driver-dm-provider")))]
            {
                Err(InputParseError::Unsupported {
                    format: "driver log",
                    feature: "driver-jdbc or driver-dm-provider",
                })
            }
        }
    }
}

/// Detect the input family without reading the whole file.
fn detect_input_format(path: &Path) -> Result<InputFormat, InputParseError> {
    let file = File::open(path).map_err(InputParseError::Io)?;
    let mut reader = BufReader::new(file);
    let mut line = Vec::new();
    loop {
        line.clear();
        let bytes_read = reader
            .read_until(b'\n', &mut line)
            .map_err(InputParseError::Io)?;
        if bytes_read == 0 {
            return Ok(InputFormat::SqlLog);
        }
        if let Some(first) = line
            .iter()
            .copied()
            .find(|byte| !byte.is_ascii_whitespace())
        {
            if first == b'[' {
                #[cfg(any(feature = "driver-jdbc", feature = "driver-dm-provider"))]
                {
                    return Ok(InputFormat::DriverLog);
                }
                #[cfg(not(any(feature = "driver-jdbc", feature = "driver-dm-provider")))]
                {
                    return Err(InputParseError::Unsupported {
                        format: "driver log",
                        feature: "driver-jdbc or driver-dm-provider",
                    });
                }
            }
            #[cfg(feature = "sqllog")]
            {
                return Ok(InputFormat::SqlLog);
            }
            #[cfg(not(feature = "sqllog"))]
            {
                return Err(InputParseError::Unsupported {
                    format: "SQL log",
                    feature: "sqllog",
                });
            }
        }
    }
}

/// Overlap parsing with export using two bounded batches, retaining input order.
/// Non-regular inputs stay synchronous so cancellation never waits on a blocked pipe.
pub(crate) fn export_records(path: &Path) -> Result<RecordIterator, InputParseError> {
    let records = open_log_file(path)?;
    if !path.is_file() {
        return Ok(records);
    }
    let (sender, receiver) = std::sync::mpsc::sync_channel(2);
    let worker = std::thread::spawn(move || {
        let mut batch = Vec::with_capacity(512);
        let mut bytes = 0;
        for record in records {
            bytes += record.as_ref().map_or(1024, |r| {
                std::mem::size_of_val(r)
                    + r.ts.capacity()
                    + r.sess_id.capacity()
                    + r.thrd_id.capacity()
                    + r.username.capacity()
                    + r.trxid.capacity()
                    + r.statement.capacity()
                    + r.appname.capacity()
                    + r.client_ip.capacity()
                    + r.tag.as_ref().map_or(0, String::capacity)
                    + r.sql.capacity()
            });
            batch.push(record);
            if batch.len() >= 512 || bytes >= 1024 * 1024 {
                if sender
                    .send(std::mem::replace(&mut batch, Vec::with_capacity(512)))
                    .is_err()
                {
                    return;
                }
                bytes = 0;
            }
        }
        if !batch.is_empty() {
            let _ = sender.send(batch);
        }
    });
    Ok(Box::new(PrefetchedRecords {
        receiver: Some(receiver),
        worker: Some(worker),
        batch: Vec::new().into_iter(),
    }))
}

struct PrefetchedRecords {
    receiver: Option<std::sync::mpsc::Receiver<Vec<Record>>>,
    worker: Option<std::thread::JoinHandle<()>>,
    batch: std::vec::IntoIter<Record>,
}

impl Iterator for PrefetchedRecords {
    type Item = Record;

    fn next(&mut self) -> Option<Record> {
        if let Some(record) = self.batch.next() {
            return Some(record);
        }
        if let Ok(batch) = self.receiver.as_ref()?.recv() {
            self.batch = batch.into_iter();
            self.batch.next()
        } else {
            if let Some(worker) = self.worker.take()
                && let Err(panic) = worker.join()
            {
                std::panic::resume_unwind(panic);
            }
            None
        }
    }
}

impl Drop for PrefetchedRecords {
    fn drop(&mut self) {
        self.receiver.take();
        if let Some(worker) = self.worker.take() {
            let result = worker.join();
            if !std::thread::panicking()
                && let Err(panic) = result
            {
                std::panic::resume_unwind(panic);
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/input/mod.rs"]
mod tests;
