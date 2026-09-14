use super::*;

#[test]
fn test_parquet_defaults_and_validation() {
    let cfg = ParquetExporterConfig::default();
    assert_eq!(cfg.compression, ParquetCompression::Zstd);
    assert_eq!(cfg.row_group_rows, 65_536);
    assert!(cfg.validate().is_ok());

    let invalid = ParquetExporterConfig {
        row_group_rows: 0,
        ..cfg
    };
    assert!(invalid.validate().is_err());
}

#[test]
fn removed_csv_split_setting_is_rejected() {
    let error = toml::from_str::<crate::config::Config>(
        "[exporter.csv]\nfile = 'out.csv'\nmax_rows_per_file = 10",
    )
    .unwrap_err();
    assert!(error.to_string().contains("max_rows_per_file"));
}
