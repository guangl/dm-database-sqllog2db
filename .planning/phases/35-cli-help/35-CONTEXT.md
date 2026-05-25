# Phase 35: CLI --help 增强 - Context

**Gathered:** 2026-05-21
**Status:** Ready for planning

## Phase Boundary

Enhance `--help` output for sqllog2db CLI using clap's `after_help` and `long_about` attributes. Add 4-5 cargo-style usage examples covering common DaMeng scenarios, deepen subcommand descriptions, and add help text to key arguments. No new dependencies — pure clap derive attribute changes in `src/cli/opts.rs`.

## Implementation Decisions

### Examples Content
- **D-01:** Provide 4-5 examples covering: basic export, export with SQL indicators filter, config generation (init), config validation, and stdin pipe input (placeholder for Phase 37)
- **D-02:** One example should demonstrate `--config` flag usage with custom path

### Examples Placement
- **D-03:** Top-level `sqllog2db --help` shows general examples (init, validate, basic run)
- **D-04:** Each subcommand (`run --help`, `init --help`, `validate --help`) has its own `after_help` with subcommand-specific examples

### Help Language
- **D-05:** All help text in English

### Examples Format
- **D-06:** cargo/crates.io convention style — no `$ ` prompt prefix, indented descriptions above or beside commands

### Description Depth
- **D-07:** Expand subcommand doc comments to `long_about` for Run, Init, Validate
- **D-08:** Add `help` text to key arguments (e.g., `--config` describes the TOML format, `--output` describes typical use)
- **D-09:** Do NOT add `value_hint` — user explicitly declined shell completion hints

### Pipe Input Examples
- **D-10:** Reserve placement for stdin pipe examples in Phase 35 (add comment markers in code), but defer actual pipe example text to Phase 37

### Configuration Reference
- **D-11:** Add a brief reference to config file sections in `sqllog2db run --help` or top-level `after_help`: mention `[csv]`, `[sqlite]`, `[pipeline]` as the three main config sections

### Claude's Discretion
- Exact wording of help text and examples
- Specific example command arguments (realistic but not misleading)
- Exact placement of config section reference

## Canonical References

### Requirements
- `.planning/REQUIREMENTS.md` — UX-03: `--help` 输出包含达梦场景实用示例

### Roadmap
- `.planning/ROADMAP.md` — Phase 35 details and success criteria (4 criteria)

### Code
- `src/cli/opts.rs` — Current clap configuration (modification target)

## Existing Code Insights

### Reusable Assets
- clap 4.6.1 derive macros (`Parser`, `Subcommand`, `arg`) already configured
- Existing `about`, `long_about`, doc comments provide baseline text

### Established Patterns
- Doc comments (`///`) used as help text for structs and variants
- `#[arg(short, long, default_value, env)]` pattern for argument configuration
- `after_help` not yet used — new addition

### Integration Points
- `src/cli/opts.rs` — sole modification target for this phase (lines 1-57)
- `Cargo.toml` — verify clap `derive` feature enabled (already is)

## Specific Ideas

- Phase 37 will add `--input` flag to Run command; Phase 35 should reserve example placement for it in comments like `<!-- TODO(Phase 37): add stdin pipe example -->` (but as Rust comments, not HTML)
- The `[pipeline]` section reference in help should mention filters: `include`, `exclude`, `indicators`, `sql`

## Deferred Ideas

None — discussion stayed within phase scope.

---

*Phase: 35-CLI --help 增强*
*Context gathered: 2026-05-21*
