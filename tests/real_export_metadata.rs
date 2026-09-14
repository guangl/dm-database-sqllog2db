//! Opt-in verification of exports from a real input directory.
#[test]
#[ignore = "requires REAL_EXPORT_DIR containing output.parquet"]
fn parquet_metadata() {
    use parquet::file::reader::{FileReader, SerializedFileReader};
    let root = std::env::var("REAL_EXPORT_DIR").unwrap();
    let reader =
        SerializedFileReader::new(std::fs::File::open(format!("{root}/output.parquet")).unwrap())
            .unwrap();
    let metadata = reader.metadata();
    let rows = metadata.file_metadata().num_rows();
    let groups = metadata.num_row_groups();
    assert!(rows > 0);
    let sum: i64 = (0..groups).map(|i| metadata.row_group(i).num_rows()).sum();
    assert_eq!(rows, sum);
    let file = std::fs::File::open(format!("{root}/output.parquet")).unwrap();
    let batches = parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap()
        .with_batch_size(8192)
        .build()
        .unwrap();
    let mut decoded_rows = 0_i64;
    for batch in batches {
        decoded_rows += i64::try_from(batch.unwrap().num_rows()).unwrap();
    }
    assert_eq!(decoded_rows, rows);
    std::fs::write(
        format!("{root}/parquet-metadata.json"),
        format!("{{\"rows\":{rows},\"row_groups\":{groups}}}\n"),
    )
    .unwrap();
    println!("Parquet decoded rows={rows}, row_groups={groups}");
}
