---
phase: 67-prog-diag
reviewed: 2026-06-05T00:00:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - src/cli/run/mod.rs
  - src/cli/run/processor.rs
  - src/cli/run/tests.rs
  - src/config/mod.rs
  - src/error.rs
findings:
  critical: 1
  warning: 4
  info: 2
  total: 7
status: fixed
---

# Phase 67: Code Review Report

**Reviewed:** 2026-06-05T00:00:00Z
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

Phase 67 adds a progress bar upgrade (file counter, ETA, records/sec), an `ErrorStats` extension with `ErrorKind` classification and `ParseErrorRecord` collection, and a summary/error-log-writing facility. The overall structure is sound, but five correctness issues were found: one is a reliable logic bug in the progress bar counter (BLOCKER), and the others are a DoS risk from unbounded merge growth, a spurious speed computation on zero records, a missing zero-length validation on the new config field, and a dead-code annotation that hides a public API surface decision.

---

## Critical Issues

### CR-01: `pb.set_position(0)` in `setup_progress_bar` resets the file-count progress on each new file

**File:** `src/cli/run/processor.rs:120`

**Issue:** The progress bar tracks overall file count (`[pos/len]`). `pb.inc(1)` is called in `log_file_result` (line 153) once per completed file to advance the counter. However, `setup_progress_bar` calls `pb.set_position(0)` unconditionally at the start of every new file. This resets the counter back to 0 before the new file begins, so the `[pos/len]` display always shows `[0/N]` at the start of each file and jumps back to 0 right after showing the completed-file message. For a run with 5 files, the user will see the sequence `[0/5] … [0/5] … [0/5]` instead of `[1/5] … [2/5] … [3/5]`. The ETA calculation in indicatif is also based on position advancement, so this makes ETA incorrect.

The progress bar `len` is set to `total_files` in `make_progress_bar` (mod.rs:209), and `pb.inc(1)` is the correct way to advance it. `set_position(0)` was presumably intended to reset an inner per-file byte or record counter, but the bar is a file-count bar, not a byte/record bar. There is no per-file inner bar.

**Fix:** Remove `pb.set_position(0)` from `setup_progress_bar`. The per-file label already comes from `pb.set_message`, which does not affect the position.

```rust
fn setup_progress_bar(
    pb: Option<&ProgressBar>,
    reset_pb: bool,
    show_progress: bool,
    file_index: usize,
    total_files: usize,
    file_name: &str,
) {
    if reset_pb && show_progress {
        if let Some(pb) = pb {
            pb.set_message(format!("[{file_index}/{total_files}] {file_name}"));
            // Remove: pb.set_position(0);
        }
    }
}
```

---

## Warnings

### WR-01: `ErrorStats::merge` does not enforce the 10,000-record cap on `parse_error_records`

**File:** `src/error.rs:132-133`

**Issue:** `processor.rs` caps each per-file `file_stats.parse_error_records` at 10,000 entries (line 239). However, `ErrorStats::merge` calls `extend` unconditionally, without checking the accumulated length. When merging from multiple files (parallel path or sequential with many files), the `run_stats.parse_error_records` can grow to `N_files × 10_000` entries. On a run with 1,000 files each containing exactly 10,000 parse errors, this accumulates 10 million `ParseErrorRecord` structs in memory (each holding two `String` fields), which is a reliable out-of-memory / DoS vector for adversarial or malformed input files.

**Fix:** Apply the cap during merge:

```rust
pub fn merge(&mut self, other: &ErrorStats) {
    // ... existing field merges ...
    const MAX_RECORDS: usize = 10_000;
    let remaining_cap = MAX_RECORDS.saturating_sub(self.parse_error_records.len());
    if remaining_cap > 0 {
        self.parse_error_records.extend(
            other.parse_error_records.iter().cloned().take(remaining_cap),
        );
    }
}
```

### WR-02: `tick_progress` triggers on `records_in_file == 0` due to `trailing_zeros` returning 64

**File:** `src/cli/run/processor.rs:167`

**Issue:** `usize::trailing_zeros()` returns `64` when the value is `0` (all bits are zero). The guard `if records_in_file.trailing_zeros() >= 10` is therefore `true` on the very first invocation where `records_in_file` could be `0`.

This can happen when: `passes=true`, the record reaches the export path, `export_one_preparsed` returns a non-fatal `Err`, so `records_in_file` remains `0`, and `ExportAction::Continue` is returned. The subsequent arm at line 223 calls `tick_progress(pb, 0, file_start, …)`. Inside `tick_progress`, `elapsed.max(1e-9)` prevents a divide-by-zero, but `0 as f64 / 1e-9 = 0.0`, so the speed label correctly shows `0 rec/s`. The immediate symptom is benign (shows `0 rec/s`), but the interrupt check runs on the very first record instead of every 1024th, slightly changing behavior.

More critically, after `total_processed.wrapping_add(1)` (line 220) this also affects the `!passes` arm at line 225-227: `total_processed` starts at 0 before the increment, so the very first filtered record has `total_processed = 1` after the increment. `1.trailing_zeros() == 0`, so the filtered-record arm fires at every record until `total_processed` reaches 1024. Wait — `total_processed` is incremented AFTER `normalize_and_export` returns, so on the first iteration `total_processed` goes from 0 to 1. `1.trailing_zeros() = 0 < 10`. The `!passes` interrupt check won't misfire. But the `tick_progress` call with `records_in_file = 0` on the first failed export remains an off-by-one in the "every 1024 records" intention.

**Fix:** Add a `records_in_file == 0` early-return guard:

```rust
fn tick_progress(
    pb: Option<&ProgressBar>,
    records_in_file: usize,
    file_start: std::time::Instant,
    file_name: &str,
    interrupted: &Arc<AtomicBool>,
) -> bool {
    if records_in_file == 0 {
        return false;
    }
    if records_in_file.trailing_zeros() >= 10 {
        // ... existing logic
    }
    false
}
```

### WR-03: `write_error_log` uses `stats.parse_error_records` but `write_error_log` is called with `cfg` not `final_cfg`

**File:** `src/cli/run/mod.rs:128`

**Issue:** `handle_run` calls `write_error_log(cfg, &run_stats)` (line 128), passing the original `cfg`, not `final_cfg`. The `final_cfg` is obtained after the prescan merge (`merge_trxid_prescan`) at line 42, and both `final_cfg` and `cfg` share the same `error` field (prescan does not modify it). So in practice, `cfg.error` and `final_cfg.error` are always identical. However, the inconsistency is a maintenance trap: if future code adds logic that modifies `final_cfg.error`, this call will silently use the stale value. All other uses of the config in this function correctly use `final_cfg`.

**Fix:** Use `final_cfg` for consistency:

```rust
write_error_log(final_cfg, &run_stats);
```

### WR-04: `ErrorLogConfig.file` is not validated; an empty or whitespace path causes a misleading panic-free but silently-no-op error log path

**File:** `src/config/mod.rs:19-21`, `src/config/validate.rs`

**Issue:** `Config::validate()` validates `logging.file`, `exporter.csv.file`, and `exporter.sqlite.database_url` against empty/whitespace strings. However, the newly added `error.file` field in `ErrorLogConfig` has no corresponding validation. A config containing `[error]\nfile = "  "` (whitespace) will pass validation, and `std::fs::File::create("  ")` will either create a file named literally `"  "` in the current directory (succeeding silently with a file the user cannot find) or fail with a permissions error on some filesystems. In both cases the user gets no actionable message about the misconfiguration.

**Fix:** Add validation in `Config::validate` (or in a new `ErrorLogConfig::validate` method called from `validate.rs`):

```rust
// In validate.rs or config/mod.rs
if let Some(err_cfg) = &self.error {
    if err_cfg.file.trim().is_empty() {
        return Err(Error::Config(ConfigError::InvalidValue {
            field: "error.file".to_string(),
            value: err_cfg.file.clone(),
            reason: "error log file path must not be empty or whitespace".to_string(),
        }));
    }
}
```

---

## Info

### IN-01: `ParseErrorRecord.file_path` is annotated `#[allow(dead_code)]` but is a structurally meaningful field

**File:** `src/error.rs:49-51`

**Issue:** `file_path` carries per-record provenance (which input file caused the parse error) and is populated in `processor.rs:241`. The `#[allow(dead_code)]` suppresses the compiler warning because the field is never read after construction. This means the collected data is silently discarded — it is never included in the error log output (which only uses `line_number`, `raw_truncated`, and `kind`). The comment "保留字段供未来格式扩展使用" acknowledges this, but the field takes memory (a `String` per record, cloned on every `merge`) for zero current benefit. This is either dead weight or an omission from the error log format.

**Suggestion:** Either (a) include `file_path` in the error log line format in `write_error_log` (mod.rs:491-497) so the field earns its place, or (b) remove it and the `#[allow(dead_code)]` annotation until it is needed.

### IN-02: `truncated` flag in `write_error_log` is computed but only used if the collection is exactly at the cap boundary

**File:** `src/cli/run/mod.rs:489-501`

**Issue:** `let truncated = stats.parse_error_records.len() >= 10_000;` is used to decide whether to write a `[truncated at 10000 records]` footer. However, with WR-01's merge bug still present, the collection could hold more than 10,000 records and `truncated` would still be `true` (which is correct), but the footer wording `"[truncated at 10000 records]"` would be misleading when the actual count is, say, 250,000. After fixing WR-01, this becomes accurate. The issue is minor but the footer message should reflect the actual record count:

```rust
if truncated {
    let _ = writeln!(
        writer,
        "[truncated; showing first 10000 of {} total parse errors]",
        stats.parse_errors
    );
}
```

---

_Reviewed: 2026-06-05T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
