//! CSV exporter：struct 与构造在 exporter.rs，trait impl 在 impls.rs，序列化在 writer.rs。

pub(crate) mod writer;

mod exporter;
mod impls;

#[cfg(test)]
mod tests;

pub use exporter::CsvExporter;
