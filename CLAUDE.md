# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build --release

# Test
cargo test

# Lint (must pass with no warnings)
cargo clippy --all-targets -- -D warnings

# Format
cargo fmt

# CLI usage
cargo run -- init -o config.toml --force
cargo run -- validate -c config.toml
cargo run -- run -c config.toml
```

## Architecture

**sqllog2db** parses DaMeng (达梦) database SQL log files and exports them to CSV or SQLite. It streams log records through an optional processing pipeline and writes to a single configured exporter.

### Data Flow

```
Input .log files (sqllogs/)
    ↓ SqllogParser        — discovers files (src/parser.rs)
    ↓ dm-database-parser-sqllog  — parses each line into Sqllog records
    ↓ Pipeline            — optional filters and normalization (src/pipeline/)
    ↓ ExporterManager     — routes to active exporter (src/exporter/)
    ↓ Output (CSV / SQLite)
```

### Key Modules

- **`cli/run/mod.rs`** — main orchestration: loads config, builds pipeline, pre-scans for transaction filters, streams records file by file
- **`cli/run/parallel.rs`** — multi-file parallel parse path for CSV export (rayon); parse errors logged via `log::warn!`
- **`cli/run/sqlite_parallel.rs`** — multi-file parallel parse path for SQLite export (rayon); parse errors logged via `log::warn!`
- **`cli/run/prescan.rs`** — two-phase pre-scan: collects transaction IDs for filter pre-population; logs file-open and non-UTF8 path errors
- **`exporter/mod.rs`** — `Exporter` trait + `ExporterManager` factory; only one exporter is active per run (priority: CSV > SQLite)
- **`exporter/csv/writer.rs`** — CSV field serialization; `has_metrics` condition includes `rowcount != 0` to avoid silent data loss
- **`pipeline/mod.rs`** — `LogProcessor` trait + `Pipeline`; `pipeline.is_empty()` enables a zero-overhead fast path when no filters are configured
- **`pipeline/filters/mod.rs`** — two-pass design: pre-scan finds matching transaction IDs, main pass applies all filters
- **`config/mod.rs`** — all config structs with serde deserialization and validation

### Performance Design

- Single-threaded streaming — constant memory regardless of file size
- Multi-file parallel parse path for CSV export via rayon (`parallel.rs`) and SQLite export (`sqlite_parallel.rs`)
- 16MB `BufWriter` + `itoa` crate for zero-allocation CSV formatting
- `pipeline.is_empty()` check in the hot loop avoids filter overhead when disabled
- Binary: LTO (`fat`) + strip + `panic=abort` + `opt-level=3`
- Benchmark: ~5.2M records/sec (synthetic CSV, criterion); ~1.55M records/sec on a real 1.1GB file

### Error Handling

Parse errors are not fatal — they are written to the configured error log file (`[error] file`) and processing continues. Structured errors use `thiserror`; all variants include path/reason context.
