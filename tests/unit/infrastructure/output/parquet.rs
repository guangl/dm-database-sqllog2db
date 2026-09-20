use super::ParquetExporter;
use crate::config::{ParquetCompression, ParquetExporterConfig};
use crate::exporter::Exporter;
use crate::model::LogRecord;
use arrow_array::{Int32Array, Int64Array, StringArray};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

fn sample_record() -> LogRecord {
    LogRecord {
        ts: "2025-01-15 10:30:28.001".into(),
        tag: Some("SEL".into()),
        username: "TESTUSER".into(),
        sess_id: "0x0001".into(),
        sql: "SELECT 1".into(),
        ..Default::default()
    }
}

#[test]
fn writes_readable_parquet_with_projection() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("out.parquet");
    let config = ParquetExporterConfig {
        file: path.to_string_lossy().into_owned(),
        overwrite: true,
        compression: ParquetCompression::Zstd,
        row_group_rows: 1,
    };
    let mut exporter = ParquetExporter::from_config(&config);
    exporter.ordered_indices = vec![10, 1, 11, 14];
    exporter.initialize().unwrap();
    exporter
        .export_one_normalized(&sample_record(), Some("SELECT ?"))
        .unwrap();
    exporter.finalize().unwrap();

    let mut reader = ParquetRecordBatchReaderBuilder::try_new(File::open(path).unwrap())
        .unwrap()
        .build()
        .unwrap();
    let batch = reader.next().unwrap().unwrap();
    assert_eq!(batch.num_rows(), 1);
    assert_eq!(batch.schema().field(0).name(), "sql");
    assert_eq!(batch.schema().field(1).name(), "ep");
    assert_eq!(batch.schema().field(2).name(), "exec_time_ms");
    assert_eq!(batch.schema().field(3).name(), "normalized_sql");
    assert!(
        !batch
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(0)
            .is_empty()
    );
    assert!(
        batch
            .column(1)
            .as_any()
            .downcast_ref::<Int32Array>()
            .is_some()
    );
    assert!(
        batch
            .column(2)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .is_null(0)
    );
    assert_eq!(
        batch
            .column(3)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(0),
        "SELECT ?"
    );
}

use arrow_array::Array;
use std::fs::File;
