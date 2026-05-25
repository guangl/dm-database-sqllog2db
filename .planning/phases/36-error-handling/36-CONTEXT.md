# Phase 36: 错误处理体系重构 - Context

**Gathered:** 2026-05-21
**Status:** Ready for planning

## Phase Boundary

Refactor the error handling system in `src/error.rs` to support typed error categories with fatal/non-fatal classification, add context fields (line_number, suggestion, severity), implement continue-on-error for non-fatal export errors in the hot path (`src/cli/run/processor.rs`), and add per-run error statistics tracking.

## Implementation Decisions

### Fatal vs Non-Fatal Classification
- **D-01:** Use fully granular fatal/non-fatal classification within sub-error enums:
  - ConfigError: InvalidLogLevel → fatal, ParseFailed → fatal, NotFound → fatal, InvalidValue → fatal, NoExporters → fatal
  - FileError: AlreadyExists → fatal (config/preflight), WriteFailed → non-fatal (per-record), CreateDirectoryFailed → fatal
  - ParserError: all variants → non-fatal (per-record parse failure should not stop the run)
  - ExportError: WriteFailed → non-fatal (per-record write failure), DatabaseFailed → fatal (SQLite connection broken)
  - Io: all variants → fatal (filesystem-level failures)
  - Interrupted → fatal

### Error Context Fields
- **D-02:** Add three new optional fields to relevant error variants:
  - `line_number: Option<u64>` — parse error source line
  - `suggestion: Option<String>` — human-readable remediation hint
  - `severity: Option<ErrorSeverity>` — enum { Warning, Error, Critical }

### is_fatal() Method
- **D-03:** Centralize fatal/non-fatal decision on `Error::is_fatal(&self) -> bool` method
- Each sub-error enum also gets `is_fatal()` for internal use, but the top-level Error delegates to sub-enum methods

### Non-Fatal Error Output
- **D-04:** Output non-fatal errors to BOTH:
  - stderr — immediate visibility (formatted: `[WARN] {file}:{line}: {message}`)
  - error log file — persistent record (via `log::warn!`)
- Note: stderr + progress bar interference will be resolved in Phase 38

### Exit Code Strategy
- **D-05:** Three-tier exit codes:
  - 0 = all records processed successfully, zero errors
  - 1 = processing completed but non-fatal errors occurred (partial success)
  - 2 = fatal error, processing could not complete

### Error Statistics
- **D-06:** New `ErrorStats` struct tracked in the Processor layer (not embedded in Error enum):
  - Fields: `total_errors: usize`, `parse_errors: usize`, `export_errors: usize`, `fatal_error: Option<String>`
  - Updated in Processor's match branches after each error
  - Passed to UX-02 (Phase 38) for summary display

### Claude's Discretion
- Exact format string for stderr error output
- ErrorSeverity enum variants and their Display implementation
- ErrorStats method signatures
- Whether to use `AtomicUsize` internally in ErrorStats

## Canonical References

### Requirements
- `.planning/REQUIREMENTS.md` — ERR-01 (error type refinement), ERR-02 (continue-on-error), ERR-03 (error message triage)

### Roadmap
- `.planning/ROADMAP.md` — Phase 36 details (5 success criteria)

### Research
- `.planning/research/PITFALLS.md` — Pitfalls 1, 4, 6 (fatal/non-fatal boundary, error code over-engineering, hot path zero-cost)

### Code
- `src/error.rs` — Current error types (modification target)
- `src/cli/run/processor.rs` — Hot path where continue-on-error lives
- `src/cli/run/mod.rs` — Orchestration layer for error stats and exit code

## Existing Code Insights

### Reusable Assets
- thiserror `#[from]` auto-conversion already wired
- `log::warn!` pattern already used for parse errors in processor.rs line 125
- Existing `Result<T>` type alias at `src/error.rs:5`

### Established Patterns
- Sub-error enum organization (ConfigError, FileError, ParserError, ExportError)
- Parse errors are already non-fatal (matched in processor.rs with `log::warn!`)
- Export errors currently use `?` — must change to `match` for continue-on-error

### Integration Points
- `src/error.rs` — add fields, is_fatal(), ErrorStats
- `src/cli/run/processor.rs` — change `?` to match for export errors
- `src/cli/run/mod.rs` — collect ErrorStats, set exit code
- `src/main.rs` — use exit code from ErrorStats

## Deferred Ideas

None — discussion stayed within phase scope.

---

*Phase: 36-错误处理体系重构*
*Context gathered: 2026-05-21*
