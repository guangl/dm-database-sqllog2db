# Technology Stack

**Project:** sqllog2db v1.10 quality/UX improvements
**Researched:** 2026-05-21

## Recommended Stack

### New Dependencies

| Dependency | Version | Purpose | Why |
|------------|---------|---------|-----|
| `indicatif` | `0.18` (latest 0.18.4) | Progress bars and spinners | Progress bar IS the UX feature. Terminal-aware output, ETA, rate formatting, `MultiProgress` for parallel mode. Replaces fragile `\r` + `eprintln!` that conflicts with `env_logger` output and breaks in parallel mode. |

### No New Dependencies Needed

| Feature | Approach | Rationale |
|---------|----------|-----------|
| ERR-01 Typed errors | Enrich `thiserror` variants with context fields | `thiserror` already in use. Add `line_number`, `suggestion` fields to existing variants. |
| ERR-02 Continue-on-error | `match` + `log::warn!` (existing pattern) | Parse errors already handled this way. No library change needed. |
| PIPE-01 Stdin input | `"/dev/stdin"` path passed to `LogParserBuilder` | On Unix/macOS, `LogParserBuilder::new("/dev/stdin").build()` calls `fs::read("/dev/stdin")` which reads all stdin bytes into memory -- same behavior as `fs::read(file)`. Zero-dep, works immediately. |
| UX-03 Better help | `after_help`, `long_about` on clap derive | Pure doc-string improvement. clap 4.6.1 already has all needed capabilities via `derive` feature. |
| UX-04 Error context | Enrich `thiserror` Display + add `suggestion()` method | File path, line number, suggestion hint included in `#[error("...")]` format strings. |
| UX-02 Output colors | Conditional ANSI escape codes + `std::io::IsTerminal` | `IsTerminal` trait stable since Rust 1.70. Two ANSI codes needed (bold, green for OK, red for error). Not worth a crate. |

## New Dependency Details

### indicatif 0.18

```toml
indicatif = "0.18"
# No rayon feature needed for v1.10 — we use MultiProgress for parallel mode,
# not ParallelProgressIterator. Add if Rayon parallel-iterator tracking is desired.
# indicatif = { version = "0.18", features = ["rayon"] }
```

**Dependency weight:** Light. indicatif depends on `console` (terminal width, term detection) and `portable-atomic` (atomic operations). The `console` crate is widely used and lightweight.

**Integration with existing architecture:**
- Current: `make_progress_bar(quiet: bool) -> bool` returns `!quiet`
- Change: `make_progress_bar() -> Option<ProgressBar>` returns progress bar or `None`
- Sequential mode: `ProgressBar::new_spinner().with_style(...)` for indeterminate, or `ProgressBar::new(total)` if count known from pre-scan
- Record loop: `bar.inc(1)` after each successful export
- File complete: `bar.set_message(...)` or `println(...)` for per-file summary
- Parallel mode: `MultiProgress` with per-file bars; each rayon task creates and manages its own bar via `MultiProgress::add()`
- Completion: `bar.finish_and_clear()` or `bar.finish_with_message(...)`

**Why not `\r` + `eprint!`?** Four problems with the zero-dependency approach:
1. `env_logger` writes to stderr -- `\r` updates fight with structured log output, causing interleaved garbage
2. Parallel mode (rayon): multiple threads writing `\r` to the same terminal is unsynchronized chaos
3. After progress, the per-file summary `eprintln!("[1/5] File X — N records")` is stuck in the middle of the bar
4. No ETA, no rate formatting, no terminal width handling

These are not abstract concerns -- the existing code already has `eprintln!` progress interleaved with `info!()` log calls, and the parallel mode (`process_csv_parallel`) skips progress entirely because it causes issues.

## Alternatives Considered

| Feature | Recommended | Alternative | Why Not |
|---------|-------------|-------------|---------|
| Progress bars | `indicatif 0.18` | `\r` + `eprint!` | Breaks with env_logger, unsynchronized in parallel mode, no ETA (see analysis above) |
| Error formatting | Enriched thiserror | `miette 7.6.0` (with fancy) | miette adds ~15 transitive deps for source-code-snippet display. SQL log errors are file:line oriented, not source-code oriented. thiserror enrichment covers 95% of value at 0% of the dependency cost. |
| Terminal detection | `std::io::IsTerminal` | `atty` crate | `IsTerminal` is stable stdlib since Rust 1.70. Project already uses Rust 1.85. No reason to add atty. |
| Stdin parsing | `/dev/stdin` path mapping | Custom stdin parser | Duplicates upstream parser's record boundary detection logic. `fs::read("/dev/stdin")` in the parser is equivalent to stdin-to-bytes. Only works on Unix -- acceptable since DaMeng database is a Unix/Linux environment. |
| Top-level errors | Current `Error` enum | `anyhow` / `eyre` | Would lose structured variant dispatch needed for exit codes. Current pattern already provides per-variant Display + exit code mapping. |
| Error colors | Conditional ANSI codes | `termcolor` / `owo-colors` | We only need 2-3 codes in one place (the completion summary line). Not worth a crate. |
| Shell completions | Out of scope for v1.10 | `clap_complete` | Shell completion is a distribution concern, not a UX priority for v1.10. |
| Structured tracing | Keep `log` + `env_logger` | `tracing` + `tracing-subscriber` | tracing adds span overhead for no benefit in a single-threaded streaming architecture. Current logging is sufficient. |

## What NOT to Add

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `miette` | Heavy dep (fancy feature pulls in owo-colors, unicode-width, etc.) for marginal benefit in this domain. Error context (file:line) is achievable with thiserror alone. | Enriched thiserror variants with `line_number: u64`, `detail: String`, `suggestion: &'static str` |
| `anyhow` / `eyre` | Would erase structured error type dispatch needed for exit codes in `main.rs`. | Current `thiserror::Error` enum |
| `atty` | Unnecessary -- stdlib has `IsTerminal` since 1.70 | `std::io::stdin().is_terminal()` |
| `console` | Already a transitive dep of indicatif. Only add directly if Term::is_terminal() or Term::size() needed separately. | Wait until indicatif is the only consumer |
| `owo-colors` / `yansi` / `colored` | Only need 2-3 ANSI codes for the completion summary. Overkill. | `format!("\x1b[32m...")` + `IsTerminal` |
| `human-panic` | `panic=abort` in release profile. Panics should not occur. If they do, backtrace is more useful. | Not needed |
| `dialoguer` | Interactive prompts not relevant for batch CLI tool | Not applicable |
| `clap_complete` | Shell completion out of scope for v1.10 | Defer |
| `tracing` | Span overhead with no benefit for streaming architecture | Keep `log` + `env_logger` |

## Installation

```bash
# Add progress bar support
cargo add indicatif@0.18

# Everything else uses existing deps or stdlib
cargo build --release
```

## Integration Points with Existing Architecture

### Error Enrichment (ERR-01, UX-04) -- `src/error.rs`

Current:
```rust
#[derive(Debug, Error)]
pub enum ParserError {
    #[error("Path not found: {path}")]
    PathNotFound { path: PathBuf },
    #[error("Invalid path {path}: {reason}")]
    InvalidPath { path: PathBuf, reason: String },
    #[error("Failed to read directory {path}: {reason}")]
    ReadDirFailed { path: PathBuf, reason: String },
}
```

Change: Add `ParseFailed` variant with line number and suggestion. Add `suggestion()` method to `Error`:
```rust
#[error("Parse error at {path}:{line}: {detail}")]
ParseFailed {
    path: PathBuf,
    line: u64,
    detail: String,
}
```

Add to `Error`:
```rust
impl Error {
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            Self::Parser(ParserError::ParseFailed { .. }) => {
                Some("Check if the log line follows the expected format")
            }
            // ...
            _ => None,
        }
    }
}
```

Top-level display in `main.rs`:
```rust
Err(e) => {
    let code = exit_code_for(&e);
    eprintln!("Error: {e}");
    if let Some(suggestion) = e.suggestion() {
        eprintln!("  Suggestion: {suggestion}");
    }
    std::process::exit(code);
}
```

### Continue-on-Error (ERR-02) -- `src/cli/run/processor.rs`

Current pattern (already working):
```rust
Err(e) => {
    errors_in_file += 1;
    log::warn!("{file_path} | {e:?}");
}
```

Change: The current `parse_record` errors are already non-fatal. The improvement is:
- Move exporter errors (write failures) to non-fatal as well — log and skip record instead of aborting file
- Wrap exporter call in `if let Err(e) = exporter_manager.export_one_preparsed(...)` with warning
- Let file-level errors (cannot open file) remain fatal

This separation needs:
- Distinguish "per-record export error" (non-fatal, continue) from "fatal I/O error" (abort)
- Add a counter for record-level export failures, shown in summary

### Stdin (PIPE-01) -- `src/cli/run/mod.rs`

Current discovery path:
```
SqllogParser::new(&cfg.sqllog.path).log_files()  // discovers .log files in directory
```

Change: Before file discovery, check for piped stdin:
```rust
use std::io::IsTerminal;

let use_stdin = cfg.sqllog.path == "-" || !std::io::stdin().is_terminal();
```

When stdin is detected:
- Skip file discovery
- Treat as a single "file" named `-` (stdin)
- The parser crate limitation: `LogParserBuilder::new("/dev/stdin").build()` works on Unix

**Platform concern:** `/dev/stdin` is Unix-only. The project primarily targets Unix environments (DaMeng database ecosystem). If Windows support is needed later, a conditional compilation branch or `from_bytes()` API on the parser would be the proper solution.

### Progress (UX-01) -- `src/cli/run/filter_processor.rs` + `processor.rs`

Change `make_progress_bar`:
```rust
use indicatif::{ProgressBar, ProgressStyle};

pub(super) fn make_progress_bar(quiet: bool) -> Option<ProgressBar> {
    if quiet { return None; }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} [{elapsed_precise}] {pos} records ({per_sec})")
            .unwrap()
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    Some(pb)
}
```

The `show_progress: bool` parameter changes to `show_progress: Option<&ProgressBar>` across the call chain. Each successful export does `if let Some(pb) = show_progress { pb.inc(1); }`.

For parallel mode: the `process_csv_parallel` function creates a `MultiProgress`, spawns one `ProgressBar` per file, and each rayon task updates its own bar.

### Better Help (UX-03) -- `src/cli/opts.rs`

Current:
```rust
#[command(
    name = "sqllog2db",
    version,
    about = "Parse DM database SQL logs and export to CSV/SQLite",
    long_about = "A lightweight and efficient CLI tool..."
)]
```

Add:
```rust
#[command(
    name = "sqllog2db",
    version,
    about = "Parse DM database SQL logs and export to CSV/SQLite",
    long_about = "A lightweight and efficient CLI tool for parsing DM database SQL logs (streaming) and exporting to CSV or SQLite.",
    after_help = "EXAMPLES:\n  sqllog2db run -c config.toml          # Export using config\n  cat log.sql | sqllog2db run -c cfg    # Pipe stdin\n  sqllog2db init -o config.toml         # Generate default config",
    max_term_width = 80,               # Readable on narrow terminals
)]
```

Add value hints for better shell completion:
```rust
#[arg(short = 'c', long = "config", default_value = "config.toml",
      value_hint = clap::ValueHint::FilePath)]
config: String,
```

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| `indicatif 0.18` | `rayon 1.x` (optional feature) | The `rayon` feature enables `ParallelProgressIterator`. Not needed if using `MultiProgress` manually. |
| `indicatif 0.18` | Any env_logger/log | indicatif draws to stderr by default by using `ProgressDrawTarget::Stderr`. Log output is also stderr. This works because indicatif manages cursor position. If log output conflicts, use `ProgressDrawTarget::Hidden` or increase log interval. |
| `--stdin` via `/dev/stdin` | Unix/macOS only | DaMeng runs on Linux, Windows DaMeng not common in practice. Acceptable for current scope. |

## Sources

- [crates.io indicatif 0.18.4 API](https://crates.io/api/v1/crates/indicatif) — version verified (published 2026-02-14), features documented
- [docs.rs indicatif](https://docs.rs/indicatif/latest/) — API docs, rayon integration, ProgressStyle templates (Context7 CLI lookup)
- [docs.rs miette 7.6.0](https://docs.rs/miette/latest/) — thiserror integration verified; dependency weight assessed (Context7 CLI lookup)
- [docs.rs clap](https://docs.rs/clap/latest/) — help_template, after_help, value_hint documentation (Context7 CLI lookup)
- [dm-database-parser-sqllog v1.1.0 source](https://github.com/guangl/dm-database-parser-sqllog/tree/v1.1.0) — Confirmed `LogParserBuilder::build()` only supports `fs::read(path)`. No `from_reader()` or `from_bytes()` API. `parse_record()` is public for manual record processing.
- [Rust std::io::IsTerminal](https://doc.rust-lang.org/stable/std/io/trait.IsTerminal.html) — Stable since Rust 1.70. MSRV 1.85 confirmed.
- `Cargo.toml` — existing deps verified: thiserror 2.0.18, clap 4.6.1 (features: derive, env), log/env_logger present
- `src/error.rs` — Current error types analyzed for enrichment opportunities
- `src/cli/run/processor.rs` — Continue-on-error pattern verified (parse errors already non-fatal)
- `src/cli/run/mod.rs` — Main orchestration analyzed for progress bar and stdin integration points
