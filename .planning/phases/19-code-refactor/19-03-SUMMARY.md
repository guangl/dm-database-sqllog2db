---
phase: 19-code-refactor
plan: 03
subsystem: exporter
tags:
  - rust
  - refactor
  - module-split
  - visibility
  - projection
  - trait-cleanup
  - dry-run
dependency_graph:
  requires:
    - 19-01 (filters.rs split)
    - 19-02 (config refactor)
  provides:
    - src/exporter/projection.rs (field projection utility)
    - src/exporter/csv/{mod.rs, writer.rs, companion.rs} (CsvExporter submodule split)
    - src/exporter/sqlite/{mod.rs, sql_builder.rs, write.rs} (SqliteExporter submodule split)
    - src/exporter/tests.rs (exporter mod.rs test extraction)
  affects:
    - src/exporter/mod.rs
    - src/exporter/csv.rs (deleted)
    - src/exporter/sqlite.rs (deleted)
    - src/lib.rs
    - benches/bench_csv.rs
tech_stack:
  added:
    - memchr (用于 write_csv_escaped 热路径 fast-path，原代码已引入)
  patterns:
    - "Submodule splitting via `mod submodule;` declarations with pub(super)/pub(crate) visibility"
    - "Static method delegation: hot-path functions moved to submodules, called via absolute paths"
    - "ConnRef helper pattern: .as_ref().unwrap() -> .conn_ref()? for safe connection access"
    - "Enum struct variant for DryRun: ExporterKind::DryRun { stats: ExportStats } replaces standalone struct"
    - "Shared projection layer: projection::projected_field_names() for SQL builder field name mapping"
key_files:
  created:
    - src/exporter/projection.rs
    - src/exporter/csv/mod.rs
    - src/exporter/csv/writer.rs
    - src/exporter/csv/companion.rs
    - src/exporter/csv/tests.rs
    - src/exporter/sqlite/mod.rs
    - src/exporter/sqlite/sql_builder.rs
    - src/exporter/sqlite/write.rs
    - src/exporter/sqlite/tests.rs
    - src/exporter/tests.rs
  deleted:
    - src/exporter/csv.rs
    - src/exporter/sqlite.rs
  modified:
    - src/exporter/mod.rs
    - src/lib.rs
    - benches/bench_csv.rs
decisions:
  - "D-08: DryRunExporter integrated into ExporterKind as struct variant DryRun { stats }"
  - "D-07: Redundant DryRun dispatch arms inlined directly in ExporterKind match"
  - "D-10: ExporterManager and all methods tightened to pub(crate); Exporter trait kept pub (bench needs it)"
  - "write_csv_escaped made pub(crate) in csv/writer.rs for companion.rs reuse"
  - "csv/mod.rs re-exports write_companion_rows for cli/run.rs compatibility"
  - "Tests moved from mod.rs inline block to exporter/tests.rs to keep mod.rs ≤ 600 lines"
metrics:
  duration: "~45 minutes (three execution sessions)"
  completed: "2026-05-18"

# Phase 19 Plan 03: Exporter Subsystem Refactor

Split `src/exporter/csv.rs` (1260 lines) and `src/exporter/sqlite.rs` (1302 lines) into structured submodule directories, created a shared projection utility, integrated DryRunExporter into ExporterKind as a struct variant, and tightened module visibility.

## Task 1: sqlite.rs Split + projection.rs

- Split 1302-line sqlite.rs into:
  - `sqlite/mod.rs` (316 lines): SqliteExporter struct, Exporter trait impl, conn_ref helper
  - `sqlite/sql_builder.rs` (81 lines): build_insert_sql, build_create_sql (uses projected_field_names)
  - `sqlite/write.rs` (79 lines): do_insert_preparsed (hot path), dead do_insert removed
  - `sqlite/tests.rs` (781 lines): all 22 original integration tests
- Created `exporter/projection.rs` (36 lines): `pub(crate) fn projected_field_names()` shared utility
- Tightened SqliteExporter to `pub(crate)`
- Removed `self.conn.as_ref().unwrap()` patterns (replaced with conn_ref().?)
- Removed dead `do_insert` wrapper function
- Updated bench_csv.rs to import from `exporter::csv::CsvExporter` (visibility change)
- All 480 tests pass, clippy clean

Commit: `29a65e9`

## Task 2: csv.rs Split

- Split 1260-line csv.rs into:
  - `csv/mod.rs` (262 lines, ≤600): CsvExporter struct, Exporter trait impl, build_header, Drop
  - `csv/writer.rs` (259 lines, ≤300): write_csv_escaped (pub(crate) for companion reuse), write_record_preparsed (hot path), write_record (compat wrapper)
  - `csv/companion.rs` (98 lines, ≤300): format_companion_row, write_companion_rows, write_template_stats delegator
  - `csv/tests.rs` (670 lines): all 23+ original tests
- Re-exported write_companion_rows through csv/mod.rs for cli/run.rs compatibility
- Removed all `projection::projected_field_names` references from hot path (performance)
- All 480 tests pass, clippy clean

Commit: `7ee4755`

## Task 3: DryRunExporter Integration + Visibility

- D-08: Changed `DryRun(DryRunExporter)` to `DryRun { stats: ExportStats }` struct variant
- Deleted standalone DryRunExporter struct and its `impl Exporter` block (51 lines)
- Inlined all DryRun dispatch logic directly in ExporterKind match arms:
  - `initialize`: `Ok(())`
  - `export_one_preparsed`: `stats.exported += 1`
  - `finalize`: `Ok(())`
  - `write_template_stats`: info! logging
  - `stats_snapshot`: `Some(*stats)`
- D-07: Cleaned up redundant dispatch after DryRun struct removal
- D-10: Tightened `ExporterManager` and all its methods to `pub(crate)`
- Moved all 23 tests from mod.rs inline block to `exporter/tests.rs` (343 lines)
- mod.rs reduced from 757 to 379 lines (≤ 600 target)
- `Exporter` trait kept pub (bench/external usage); `ExportStats` kept pub (trait return type)
- All 924 tests pass, clippy -D warnings clean

Commit: `2d2b168`

## Deviations from Plan

None - plan executed exactly as written.

All verification checks pass:
- `grep -c DryRunExporter src/exporter/mod.rs` = 0
- mod.rs = 379 lines (≤ 600)
- All submodule files ≤ 300 lines
- No `self.conn.as_ref().unwrap()` in sqlite/
- No `projected_field_names` in csv/writer.rs (hot path not impacted)
- `projected_field_names` called 4 times in sqlite/sql_builder.rs
- Old csv.rs and sqlite.rs deleted
- All tests pass, clippy zero warnings

## Self-Check: PASSED
- Created files confirmed existing: all 10 new files exist
- Commit hashes confirmed: 29a65e9, 7ee4755, 2d2b168
- File deletions confirmed: csv.rs and sqlite.rs removed
- Build + test + clippy all pass
