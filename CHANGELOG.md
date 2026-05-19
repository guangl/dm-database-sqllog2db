# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.4] - 2026-05-18

### Changed

- **Nested sub-table config model**: `[filter.include]`, `[filter.exclude]`, `[template]`, `[charts]` moved to top-level TOML sections (v1.4 format). Old flat format supported through `RawFiltersFeature` intermediate struct and serde alias for backward compatibility.
- **Config validation**: `validate_and_compile()` validates the final form and rejects legacy layouts.
- **Module restructuring**: 5 large files split into focused modules for better maintainability.
- **ExporterManager tightened**: visibility reduced to `pub(crate)`.

### Added

- **Property-based testing**: `proptest` integration for filter pipeline and config validation.
- **Test coverage**: 933 tests, ~74% line coverage.

---

## [1.3] - 2026-05-17

### Added

- **SQL template normalization engine** (`normalize_template`): strips comments, folds IN-list values, uppercases keywords, collapses whitespace. Produces stable template keys from structurally identical queries with different parameter values.
- **TemplateAggregator**: streaming statistics engine that counts occurrences per template, accumulates execution time distribution via `hdrhistogram` (compact ~24 KB per template), and records first/last timestamps alongside a representative example SQL.
- **Dual-stat output**: aggregated template data written to both a CSV summary file and a dedicated SQLite table (`sql_templates`) in a single run.
- **Four SVG chart types**: frequency bar (top-N templates by occurrence), latency histogram (execution time distribution per template), trend line (normalized template frequency over time), and user pie (proportional share of queries by user). Rendered via plotters with SVG-only backend -- no system fonts or image libraries required.
- **8 new config fields**: `[template]` and `[charts]` TOML sections with per-chart type toggles and output directory.

### Performance

- **Parallel CSV map-reduce merge**: `merge()` function eliminates lock contention by merging per-thread CSVs in a single pass.
- Benchmark: ~5.2M records/sec CSV (synthetic), ~1.55M records/sec on a real 1.1 GB file.

---

## [1.2.1] - 2026-05-15

### Fixed

- Minor bug fixes and improvements discovered during v1.2 deployment.

---

## [1.2] - 2026-05-15

### Added

- **Exclude filters** (FILTER-03): record-level exclude filters with OR-veto semantics integrated into `CompiledMetaFilters`.
- **Unified filter interface**: `validate_and_compile()` eliminates double `Regex::new()` calls.

### Changed

- **Cargo features removed**: `csv`, `jsonl`, `sqlite`, `filters`, `replace_parameters`, `full` features removed. All functionality always compiled into the binary.
- **CLI startup optimization**: parallel feature compilation via `rayon` removed from prescan; `binrw` dependency removed.

### Performance

- Hot-path optimization based on PERF-10 gate analysis. No blind optimization -- every change verified by criterion benchmarks.
- CSV: ~2.13M records/sec, SQLite: ~1.11M records/sec.

### Fixed

- **SQLite technical debt** (DBUS-01/02/03): prepared statement caching, transaction handling, and PRAGMA tuning.
- **Nyquist audit remediation**: improved test coverage and validation gaps.

> **Note:** v1.1 profiling and benchmarking work laid the foundation for v1.2 performance improvements. v1.1 functionality is fully merged into v1.2.

---

## [1.0.0] - 2026-04-18

### Migration Note

This is the first stable release. Upgrading from 0.x:

- Configuration format changed from array `[[exporter.*]]` to single `[exporter.*]` sections.
- JSONL exporter removed; migrate to CSV or SQLite.
- Error logging changed from separate error log file to `log::trace!()` in application log.
- `--set` flag replaces direct config file edits for one-off overrides.

### Added

- **Regular expression multi-field filtering**: AND semantics for include + OR-veto for exclude.
- **Field projection via `ordered_indices`**: exact column order and subset selection.
- **FieldMask-based output field control**.
- **Initial CLI**: `init`, `validate`, `run`, `stats`, `digest`, `show-config`, `man`, `completions`.
- **690+ tests** with comprehensive coverage.

---

## [0.x] - 2025-11 to 2026-04

The 0.x series (0.1.0 through 0.10.7) covered the initial development of sqllog2db:

- Streaming SQL log parsing with multi-file, directory, and glob input modes
- Multi-exporter architecture (CSV, JSONL, SQLite, DuckDB, PostgreSQL, Oracle, DM) -- later simplified to CSV + SQLite
- Feature flags for conditional compilation (`[csv]`, `[jsonl]`, `[sqlite]`, `[filters]`) -- later removed for unified binary
- CLI commands: `init`, `validate`, `run`, `stats`, `digest`, `show-config`, `completions`, `man`
- GB18030 encoding support, `replace_parameters` SQL normalization
- Error logging, progress bar, exit codes, graceful shutdown (SIGINT)
- Performance optimization: mmap + SIMD parser, `itoa` zero-alloc CSV formatting, batch writer, pipeline fast path, SQLite prepared statement caching
- Architecture simplification: single-exporter mode, JSONL removal, Cargo features removal, module restructuring
- Peak memory reduced from 2.42GB to ~179MB (-92.6%) through streaming and zero-copy design
- Extensive test coverage, CI with clippy, coverage gates, and performance benchmarks

See git history for full details.

[1.4]: https://github.com/guangl/sqllog2db/releases/tag/v1.4
[1.3]: https://github.com/guangl/sqllog2db/releases/tag/v1.3
[1.2.1]: https://github.com/guangl/sqllog2db/releases/tag/v1.2.1
[1.2]: https://github.com/guangl/sqllog2db/releases/tag/v1.2
[1.0.0]: https://github.com/guangl/sqllog2db/releases/tag/v1.0
[0.x]: https://github.com/guangl/sqllog2db/releases/tag/v0.10.7
