# Architecture Research — v1.10 Quality Features Integration

**Domain:** Rust CLI tool for parsing DM database SQL logs
**Researched:** 2026-05-21
**Confidence:** HIGH (based on direct codebase analysis of existing architecture)

## Data Flow Overview (Existing)

```
sqllog2db run -c config.toml
    │
    ▼
handle_run()                          [src/cli/run/mod.rs]
    │
    ├─ SqllogParser::new(path).log_files()   [src/parser.rs]
    │   └─ Returns Vec<PathBuf> (sorted .log files)
    │
    ├─ scan_for_trxids()               [src/cli/run/prescan.rs]
    │   └─ Only if transaction-level filters configured
    │
    ├─ build_pipeline()                [src/cli/run/filter_processor.rs]
    │
    ├─ [branch] use_parallel?
    │   ├─ YES: process_csv_parallel()  [src/cli/run/parallel.rs]
    │   │   └─ rayon par_iter per file
    │   └─ NO:  sequential loop
    │       └─ process_log_file() per file  [src/cli/run/processor.rs]
    │           └─ LogParserBuilder::new(path).build().iter()
    │               └─ Hot loop: parse → filter → export_one_preparsed()
    │
    └─ summary output (eprintln!)
```

### Component Boundaries

| Component | File | Responsibility | Communicates With |
|-----------|------|----------------|-------------------|
| `handle_run` | `cli/run/mod.rs` | Main orchestration: file discovery, pipeline build, loop orchestrator | `SqllogParser`, `Pipeline`, `ExporterManager`, `process_log_file`, `process_csv_parallel` |
| `SqllogParser` | `parser.rs` | File/glob pattern discovery, path validation | Filesystem, `LogParserBuilder` |
| `process_log_file` | `cli/run/processor.rs` | Single-file hot loop: parse records through pipeline to exporter | `LogParserBuilder`, `Pipeline`, `ExporterManager` |
| `Pipeline` | `pipeline/mod.rs` | Ordered filter execution, `is_empty()` fast path | `LogProcessor` trait implementations |
| `ExporterManager` | `exporter/mod.rs` | Factory + enum dispatch to active exporter | `CsvExporter` or `SqliteExporter` |
| `Error` types | `error.rs` | Typed errors: `ConfigError`, `ParserError`, `FileError`, `ExportError`, `Io`, `Interrupted` | All components via `Result<T>` |
| `preflight` | `preflight.rs` | Pre-run validation of paths and output writability | `SqllogParser`, filesystem |

### Data Flow Characteristics

- **Single-threaded streaming** (parallel path is CSV-only with per-file parallelism via rayon)
- **Constant memory**: records processed one-at-a-time, only `BufWriter` buffer allocated
- **File-level parallelism**: parallel path splits files across threads, each thread has own CSV temp file
- **Zero-overhead fast path**: `pipeline.is_empty()` check avoids filter overhead
- **Pre-scan**: two-pass for transaction-level filters (scan for trxids, then filter by trxid)

---

## Feature 1: Typed Error with Continue-on-Error (ERR-01, ERR-02)

### Current State

Error types in `src/error.rs` already have a reasonable hierarchy:
- `Error` enum with `Config`, `File`, `Parser`, `Export`, `Io`, `Interrupted` variants
- Each variant delegates to sub-error types via `#[from]`

Parse errors in the hot loop (`processor.rs:123-131`) are already non-fatal — they are logged as `warn!` and processing continues:
```rust
Err(e) => {
    errors_in_file += 1;
    log::warn!("{file_path} | {e:?}");
}
```

However, **exporter errors** (line 100 `export_one_preparsed(...)?`) are fatal — the `?` propagates and terminates the entire run.

### Integration Points

#### Point 1: Hot loop in `processor.rs` — export error handling

**File:** `src/cli/run/processor.rs`, lines 98-101

**Current code:**
```rust
exporter_manager.export_one_preparsed(&record, include_pm, ns)?;
records_in_file += 1;
```

**Change:** Wrap exporter call in match, log error and continue:
```rust
match exporter_manager.export_one_preparsed(&record, include_pm, ns) {
    Ok(()) => records_in_file += 1,
    Err(e) => {
        errors_in_file += 1;
        log::warn!("{file_path} | export error: {e}");
    }
}
```

**Risk:** LOW. Pattern already used for parse errors. Need to ensure `errors_in_file` is propagated back to caller and displayed correctly.

#### Point 2: Error type refinement — `ParseError` needs context fields

**File:** `src/error.rs`

**Change:** Add `source_path: Option<PathBuf>` and `line_number: Option<u64>` fields to relevant error variants. The upstream `dm-database-parser-sqllog` crate already provides `line_number` in its `ParseError::InvalidFormat` variant.

**Risk:** LOW. Adding Option fields is backward-compatible for match patterns.

#### Point 3: Error counting and reporting

**File:** `src/cli/run/mod.rs`, summary output (lines 140-151)

**Change:** Include error count in the final summary. Currently `total_records` is tracked but error count is per-file only. Need to accumulate total errors across all files.

**Risk:** LOW. Mechanical change.

### Module Impact Summary

| File | Change | Risk |
|------|--------|------|
| `src/error.rs` | Add `source_path`, `line_number` to `ParseError` | LOW |
| `src/cli/run/processor.rs` | Wrap `export_one_preparsed` in match, increment error counter | LOW |
| `src/cli/run/mod.rs` | Aggregate error count across files, display in summary | LOW |

---

## Feature 2: Stdin Input (PIPE-01)

### Current State

`SqllogConfig.path` is always a filesystem path. `SqllogParser::scan_log_files()` calls `path.exists()`, `path.is_file()`, `path.is_dir()` — all filesystem operations.

`LogParserBuilder::new(path).build()` from the upstream crate calls `fs::read(&self.path)` — it reads the **entire file into memory** as `Vec<u8>`, then parses records from the in-memory buffer. It does not support `io::Read` trait input.

### Architecture Decision: Platform Stdin Path Mapping

**Approach:** Map a magic path value `"-"` to the platform's stdin device path. On Unix: `/dev/stdin`. On Windows: `CONIN$`.

This is the minimal-change approach because:
1. `LogParserBuilder` and `LogParser` read the entire file into memory via `fs::read()` — this is fine for stdin too (pipe input is never truly streaming at the parser level)
2. No changes needed to the upstream crate
3. `/dev/stdin` is a standard Unix convention, `/dev/stdin` on macOS is supported
4. The rest of the pipeline is path-agnostic

**Why not alternative approaches:**
- **Writing a custom stdin parser**: duplicates crate logic for multi-line record boundary detection
- **Temp file**: breaks constant-memory guarantee, adds I/O overhead
- **`io::Read` adapter on `LogParser`**: not supported by the upstream crate API

### Integration Points

#### Point 1: Stdin detection in `SqllogConfig`

**File:** `src/config/sqllog.rs`

**Change:** Add `const STDIN_MARKER: &str = "-"` and method:
```rust
impl SqllogConfig {
    pub fn is_stdin(&self) -> bool {
        self.path.trim() == "-"
    }
    
    /// Returns platform stdin device path
    pub fn stdin_device_path() -> &'static str {
        if cfg!(target_os = "windows") { "CONIN$" } else { "/dev/stdin" }
    }
}
```

**Risk:** LOW. Pure addition, no behavioral change for existing paths.

#### Point 2: Orchestration in `handle_run`

**File:** `src/cli/run/mod.rs`

**Change:** Before file scanning, check stdin mode:
```rust
if cfg.sqllog.is_stdin() {
    // Bypass SqllogParser, skip file scanning
    // Build LogParserBuilder directly with stdin device path
    // Process as single virtual file "-"
    // Skip parallel path (stdin is always sequential)
} else {
    // existing file-based flow
}
```

**Detailed flow for stdin:**
1. Build `LogParserBuilder::new(SqllogConfig::stdin_device_path()).build()`
2. Call a new function `process_stdin()` (or reuse `process_log_file` with the stdin path)
3. Display file name as `"-"` instead of `/dev/stdin`
4. Skip preflight log path check for stdin mode
5. Skip pre-scan for transaction filters (no file to pre-scan; stdin cannot be pre-scanned)

**Risk:** MEDIUM. The `process_log_file` function uses `file_path` for `LogParserBuilder::new()`. Passing `/dev/stdin` works but display labels need adjustment. The `file_name` extraction in `process_log_file` (line 39-41) would show `"stdin"` from `/dev/stdin` — acceptable but subtle.

#### Point 3: Preflight adjustment

**File:** `src/preflight.rs`

**Change:** In `check_log_path()`, skip filesystem checks when path is stdin marker:
```rust
fn check_log_path(path_str: &str, result: &mut PreflightResult) {
    if path_str.trim() == "-" {
        return; // stdin mode, skip path validation
    }
    // ... existing checks
}
```

**Risk:** LOW. Early return avoids filesystem operations on non-file path.

#### Point 4: Pre-scan bypass for stdin

**File:** `src/cli/run/mod.rs`, around lines 44-58

**Change:** Skip `scan_for_trxids_by_transaction_filters` when stdin mode (stdin data can't be pre-scanned). Transaction-level filters combined with stdin input should either be rejected with a clear error message, or trxid pre-scan should be skipped and filters applied per-record (with degraded semantics).

**Recommendation:** Print a warning when stdin + transaction filters are combined: "transaction-level filters with stdin input cannot pre-scan — filters will apply per-record only"

**Risk:** MEDIUM. This is a semantic change — transaction-level filters lose their "keep whole transaction" property with stdin.

#### Point 5: CLI flag or convention documentation

**File:** `src/cli/opts.rs`

**Change:** Update `--help` to document stdin convention:
```rust
/// Path to SQL log files (directory, file, glob pattern, or "-" for stdin)
```

**Risk:** LOW.

### Module Impact Summary

| File | Change | Risk |
|------|--------|------|
| `src/config/sqllog.rs` | Add `is_stdin()`, `stdin_device_path()` | LOW |
| `src/cli/run/mod.rs` | Branch on stdin: bypass file scan, build parser directly, skip parallel path | MEDIUM |
| `src/preflight.rs` | Skip log path check when stdin | LOW |
| `src/cli/opts.rs` | Document `-` convention in help | LOW |

---

## Feature 3: Progress Display (UX-01)

### Current State

Progress is basic `eprintln!` statements:
- Per-file start: `"[{idx}/{total}] {filename}"` (in `process_log_file` line 45)
- Per-file completion: `"✓ [{idx}/{total}] {path} — {count}{errors}, {elapsed:.2}s"` (line 146)
- `show_progress: bool` is `!quiet` (from `make_progress_bar` in `filter_processor.rs`)
- No per-record progress during file processing

### Architecture Decision: Carriage Return + Periodic Update

**Approach:** Use `\r` (carriage return) for in-place single-line progress updates, updated every N records. Zero new dependencies. Compatible with both sequential and parallel paths.

**Format:** `"\r[{file_index}/{total_files}] {file_name} | {count} records | {elapsed:.1}s | {rate:.0}/s"`

**Update frequency:** Every 1024 records (aligned with existing interrupt check at line 104-107).

### Integration Points

#### Point 1: Hot loop progress update

**File:** `src/cli/run/processor.rs`, after line 107 (existing interrupt check)

**Change:** Add progress update alongside interrupt check:
```rust
// Every 1024 records: check interrupt + update progress
if records_in_file.trailing_zeros() >= 10 {
    if show_progress {
        let elapsed = file_start.elapsed().as_secs_f64();
        let rate = if elapsed > 0.0 { records_in_file as f64 / elapsed } else { 0.0 };
        eprint!("\r[{file_index}/{total_files}] {file_name} | {records_in_file} records, {elapsed:.1}s ({rate:.0}/s)");
    }
    if interrupted.load(Ordering::Relaxed) {
        break 'outer;
    }
}
```

**Risk:** LOW. Non-blocking, single-line update. `\r` is a standard terminal escape.

#### Point 2: Clear progress line before file-completion message

**File:** `src/cli/run/processor.rs`, before the completion eprintln at line 145

**Change:** Clear the in-place progress line:
```rust
if show_progress && records_in_file > 0 {
    eprint!("\r\x1b[K"); // Clear line
}
```

**Risk:** LOW.

#### Point 3: Remove redundant per-file start message

If progress line already shows `[{idx}/{total}] {filename}`, the start eprintln at line 45 becomes redundant when `show_progress` is true.

**Change:** Conditionally skip start eprintln when progress display is active:
```rust
if reset_pb && show_progress {
    // Progress display will show file info inline
    // eprintln!("[{file_index}/{total_files}] {file_name}");  // conditional
}
```

**Risk:** LOW.

#### Point 4: Parallel path progress

**File:** `src/cli/run/parallel.rs`

**Challenge:** Multiple threads writing to stderr with `\r` will interleave. The simplest approach: keep current per-file start/completion `eprintln!` for parallel path (no per-record progress). Per-record progress is most useful for large single-file sequential runs.

**Risk:** LOW. Parallel progress is a future enhancement.

### Module Impact Summary

| File | Change | Risk |
|------|--------|------|
| `src/cli/run/processor.rs` | Add `\r` progress line after interrupt check, clear before completion msg | LOW |
| `src/cli/run/parallel.rs` | No change (keep current eprintln progress) | LOW |

---

## Feature 4: Better Help and Error Messages (UX-02, UX-03, UX-04)

### Current State

**CLI help** (`src/cli/opts.rs`):
- Minimal `about` and `long_about` strings
- Basic arg descriptions
- No examples in help text
- No environment variable documentation beyond `SQLLOG2DB_CONFIG`

**Error display** (`src/main.rs`):
- Simple `eprintln!("Error: {e}")`

**Summary output** (`src/cli/run/mod.rs`, lines 140-151):
- Basic single-line summary with elapsed time, record count, skipped count
- No error count display (will be added by ERR-02)

### Integration Points

#### Point 1: CLI help text improvements

**File:** `src/cli/opts.rs`

**Changes:**
1. Add `after_help` or `after_long_help` with usage examples
2. Improve command descriptions: `run`, `init`, `validate`
3. Document env var conventions (`SQLLOG2DB_CONFIG`)
4. Document stdin convention `-`

**Example addition:**
```rust
#[command(
    after_help = "EXAMPLES:\n  \
        sqllog2db init -o config.toml           Generate default config\n  \
        sqllog2db run -c config.toml            Run with config file\n  \
        cat sqllogs/2025-01.log | sqllog2db run -c config.toml -- -  Read from stdin\n  \
        sqllog2db validate -c config.toml       Validate config",
)]
```

**Risk:** LOW. Pure markup change, no logic impact.

#### Point 2: Error display improvements

**File:** `src/main.rs` and `src/cli/run/mod.rs`

**Current error display:**
```rust
Err(e) => {
    let code = exit_code_for(&e);
    if code != EXIT_INTERRUPTED {
        eprintln!("Error: {e}");
    }
    std::process::exit(code);
}
```

**Changes:**
1. Add error context: include file path, line number in error strings
2. Add suggestion for actionable errors (e.g., "Tip: run 'sqllog2db init' to generate a config")
3. For parse errors with `line_number`, include it in the error message
4. Use ANSI color codes for error severity (red for errors, yellow for warnings) — lightweight, no new dependency

**ErrorWithContext struct (optional):**
```rust
pub struct ErrorWithContext {
    pub error: Error,
    pub source_path: Option<PathBuf>,
    pub line_number: Option<u64>,
    pub suggestion: Option<String>,
}
```

**Risk:** LOW. Color codes work on most modern terminals. Display logic is straightforward.

#### Point 3: Summary output improvement

**File:** `src/cli/run/mod.rs`, summary section (lines 140-151)

**Current:**
```rust
eprintln!("\n✓ SQL Log Export Task Completed{mode_label} in {elapsed:.2}s — {total_records} records total{skip_label}");
```

**Changes:**
1. Include error count in summary (after ERR-02 adds it)
2. Add human-friendly rate display (records/sec)
3. Mention output file path
4. Add ANSI green checkmark for success, red X for errors
5. Show file count

**Example improved:**
```
✓ SQL Log Export Task Completed in 12.34s  
   Files: 5 (1 skipped)  
   Records: 125,432 exported | 3 errors  
   Output: export/out.csv  
   Rate: 10,162 records/s
```

**Risk:** LOW. Display-only change.

### Module Impact Summary

| File | Change | Risk |
|------|--------|------|
| `src/cli/opts.rs` | Improve `after_help`, arg descriptions, examples | LOW |
| `src/main.rs` | Color-coded error display, suggestions | LOW |
| `src/cli/run/mod.rs` | Structured summary with error count, rate, file count | LOW |
| `src/error.rs` | (Optional) `ErrorWithContext` struct or context fields | LOW |

---

## Build Order and Dependencies

```
                 ┌──────────────────────┐
                 │  Wave 1: Foundation   │
                 ├──────────────────────┤
                 │ ERR-01: Error types   │◄──── Independent
                 │  (refinement)         │
                 │                       │
                 │ UX-03: CLI help text  │◄──── Independent
                 │  (after_help, docs)   │
                 └───────┬──────────────┘
                         │
                         ▼
                 ┌──────────────────────┐
                 │  Wave 2: Core Logic   │
                 ├──────────────────────┤
                 │ ERR-02: Continue-on-  │◄──── Depends on ERR-01
                 │  error in hot loop   │      (uses refined error types)
                         │
                 ┌───────┴──────────────┐
                 │                      │
                 ▼                      ▼
        ┌──────────────────┐  ┌──────────────────┐
        │  Wave 3: Features │  │  Wave 3: UX       │
        ├──────────────────┤  ├──────────────────┤
        │ PIPE-01: Stdin    │  │ UX-04: Error     │
        │  input            │  │  context msgs    │
        │                   │  │                  │
        │ UX-01: Progress   │  │                  │
        │  display          │  │                  │
        └──────────────────┘  └──────────────────┘
                         │
                         ▼
                 ┌──────────────────────┐
                 │  Wave 4: Polish      │
                 ├──────────────────────┤
                 │ UX-02: Summary       │◄──── Depends on ERR-02
                 │  output formatting   │      (needs error count)
                 └──────────────────────┘
```

### Wave 1 (Parallelizable)
| Task | Files | Est. Effort | Rationale |
|------|-------|-------------|-----------|
| ERR-01: Error type refinement | `error.rs`, config errors | Small | Foundation, sets error patterns for all others |
| UX-03: CLI help text | `cli/opts.rs` | Small | Pure markup, zero risk |

### Wave 2
| Task | Files | Est. Effort | Rationale |
|------|-------|-------------|-----------|
| ERR-02: Continue-on-error in hot loop | `processor.rs`, `run/mod.rs` | Medium | Core behavioral change, test-heavy |

### Wave 3 (Parallelizable)
| Task | Files | Est. Effort | Rationale |
|------|-------|-------------|-----------|
| PIPE-01: Stdin input | `sqllog.rs`, `run/mod.rs`, `preflight.rs` | Medium | New code path, needs edge case handling |
| UX-01: Progress display | `processor.rs` | Small | Hot loop addition, display-only |
| UX-04: Error context | `error.rs`, `main.rs` | Small | Error message formatting |

### Wave 4
| Task | Files | Est. Effort | Rationale |
|------|-------|-------------|-----------|
| UX-02: Summary output | `run/mod.rs` | Small | Needs ERR-02 error counts |

### Key Dependency: ERR-02 blocks UX-02

The summary output improvement depends on error counts being available from ERR-02. All other wave 3/4 tasks are independent of each other.

### Risk: Stdin + Transaction Filters

Combining stdin input with transaction-level filters (`indicators.*` or `sql.*` in config) loses the "keep whole transaction" property because pre-scan is impossible on a stream. This should emit a warning and degrade gracefully (apply filters per-record).

---

## Edge Cases and Constraints

### Stdin
1. **No data piped**: `is_terminal()` check → error with suggestion "pipe log data to stdin or specify a file path"
2. **Empty pipe**: `cat /dev/null | sqllog2db run -c config.toml -- -` → handle gracefully (0 records)
3. **Parallel path incompatible**: stdin is always sequential
4. **Pre-scan incompatible**: emit warning when transaction filters + stdin

### Continue-on-Error
1. **All records errored**: Should still exit cleanly with non-zero count
2. **Exporter fatal errors**: e.g., disk full — should still propagate (can't continue meaningfully)
3. **Error counting**: Must distinguish parse errors from export errors in summary

### Progress
1. **Piped output**: `2>&1 | ...` includes progress lines. When stderr is piped, `\r` sequences appear raw. Detect piped stderr via `is_terminal()` on stderr and disable `\r` progress (fall back to per-file eprintln)
2. **Very fast processing**: At 5.2M records/sec, progress updates at 1024-record intervals would fire ~5000 times/sec — too fast for visible updates. Add rate-limiting: update at most every 100ms

### Help/Errors
1. **Subcommand-specific help**: `sqllog2db run --help` should show run-specific examples
2. **Color in non-terminal**: Detect whether stderr is a terminal before using ANSI codes
3. **i18n**: Error messages are currently in Chinese. Keep consistent with existing style

---

## Files Changed Summary

### New Files
*(None — all changes are modifications to existing files)*

### Modified Files

| File | Features Touched | Nature of Change |
|------|-----------------|------------------|
| `src/error.rs` | ERR-01, UX-04 | Add context fields to error variants, optional suggestion |
| `src/cli/run/processor.rs` | ERR-02, UX-01 | Export error handling (match instead of `?`), progress `\r` updates, error counting |
| `src/cli/run/mod.rs` | ERR-02, PIPE-01, UX-02 | Stdin branch in orchestration, aggregate error counts, improved summary |
| `src/config/sqllog.rs` | PIPE-01 | `is_stdin()`, `stdin_device_path()` methods |
| `src/preflight.rs` | PIPE-01 | Skip log path check for stdin mode |
| `src/cli/opts.rs` | UX-03 | Rich after_help, examples, arg descriptions |
| `src/main.rs` | UX-04 | Color-coded error display, suggestions |

### Integration Risk Summary

| Integration | Risk Level | Mitigation |
|-------------|-----------|------------|
| Continue-on-error + exporter errors | LOW | Same pattern as existing parse error handling |
| Stdin + transaction filters | MEDIUM | Graceful degradation with warning; document limitation |
| Stdin + parallel path | LOW | Stdin bypasses parallel branch entirely |
| Progress + very fast processing | LOW | Rate-limiting at 100ms intervals |
| Progress + piped stderr | LOW | Detect non-terminal stderr, disable `\r` |
| Error context + existing match patterns | LOW | Optional fields, backward-compatible Display |

---

## Sources

- Direct codebase analysis of all modules in `/Users/guang/Projects/sqllog2db/src/`
- `dm-database-parser-sqllog` crate v1.1.0 source at `~/.cargo/registry/src/.../dm-database-parser-sqllog-1.1.0/src/parser.rs` (confirms `LogParserBuilder` uses `fs::read` internally, no `io::Read` support)
- Project requirements: `.planning/PROJECT.md`
- Rust `std::io::Stdin::is_terminal()`: stable since Rust 1.70
