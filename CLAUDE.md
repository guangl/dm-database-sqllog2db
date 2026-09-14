# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build --release

# Test
cargo test
python3 -B -m unittest discover -s tests/python -p 'test_*.py' -v

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

**sqllog2db** parses DaMeng (达梦) database SQL log files and exports them to Parquet or CSV. It streams log records through an optional processing pipeline and writes to a single configured exporter.

### Data Flow

```
Input .log files (sqllogs/)
    ↓ SqllogParser        — discovers files (src/parser.rs)
    ↓ dm-database-parser-sqllog  — parses each line into Sqllog records
    ↓ Pipeline            — optional filters and normalization (src/pipeline/)
    ↓ ExporterManager     — routes to active exporter (src/exporter/)
    ↓ Output (Parquet / CSV)
```

### Key Modules

**Layering:** `cli/` is the thin command layer (arg parsing in `opts.rs`, per-command handlers). `cli/mod.rs` dispatches commands, `cli/runtime.rs` applies config overrides, initializes logging and formats errors. Command-level orchestration lives in top-level domain modules — mirror this split when adding a command (see `stats/` + `cli/stats.rs`).

- **`engine/mod.rs`** — run-command orchestration: resolves inputs, pre-scans transaction filters, builds context, executes the selected plan, and reports results (`engine::run`, called from `cli/mod.rs`)
- **`engine/context.rs`** — shared run configuration and driver result types
- **`engine/prepare.rs`** — input resolution + two-phase pre-scan (collects transaction IDs for filter pre-population) + memory-budget job capping
- **`engine/record.rs`** — record-level filter → normalize → export loop shared by all driver paths
- **`exporter/mod.rs`** — `Exporter` trait + `ExporterManager` factory; only one exporter is active per run (priority: Parquet > CSV)
- **`exporter/csv.rs`** — CSV field serialization; `has_metrics` condition includes `rowcount != 0` to avoid silent data loss
- **`pipeline/mod.rs`** — `LogProcessor` trait + `Pipeline`; `pipeline.is_empty()` enables a zero-overhead fast path when no filters are configured
- **`pipeline/filters/processor.rs`** — `build_pipeline` + `FilterProcessor`; two-pass design: pre-scan finds matching transaction IDs, main pass applies all filters
- **`config/mod.rs`** — all config structs with serde deserialization and validation; `config/template.rs` holds the `init` default-config TOML assets

### Performance Design

- Single-threaded streaming — constant memory regardless of file size
- 1 MiB `BufWriter` + `itoa` crate for zero-allocation CSV formatting
- `pipeline.is_empty()` check in the hot loop avoids filter overhead when disabled
- Binary: LTO (`fat`) + strip + `panic=abort` + `opt-level=3`
- Benchmark: ~5.2M records/sec (synthetic CSV, criterion); ~1.55M records/sec on a real 1.1GB file

### Error Handling

Parse errors are not fatal — they are written to the configured error log file (`[error] file`) and processing continues. Structured errors use `thiserror`; all variants include path/reason context.

### Test organization

All test implementations and test-only helpers live under root `tests/`: Rust unit tests in `tests/unit/` mirror source modules, Cargo integration tests are root-level `.rs` files, and Python tests live in `tests/python/`. Production modules reference unit tests with `#[cfg(test)]` plus `#[path]`; keep private APIs private.

- CSV and Parquet share `engine/sequential.rs`: a common bounded parse-ahead queue, one ordered streaming writer, one output file, progress unless quiet. No file splitting or exporter-specific concurrency.
