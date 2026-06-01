---
phase: "41"
status: has_findings
depth: standard
files_reviewed: 1
files_reviewed_list:
  - src/cli/run/prescan.rs
findings:
  critical: 0
  warning: 3
  info: 2
  total: 5
---

# Phase 41: Code Review Report

**Reviewed:** 2026-05-25T00:00:00Z
**Depth:** standard
**Files Reviewed:** 1
**Status:** has_findings

## Summary

Reviewed `src/cli/run/prescan.rs` — the pre-scan orchestration module responsible for
discovering transaction IDs before the main pass. The file is small (105 lines) and
well-structured. No critical issues were found. Three warnings relate to silent error
discard, an `eprintln!` that bypasses the log infrastructure, and non-UTF8 path handling.
Two info items cover a minor logic inconsistency in `recompile_meta_if_needed` and
unnecessary intermediate duplicates in the cross-file merge.

---

## Warnings

### WR-01: `scan_log_file_for_matches` silently discards file-open errors

**File:** `src/cli/run/prescan.rs:18-20`
**Issue:** When `LogParserBuilder::new(file_path).build()` fails (permission denied,
file deleted between discovery and scan, etc.), the function silently returns an empty
`Vec`. No message is logged, no error counter is incremented. The caller
`scan_for_trxids_by_transaction_filters` cannot distinguish "file opened but had no
matching records" from "file failed to open". Users get no diagnostic for pre-scan
failures, which can lead to silent data loss: if the pre-scan of a file fails, that
file's transactions are not collected into the trxid set, so the main pass will exclude
those transactions even though the file itself may parse successfully in the main pass.

```rust
// Current — silently returns empty Vec on any error
let Ok(parser) = LogParserBuilder::new(file_path).build() else {
    return Vec::new();
};

// Fix — log the error before returning
let parser = match LogParserBuilder::new(file_path).build() {
    Ok(p) => p,
    Err(e) => {
        log::warn!("Pre-scan: failed to open '{}': {e}", file_path);
        return Vec::new();
    }
};
```

---

### WR-02: `eprintln!` bypasses the `log` crate in orchestration code

**File:** `src/cli/run/prescan.rs:61-64`
**Issue:** The pre-scan progress message is emitted via `eprintln!` instead of
`log::info!`. The rest of the codebase (including `mod.rs` of the same module) uses the
`log` crate consistently. Using `eprintln!` means:
- The message cannot be suppressed by setting `RUST_LOG` or the configured log level.
- It is written to stderr regardless of the `--quiet` flag.
- It bypasses the rolling-file log sink configured in `[logging]`.

```rust
// Current
eprintln!(
    "Pre-scanning {} files for transaction-level filters...",
    log_files.len()
);

// Fix
log::info!(
    "Pre-scanning {} files for transaction-level filters...",
    log_files.len()
);
```

---

### WR-03: `to_string_lossy()` silently corrupts non-UTF8 file paths

**File:** `src/cli/run/prescan.rs:76`
**Issue:** `file.to_string_lossy()` replaces invalid UTF-8 bytes in a `PathBuf` with
`\u{FFFD}` (Unicode replacement character). The resulting string is no longer a valid
path to the original file. On platforms that allow non-UTF8 filenames (Linux with
arbitrary byte filenames), the corrupted string is passed to
`scan_log_file_for_matches`, which silently swallows the open error (WR-01). The result:
any log file with a non-UTF8 path is silently skipped during pre-scan, causing the same
silent transaction-loss described in WR-01 even when the file is readable.

```rust
// Current
.flat_map(|file| scan_log_file_for_matches(&file.to_string_lossy(), cfg))

// Fix — propagate an error instead of silently corrupting the path
.flat_map(|file| {
    match file.to_str() {
        Some(path) => scan_log_file_for_matches(path, cfg),
        None => {
            log::warn!("Pre-scan: skipping file with non-UTF8 path: {:?}", file);
            Vec::new()
        }
    }
})
```

---

## Info

### IN-01: Cross-file merge produces duplicate `trxid` strings before `HashSet` dedup

**File:** `src/cli/run/prescan.rs:73-78`
**Issue:** `flat_map` concatenates the per-file `Vec<String>` results directly. Each
individual file's output is deduplicated internally (via `HashSet` in
`scan_log_file_for_matches`), but the same `trxid` may appear in multiple files. The
returned `Vec<String>` from `scan_for_trxids_by_transaction_filters` therefore contains
cross-file duplicates before the caller inserts them into a `TrxidSet`. Correctness is
preserved (the `TrxidSet` deduplicates on insert), but unnecessary `String` allocations
are made — one clone per duplicate occurrence across files.

For large pre-scans with broad transaction filters spanning many files, consider
collecting directly into a `HashSet<String>` inside `pool.install()`:

```rust
let matched: std::collections::HashSet<String> = pool.install(|| {
    log_files
        .par_iter()
        .flat_map(|file| scan_log_file_for_matches(&file.to_string_lossy(), cfg))
        .collect()
});
Ok(matched.into_iter().collect())
```

---

### IN-02: `recompile_meta_if_needed` always creates `Some(...)` when `filter.enable` is true, even if no include/exclude patterns exist

**File:** `src/cli/run/prescan.rs:90-104`
**Issue:** When `filter.enable` is `true` but `filter.include` and `filter.exclude` are
empty (user only configured `indicators` or `sql` for the pre-scan phase),
`recompile_meta_if_needed` returns `Some(CompiledMetaFilters { all fields: None })`.
The downstream `build_pipeline` guards correctly against this via `f.has_filters()`
(so no incorrect filtering occurs), but the doc comment claims "回传原始值的情形：无
filters 配置" — which is contradicted by this code path. The actual behavior is that an
empty `CompiledMetaFilters` is created and then discarded by `build_pipeline`. This is a
documentation/logic inconsistency, not a runtime bug.

Consider adding an early-return guard for the empty case:

```rust
pub(super) fn recompile_meta_if_needed(
    final_cfg: &Config,
    original: Option<CompiledMetaFilters>,
) -> Result<Option<CompiledMetaFilters>> {
    let filters = match &final_cfg.filter {
        Some(f) if f.enable => f,
        _ => return Ok(original),
    };
    // Early return if there are no include/exclude patterns to compile
    if !filters.include.has_filters() && !filters.exclude.has_filters() {
        return Ok(original);
    }
    let recompiled = crate::pipeline::CompiledMetaFilters::try_from_include_exclude(
        &filters.include,
        &filters.exclude,
    )?;
    Ok(Some(recompiled))
}
```

---

_Reviewed: 2026-05-25T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
