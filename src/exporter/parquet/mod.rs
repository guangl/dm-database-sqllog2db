//! Parquet exporter: Arrow record batches written as compressed Parquet row groups.

mod exporter;

#[cfg(test)]
mod tests;

pub(crate) use exporter::ParquetExporter;
