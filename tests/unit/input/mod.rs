#[cfg(any(
    feature = "sqllog",
    feature = "driver-jdbc",
    feature = "driver-dm-provider"
))]
use super::*;

#[cfg(feature = "sqllog")]
fn fixture() -> tempfile::NamedTempFile {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new().unwrap();
    writeln!(file, "malformed record").unwrap();
    for i in 0..4096 {
        writeln!(file, "2025-01-15 10:30:28.001 (EP[0] sess:0x1 user:U trxid:{i} stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT {i}. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: {i}.").unwrap();
    }
    file
}

#[test]
#[cfg(feature = "sqllog")]
fn prefetch_preserves_records_errors_and_order() {
    let file = fixture();
    let direct: Vec<_> = open_log_file(file.path())
        .unwrap()
        .map(|r| format!("{r:?}"))
        .collect();
    let prefetched: Vec<_> = export_records(file.path())
        .unwrap()
        .map(|r| format!("{r:?}"))
        .collect();
    assert_eq!(direct.len(), 4097);
    assert!(direct[0].starts_with("Err("));
    assert_eq!(direct, prefetched);
}

#[test]
#[cfg(feature = "sqllog")]
fn dropping_early_stops_the_producer() {
    let file = fixture();
    let mut records = export_records(file.path()).unwrap();
    assert!(records.next().is_some());
    drop(records);
    // Removing the fixture after Drop also checks the producer released its file handle.
    file.close().unwrap();
}

#[test]
#[cfg(feature = "driver-jdbc")]
fn jdbc_driver_logs_are_adapted_to_the_shared_record_model() {
    use std::io::Write;

    let mut file = tempfile::NamedTempFile::new().unwrap();
    writeln!(
        file,
        "[INFO - 2026-09-16 17:45:19.763] tid:119 - [worker] {{ conn-3, pstmt-854 }} executeQuery(): rs-2216; [USED TIME]: 8.5ms; [EXEC_ID]: 19010657;"
    )
    .unwrap();

    let record = open_log_file(file.path()).unwrap().next().unwrap().unwrap();
    assert_eq!(record.ts, "2026-09-16 17:45:19.763");
    assert_eq!(record.tag.as_deref(), Some("execute"));
    assert_eq!(record.sess_id, "3");
    assert_eq!(record.thrd_id, "119");
    assert_eq!(record.statement, "executeQuery");
    assert_eq!(record.exec_id, 19_010_657);
    assert!((record.exectime - 8.5).abs() < f32::EPSILON);
    assert!(record.sql.contains("executeQuery"));
}

#[test]
#[cfg(feature = "driver-dm-provider")]
fn dm_provider_logs_are_adapted_to_the_shared_record_model() {
    use std::io::Write;

    let mut file = tempfile::NamedTempFile::new().unwrap();
    writeln!(
        file,
        "[SQL - 2026-09-12 08:55:30.193] tid:68 (IsBackground-True) {{ conn-2095 (sessId:281421579449976), command-4579 }} ExecuteDbDataReader(CommandBehavior) [SQL]: SELECT 1 [USED TIME]: 2ms; [EXEC_ID]: 908131301;"
    )
    .unwrap();

    let record = open_log_file(file.path()).unwrap().next().unwrap().unwrap();
    assert_eq!(record.ts, "2026-09-12 08:55:30.193");
    assert_eq!(record.tag.as_deref(), Some("execute"));
    assert_eq!(record.sess_id, "281421579449976");
    assert_eq!(record.thrd_id, "68");
    assert_eq!(record.statement, "ExecuteDbDataReader");
    assert_eq!(record.exec_id, 908_131_301);
    assert!((record.exectime - 2.0).abs() < f32::EPSILON);
    assert_eq!(record.sql, "SELECT 1");
}

#[test]
#[cfg(feature = "sqllog")]
fn blank_files_keep_the_sql_log_parser_path() {
    let file = tempfile::NamedTempFile::new().unwrap();
    assert!(open_log_file(file.path()).unwrap().next().is_none());
}
