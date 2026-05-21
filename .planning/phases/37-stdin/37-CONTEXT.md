# Phase 37: stdin 管道输入与错误实时输出 - Context

**Gathered:** 2026-05-21
**Status:** Ready for planning

## Phase Boundary

Add stdin pipe input support via automatic detection (non-TTY stdin → pipe mode), using `/dev/stdin` path mapping to work with the upstream `dm-database-parser-sqllog` crate. Skip file discovery and pre-scan in pipe mode. Transaction-level filters degrade with a warning. Error real-time stderr output already implemented in Phase 36.

## Implementation Decisions

### Stdin Detection
- **D-01:** Auto-detect stdin: when `std::io::stdin().is_terminal()` returns false (non-TTY), enable pipe mode
- **D-02:** Pipe mode uses `/dev/stdin` as the input path passed to `LogParserBuilder` (upstream crate only supports file paths via `fs::read`)

### Pre-scan and File Discovery
- **D-03:** Pipe mode skips `SqllogParser::log_files()` file discovery entirely
- **D-04:** Pipe mode skips pre-scan (transaction ID scanning) entirely

### Transaction Filter Conflict
- **D-05:** When transaction-level filters are configured AND pipe mode is active: emit warning to stderr + log, degrade to per-record matching (no "keep whole transaction" semantics)

### Error Output
- **D-06:** Non-fatal error stderr output already implemented in Phase 36 — no additional changes needed for UX-04

### Claude's Discretion
- Exact warning message text for transaction filter degradation
- Whether to add a `--input` flag as explicit alternative to auto-detection
- Exact integration point in `handle_run` for stdin path injection

## Canonical References

### Requirements
- `.planning/REQUIREMENTS.md` — PIPE-01 (stdin input), PIPE-02 (pre-scan skip + warn), UX-04 (stderr errors)

### Roadmap
- `.planning/ROADMAP.md` — Phase 37 details

### Research
- `.planning/research/ARCHITECTURE.md` — stdin integration analysis
- `.planning/research/PITFALLS.md` — Pitfall 2 (stdin + path conflict)

### Code
- `src/cli/run/mod.rs` — handle_run orchestration (integration point)
- `src/parser.rs` — SqllogParser file discovery
- `src/cli/run/processor.rs` — process_log_file (already supports /dev/stdin via LogParserBuilder)

## Existing Code Insights

### Integration Points
- `src/cli/run/mod.rs:handle_run()` — add stdin detection before file discovery, inject `/dev/stdin` path
- `src/cli/run/prescan.rs` — skip when pipe mode active

## Deferred Ideas

None.

---

*Phase: 37-stdin 管道输入与错误实时输出*
*Context gathered: 2026-05-21*
