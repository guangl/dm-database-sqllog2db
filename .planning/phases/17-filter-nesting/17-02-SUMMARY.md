# Plan 17-02 Summary: Update Filter Callers + Init Template

**Phase:** 17-filter-nesting
**Plan:** 02
**Status:** Complete
**Commits:**
- `eb51a1c` feat(17-02): update filter callers to new nested API
- `653a34a` feat(17-02): regenerate init template with nested filter format

---

## What Was Built

### Task 1: Update Callers

Updated all caller files to use the new nested `FiltersFeature` API from Plan 17-01:

**src/config.rs:**
- Replaced `CompiledMetaFilters::try_from_meta(&filters.meta)` with `try_from_include_exclude(&filters.include, &filters.exclude)` (2 call sites in `validate()` and `validate_and_compile()`)
- Updated error message assertions in tests to use new path `features.filters.include.users`
- Added `test_validate_new_nested_format_passes` — validates new `[features.filters.include]` / `[features.filters.exclude]` TOML format
- Added `test_validate_old_flat_format_passes` — validates old flat `[features.filters]` TOML format (backward compat)

**src/cli/run.rs:**
- Updated `FilterProcessor::new`: `filter.meta.start_ts` → `filter.include.start_ts`, `filter.meta.end_ts` → `filter.include.end_ts`
- Updated `recompile_meta_if_needed`: `try_from_meta(&filters.meta)` → `try_from_include_exclude(&filters.include, &filters.exclude)`

**src/cli/show_config.rs, stats.rs, validate.rs, main.rs:**
- Updated all remaining references from old `filters.meta.*` field paths to new `filters.include.*` / `filters.exclude.*`

**tests/integration.rs:**
- Replaced all `FiltersFeature { meta: MetaFilters { ... } }` literals with `FiltersFeature { include: IncludeFilters { ... }, exclude: ExcludeFilters { ... }, ... }`
- Replaced `SqlFilters { include_patterns: ..., exclude_patterns: ... }` with `SqlFilters { includes: ..., excludes: ... }`
- Added import `use dm_database_sqllog2db::features::filters::{ExcludeFilters, IncludeFilters}`

### Task 2: Init Template Update

**src/cli/init.rs:**
- Replaced old flat filter section in `CONFIG_TEMPLATE_ZH` (Chinese) with new nested format:
  - `[features.filters.include]` — users/ips/sessions/threads/statements/apps/tags/start_ts/end_ts/trxids
  - `[features.filters.exclude]` — users/ips/sessions/threads/statements/apps/tags
  - `[features.filters.indicators]` — exec_ids/min_runtime_ms/min_row_count (unchanged)
  - `[features.filters.sql]` — includes/excludes (new field names)
- Applied same transformation to `CONFIG_TEMPLATE_EN` (English template)
- All old field names (usernames, client_ips, sess_ids, appnames, include_patterns, exclude_patterns) removed from templates

**tests/integration.rs:**
- Added `test_init_generates_new_nested_format`: runs `handle_init`, verifies generated file contains `[features.filters.include]` / `[features.filters.exclude]` / `[features.filters.indicators]` / `[features.filters.sql]`, parses with `toml::from_str`, calls `cfg.validate()` — all pass

---

## Verification Results

- `cargo test` — 51 tests pass (417 unit + 51 integration)
- `cargo clippy --all-targets -- -D warnings` — zero warnings
- `cargo run -- init -o /tmp/test.toml --force && cargo run -- validate -c /tmp/test.toml` — exit 0 ✓
- `cargo run -- validate -c config.toml` (old flat format) — exit 0 ✓ (backward compat)
- No remaining references to `try_from_meta`, `filters.meta`, or `MetaFilters` in caller files
