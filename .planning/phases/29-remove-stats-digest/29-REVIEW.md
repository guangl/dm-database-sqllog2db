---
phase: 29-remove-stats-digest
reviewed: 2026-05-20T12:00:00Z
depth: standard
files_reviewed: 36
files_reviewed_list:
  - benches/bench_csv.rs
  - benches/bench_filters.rs
  - benches/bench_sqlite.rs
  - Cargo.toml
  - docs/architecture.md
  - README.md
  - src/cli/init.rs
  - src/cli/mod.rs
  - src/cli/opts.rs
  - src/cli/run/mod.rs
  - src/cli/run/parallel.rs
  - src/cli/run/processor.rs
  - src/cli/run/tests.rs
  - src/cli/show_config.rs
  - src/config/apply_one.rs
  - src/config/exporter.rs
  - src/config/logging.rs
  - src/config/mod.rs
  - src/config/sqllog.rs
  - src/config/validate.rs
  - src/error.rs
  - src/exporter/csv/mod.rs
  - src/exporter/csv/tests.rs
  - src/exporter/csv/writer.rs
  - src/exporter/mod.rs
  - src/exporter/projection.rs
  - src/exporter/sqlite/mod.rs
  - src/exporter/sqlite/sql_builder.rs
  - src/exporter/sqlite/tests.rs
  - src/exporter/sqlite/write.rs
  - src/exporter/tests.rs
  - src/lang.rs
  - src/lib.rs
  - src/main.rs
  - src/pipeline/mod.rs
  - src/pipeline/normalizer.rs
  - src/pipeline/filters/mod.rs
  - src/pipeline/filters/types.rs
  - src/pipeline/filters/compiled.rs
  - tests/integration.rs
findings:
  critical: 1
  warning: 5
  info: 3
  total: 9
status: issues_found
---

# Phase 29: Code Review Report

**Reviewed:** 2026-05-20T12:00:00Z
**Depth:** standard
**Files Reviewed:** 36 (28 source, 8 non-source)
**Status:** issues_found

## Summary

Phase 29 removed the `stats` and `digest` CLI subcommands, removed `fingerprint.rs`, and migrated `normalize_template` to `normalizer.rs`. The code migration itself (normalizer.rs, cli/mod.rs, cli/opts.rs, main.rs) is structurally sound -- no dangling imports, no missing module references, no compilation regressions.

However, two significant findings were discovered:

1. **Critical: `benches/bench_filters.rs` uses the wrong TOML section name** for all filter benchmark configurations. Every scenario uses `[features.filters]` which is silently ignored by serde because the current `Config` struct has no `features` field. All 7 benchmarks measure the no-filter fast path, producing invalid benchmark results.

2. **Multiple stale documentation references** to the removed `stats`/`digest` commands in README.md and docs/architecture.md will cause confusion for users following the examples.

The remaining findings are dead code preservation (`open_connection_only`, `normalize_template`, `FileError::ReadFailed`) and gaps in test coverage.

## Critical Issues

### CR-01: bench_filters.rs uses `[features.filters]` instead of `[filter]` -- all benchmarks silently test no-filter path

**File:** `benches/bench_filters.rs:76,91,107,122,136,151`

**Issue:** All seven benchmark scenarios construct TOML configurations using `[features.filters]` as the section name for filter settings. The current `Config` struct in `src/config/mod.rs` has a `filter: Option<FiltersFeature>` field but no `features` field. Serde silently ignores unknown table keys (no `deny_unknown_fields` on Config). Consequently:

- `[features.filters] enable = true` is silently ignored -- `cfg.filter` remains `None`
- `validate_and_compile()` returns `Ok(None)` for every scenario
- `handle_run` builds an empty pipeline every time
- All 7 benchmarks measure only the no-filter fast path, not the intended filter overhead

The affected config builder functions are:
- `cfg_pipeline_passthrough` (line 73)
- `cfg_trxid_small` (line 87)
- `cfg_trxid_large` (line 103)
- `cfg_indicator_prescan` (line 119)
- `cfg_exclude_passthrough` (line 134)
- `cfg_exclude_active` (line 148)

**Impact:** The entire bench_filters benchmark suite produces invalid results. This is especially dangerous because the benchmark outputs look plausible (they complete successfully) but measure nothing close to what their names imply. `pipeline_passthrough`, `trxid_small`, `trxid_large`, `indicator_prescan`, `exclude_passthrough`, and `exclude_active` all produce identical (or near-identical) results to `no_pipeline`. Any regression in the filter path would be invisible.

**Fix:** Replace `[features.filters]` with `[filter]` and update the filter configurations to use the current nested sub-table format or the backward-compatible flat format but under the correct section name. Example for `cfg_pipeline_passthrough`:

```rust
fn cfg_pipeline_passthrough(sqllog_dir: &Path, bench_dir: &Path) -> Config {
    let toml = format!(
        "{base}
[filter]
enable = true
start_ts = \"2000-01-01\"
",
        base = base_toml(sqllog_dir, bench_dir)
    );
    toml::from_str(&toml).unwrap()
}
```

Note: The old flat format IS supported by `FiltersFeature` via `RawFiltersFeature` for backward compatibility, but the section name must be `[filter]`, not `[features.filters]`. Each benchmark config must be verified to use the correct key from the old flat format (e.g., `exclude_usernames` stays as-is once under `[filter]`; no nested sub-table needed for backward compat).

## Warnings

### WR-01: README.md contains stale references to removed `stats` and `digest` commands

**File:** `README.md:49,131-134,177`

**Issue:** Three locations reference CLI commands that were removed in Phase 29:

- Line 49: Lists `stats` (按文件统计记录数) and `digest` (SQL 指纹聚合) as available CLI commands
- Lines 131-134: Provides usage examples including `sqllog2db stats -c config.toml --top 10` and `sqllog2db digest -c config.toml --sort exec --top 20`
- Line 177: Mentions `sqllog2db stats` command for SVG chart generation

These examples will produce `error: unrecognized subcommand` for any user who follows them.

**Fix:** Remove the `stats`/`digest` bullet from line 49. Remove or replace the examples at lines 131-134 with valid commands (`show-config`, `validate`). Remove the chart/SVG paragraph at line 177 which was gated behind the `stats` command.

### WR-02: docs/architecture.md contains stale references to removed modules

**File:** `docs/architecture.md:26,51-52`

**Issue:**
- Line 26: "stats 和 digest 命令读取已导出的输出文件" -- both commands no longer exist
- Lines 51-52: Lists `stats.rs` and `digest.rs` as existing source files under `src/cli/`

**Fix:** Remove the `stats`/`digest` mention from line 26. Remove the bullet points listing `stats.rs` and `digest.rs` from lines 51-52.

### WR-03: `SqliteExporter::open_connection_only()` is dead code

**File:** `src/exporter/sqlite/mod.rs:131`

**Issue:** The function `open_connection_only()` is annotated with `#[allow(dead_code)]`. It was part of the stats/digest template analysis path that was removed in Phase 29. The function opens a SQLite connection and sets PRAGMAs without creating the main data table. Since the stats/digest path is gone, this function has no callers.

**Fix:** Remove the function and its `#[allow(dead_code)]` annotation. If it is likely to be needed in the future, add a documented TODO explaining the intended use case.

### WR-04: `normalize_template()` is dead code in normalizer.rs

**File:** `src/pipeline/normalizer.rs:462`

**Issue:** The `normalize_template` function (including all supporting functions: `scan_sql_bytes`, `dispatch_byte`, `handle_quote`, `handle_line_comment`, `handle_block_comment`, `handle_word`, `try_fold_in_list`, `skip_quoted`, `is_subquery`, `is_keyword`, `is_ident_byte`, `prev_is_ident_byte`, plus the `NEEDS_SPECIAL_NORM` lookup table) is annotated with `#[allow(dead_code)]`. These ~280 lines of code were migrated from `fingerprint.rs` to preserve them, but the template analysis pipeline they served was also removed in Phase 29. The extensive test suite (lines 944-1021 of normalizer.rs) tests dead functionality.

**Fix:** If the template pipeline is not planned for restoration, remove `normalize_template` and all supporting private functions plus the associated tests. If it IS planned for restoration, remove `#[allow(dead_code)]` and add a clear TODO comment explaining the intended re-integration point.

### WR-05: No test coverage for pre-compiled filter path

**File:** `src/cli/run/tests.rs`

**Issue:** All tests in this file pass `None` for the `compiled_filters` argument to `handle_run` (the 8th parameter introduced together with `validate_and_compile`). The pre-compiled path -- where callers explicitly compile filters via `cfg.validate_and_compile()` and pass `Some((CompiledMetaFilters, CompiledSqlFilters))` -- has zero test coverage in the unit tests. A regression in the pre-compiled path (e.g., `recompile_meta_if_needed` modifying filter state incorrectly, or `build_pipeline` handling pre-compiled meta filters differently) would go undetected.

**Fix:** Add at least one test that exercises the explicit pre-compiled path:

```rust
let compiled_filters = cfg.validate_and_compile().unwrap();
handle_run(&cfg, None, true, true, &interrupted, 80, 1, compiled_filters).unwrap();
```

## Info

### IN-01: `FileError::ReadFailed` dead code (scheduled for Phase 32)

**File:** `src/error.rs:59`

**Issue:** `FileError::ReadFailed` variant is annotated `#[allow(dead_code)]` with a TODO comment scheduling cleanup for Phase 32. This is known tracked debt. Phase 29 would have been a natural cleanup point since stats/digest were its likely consumers.

### IN-02: Migration comments reference removed `fingerprint.rs`

**File:** `src/pipeline/normalizer.rs:460,944`

**Issue:** Two comments document the migration history:
- Line 460: "原位于 `fingerprint.rs`，为保留模板管道功能迁移至此。"
- Line 944: "// ---- normalize_template 测试（从 fingerprint.rs 迁移）"

These are accurate historical notes but refer to a file that no longer exists in the codebase.

### IN-03: Inconsistent import paths in validate.rs

**File:** `src/config/validate.rs:49,70`

**Issue:** Two methods use different paths for the same type:
- `validate_and_compile` (line 49): `crate::pipeline::CompiledMetaFilters`
- `validate_filter` (line 70): `crate::pipeline::filters::CompiledMetaFilters`

Both resolve to the same type (re-exported in `pipeline/mod.rs`). Prefer the shorter path or add a `use` import for consistency.

---

_Reviewed: 2026-05-20T12:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
