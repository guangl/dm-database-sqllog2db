use super::*;

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
fn dropping_early_stops_the_producer() {
    let file = fixture();
    let mut records = export_records(file.path()).unwrap();
    assert!(records.next().is_some());
    drop(records);
    // Removing the fixture after Drop also checks the producer released its file handle.
    file.close().unwrap();
}
