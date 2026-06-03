---
phase: 60-error-handling
reviewed: 2026-06-03T00:00:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - src/logging.rs
  - src/cli/run/parallel.rs
findings:
  critical: 0
  warning: 3
  info: 2
  total: 5
status: issues_found
---

# Phase 60: Code Review Report

**Reviewed:** 2026-06-03
**Depth:** standard
**Files Reviewed:** 2
**Status:** issues_found

## Summary

Two files reviewed: the custom log implementation (`src/logging.rs`) and the parallel CSV
processing path (`src/cli/run/parallel.rs`). Neither file contains security vulnerabilities
or data-loss blockers under normal configuration. Three quality-robustness warnings were
found: a silent file path divergence in logging, a file corruption window in the parallel
concat path when `overwrite=false` and a pre-existing output file exists, and a silently
discarded set-logger error that leaves the newly opened log file unused. Two info-level
items cover unnecessary code complexity.

---

## Warnings

### WR-01: `init_logging` silently opens a different file when configured path has no extension

**File:** `src/logging.rs:91-107`

**Issue:** `log_path` is unnecessarily reconstructed from `file_stem` + `extension` to
form `log_file_path`. When the configured `logging.file` has no extension (e.g.,
`"logs/app"`), `log_path.extension()` returns `None`, and `.unwrap_or("log")` invents
the `.log` suffix. The actual file opened is `logs/app.log`, but the `log::info!` message
at line 184 reports `config.file` (`"logs/app"`). Operators monitoring file paths will be
misled, and any external log-rotation tooling targeting the configured path will silently
fail to find the file.

The reconstruction is also pointless: `parent_dir.join(format!("{file_stem}.{extension}"))` is
equivalent to `log_path` when an extension is present, and diverges silently when it is
absent. The simplest fix is to open `log_path` directly and remove the reconstruction
entirely.

**Fix:**
```rust
// Replace lines 91-117 with:
let log_file_path = log_path; // use path as-is
let file = OpenOptions::new()
    .create(true)
    .append(true)
    .open(log_file_path)
    .map_err(|e| {
        Error::File(FileError::CreateDirectoryFailed {
            path: log_file_path.to_path_buf(),
            reason: e.to_string(),
        })
    })?;
```

---

### WR-02: `concat_csv_parts` corrupts output file tail when `overwrite=false` and a larger file already exists

**File:** `src/cli/run/parallel.rs:37-41`

**Issue:** When `overwrite=false` and `append_to_existing=false`, the output file is
opened with `create(true) + write(true) + truncate(false)`. On all POSIX systems and
Windows, this opens an existing file at offset 0 without truncating it. The new CSV
content is written from the beginning, but if the pre-existing file is larger than the
new content, the old bytes remain at the tail of the file. The result is a syntactically
malformed CSV whose final rows are corrupted bytes from the previous run.

The default `overwrite=true` means most users will never hit this. However, a user who
sets `overwrite = false` in config and has a leftover output file from a prior run will
silently receive a corrupt CSV with no error or warning.

The sequential `CsvExporter::initialize` (csv/mod.rs:110-114) has the exact same pattern
and is equally affected.

**Fix:** When `overwrite=false` and not appending, use `create_new(true)` to fail-fast
if the file already exists, matching what `FileError::AlreadyExists` is designed to
communicate. Alternatively, add an explicit `set_len(0)` after opening to guarantee
truncation:

```rust
// Option A — fail-fast (preferred, consistent with AlreadyExists error variant):
OpenOptions::new()
    .create_new(!overwrite)  // fails with AlreadyExists if file exists and overwrite=false
    .create(overwrite)
    .write(true)
    .truncate(overwrite)
    .open(output_path)?

// Option B — always truncate on non-append write:
let file = OpenOptions::new()
    .create(true)
    .write(true)
    .truncate(true) // always truncate when overwrite=false (no append): parallel path always rewrites anyway
    .open(output_path)?;
```

---

### WR-03: `log::set_boxed_logger` failure silently opens and discards a log file

**File:** `src/logging.rs:172-182`

**Issue:** When `set_boxed_logger` fails (i.e., a logger is already registered), the
error is silently swallowed (`let _ = e`). At this point the file at `log_file_path` has
already been created/opened. All subsequent `log::info!` / `log::warn!` calls go to the
**previously** registered logger's file—not the new file—while the newly opened file
descriptor is leaked (dropped with nothing written). The `log::info!` at line 184
succeeds only because the old logger is still active.

In a single-process CLI run this is mostly benign (it only happens in tests where
`set_boxed_logger` is called multiple times in the same process). However, the silent
discard masks the fact that logging configuration for this run is not in effect.

**Fix:** Log a warning to `stderr` so the operator or test knows logging was not
re-initialised, and make clear why:

```rust
Err(_) => {
    // Logger already registered (e.g., during tests); this config has no effect.
    eprintln!(
        "warning: logging already initialized; config {:?} ignored",
        config.file
    );
}
```

---

## Info

### IN-01: `Arc<Mutex<File>>` in `SimpleLogger` is unnecessarily complex

**File:** `src/logging.rs:119-169`

**Issue:** `shared_file` is created as `Arc<Mutex<std::fs::File>>` and then
`.clone()`d into `SimpleLogger`. After the logger is boxed and registered, the original
`shared_file` binding goes out of scope and its `Arc` ref-count drops to 1. The `Arc`
serves no structural purpose—there is no second owner of the file. A plain `Mutex<File>`
(without `Arc`) would be sufficient and simpler since `SimpleLogger` owns the file
exclusively once initialised.

**Fix:** Change the field type to `Mutex<std::fs::File>` and construct it directly:
```rust
struct SimpleLogger {
    level: LevelFilter,
    file: Mutex<std::fs::File>,
    log_to_stdout: bool,
}
// ...
let logger = SimpleLogger { level, file: Mutex::new(file), log_to_stdout };
```

---

### IN-02: Unnecessary log file path reconstruction in `init_logging`

**File:** `src/logging.rs:91-107`

**Issue:** Lines 91-107 decompose `log_path` into `file_stem + extension` and then
reassemble them with `parent_dir.join(...)` to produce `log_file_path`. This is
structurally equivalent to `log_path` whenever an extension is present. The multi-step
decompose-reconstruct pattern introduces the silent divergence described in WR-01 and
adds unnecessary complexity. Remove the reconstruction and open `log_path` directly
(see WR-01 fix).

---

_Reviewed: 2026-06-03_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
