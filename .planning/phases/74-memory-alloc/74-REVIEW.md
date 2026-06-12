---
phase: 74-memory-alloc
reviewed: 2026-06-11T00:00:00Z
depth: standard
files_reviewed: 3
files_reviewed_list:
  - src/pipeline/normalizer.rs
  - src/cli/run/tests.rs
  - src/exporter/csv/exporter.rs
findings:
  critical: 0
  warning: 3
  info: 3
  total: 6
status: issues_found
---

# Phase 74: Code Review Report

**Reviewed:** 2026-06-11T00:00:00Z
**Depth:** standard
**Files Reviewed:** 3
**Status:** issues_found

## Summary

Three files from the phase-74 memory allocation optimisation were reviewed: the core normalizer with its `ParamBuffer`/`Arc<Vec<ParamValue>>` design, the CSV exporter struct and `open_for_write` helper, and the integration test suite. No critical (data-loss or security) defects were found. Three warnings and three informational issues were identified. The most actionable finding is misleading per-file error logging in `process_log_file` (hardcoded zero errors) and a silent file-truncation inconsistency in `from_config`.

Cross-file analysis also touched `src/cli/run/processor.rs`, `src/cli/run/collector.rs`, `src/cli/run/prescan.rs`, and `src/exporter/csv/writer.rs` to verify callers.

## Warnings

### WR-01: Per-file log always reports "0 errors" even when export errors occurred

**File:** `src/cli/run/processor.rs:236`

**Issue:** `log_file_result` is called with the `errors_in_file` argument hardcoded to `0`. The actual error counts are accumulated in `file_stats` (via `file_stats.add_export_error()` and `file_stats.set_fatal()`), which is returned to the caller and merged into run-level stats correctly. However, the per-file log line and progress-bar message always show `0 errors` regardless of how many export failures occurred during that file's processing. This makes the debug log misleading during troubleshooting.

**Fix:**
```rust
// Replace the hardcoded 0 with the actual error count from file_stats:
log_file_result(
    pb, show_progress, file_path, file_index, total_files,
    records_in_file,
    file_stats.total_errors as usize,  // was: 0
    elapsed,
);
```

---

### WR-02: TOCTOU race in `collect_log_file` — deleted file silently produces empty results

**File:** `src/cli/run/collector.rs:18-30`

**Issue:** `collect_log_file` checks `file.exists()` (line 18) and then opens the file via `AsyncLogParser::new(file).parse()` (line 25). If the file is deleted in the window between the two operations, `parse()` returns an `Err`, which the code handles with `return Ok((Vec::new(), ErrorStats::default()))` — silently producing zero records rather than the `Err(ParserError::InvalidPath)` the code comment claims. The comment on line 17 states: "so IO errors are distinct from parse errors" — but this guarantee breaks for the file-disappears case.

**Fix:** Either remove the `file.exists()` pre-check and let the `AsyncLogParser` error propagate distinctly, or match on the inner error type to distinguish `InvalidPath` from other IO failures:
```rust
// Option A: remove exists() check entirely, match parse error kind:
let records = AsyncLogParser::new(file).parse().await.map_err(|e| {
    Error::Parser(ParserError::InvalidPath {
        path: file.to_path_buf(),
        reason: e.to_string(),
        line_number: None,
    })
})?;
```

---

### WR-03: `overwrite=false, append=false` silently truncates existing CSV file

**File:** `src/exporter/csv/exporter.rs:56-64`

**Issue:** `CsvExporter::from_config` only sets `WriteMode::Append` when `config.append` is `true`, and `WriteMode::Truncate` when `config.overwrite` is `true`. When both are `false`, `write_mode` retains the default `WriteMode::Truncate` from `CsvExporter::new`. `open_for_write` then calls `.truncate(write_mode == WriteMode::Truncate)` which equals `.truncate(true)` — truncating the file silently despite the user having set `overwrite = false`. No validation in `CsvExporterConfig::validate()` rejects this combination.

**Fix:** Either reject the combination at validation time, or treat `!append && !overwrite` as a third mode (e.g. fail if file already exists):
```rust
// In CsvExporterConfig::validate():
if !self.append && !self.overwrite {
    // check at runtime if file exists and non-empty, or add a dedicated mode
    // Simplest: reject in config
    return Err(Error::Config(ConfigError::InvalidValue {
        field: "exporter.csv".to_string(),
        value: "overwrite=false + append=false".to_string(),
        reason: "one of overwrite or append must be true".to_string(),
    }));
}
```

## Info

### IN-01: `write_record` is a no-op wrapper over `write_record_preparsed`

**File:** `src/exporter/csv/writer.rs:237-261`

**Issue:** `write_record` passes all its arguments verbatim to `write_record_preparsed` and adds nothing. It exists as a named alias but provides no abstraction value — callers in `impls.rs` (lines 37 and 55) could call `write_record_preparsed` directly, eliminating one function call indirection.

**Fix:** Remove `write_record`, update the two call sites in `impls.rs` to call `write_record_preparsed` directly.

---

### IN-02: `compute_normalized` return value silently dropped in `collector.rs` `!passes` branch

**File:** `src/cli/run/collector.rs:88-94`

**Issue:** In the `!passes` branch of `process_record`, `compute_normalized` is called purely for the side effect of updating `params_buf`, but the return value is dropped without an explicit `let _ =` binding. While Rust does not warn for this (the return type is not `#[must_use]`), the intent is not obvious to a reader: it looks like the result might have been accidentally discarded rather than intentionally ignored.

**Fix:**
```rust
// Make the intentional discard explicit:
let _ = crate::pipeline::compute_normalized(
    &record,
    &record.sql,
    params_buf,
    placeholder_override,
    ns_scratch,
);
```

---

### IN-03: `apply_params_into` reservation underestimates capacity for multi-digit `:N` placeholders

**File:** `src/pipeline/normalizer.rs:204-208`

**Issue:** The pre-reservation for `out` computes `extra` as the sum of `param_len - 1` for each parameter, assuming the placeholder being replaced is one character. For colon-style placeholders (`:10`, `:99`, etc.) the placeholder is 3 or 4 bytes, meaning the subtracted amount should be `placeholder_len - 1` rather than a uniform `1`. The undershoot causes `Vec` to reallocate mid-write for SQL with many two-digit-or-longer `:N` placeholders.

**Fix:** This is a performance nuance rather than a correctness issue; no reallocation causes data loss. A precise fix would require scanning the SQL to count placeholder byte-widths before reserving, which adds cost. Acceptable as-is given that correctness is unaffected.

---

_Reviewed: 2026-06-11T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
