---
phase: 21
plan: 01
subsystem: README
tags: [documentation, readme, charts]
requires: []
provides: [README.md, docs/images/frequency_bar.png, docs/images/latency_histogram.png]
affects: [documentation]
tech-stack:
  added: []
  patterns: [Mermaid.js architecture diagram, shields.io badges, embedded PNG chart screenshots]
key-files:
  created:
    - docs/images/frequency_bar.png
    - docs/images/latency_histogram.png
  modified:
    - README.md
decisions:
  - Use rsvg-convert (librsvg) for SVG-to-PNG conversion since ImageMagick had font rendering issues on macOS
  - Remove Chinese parenthetical "(达梦)" from project description for pure English compliance per D-02
metrics:
  duration: "~15 min"
  completed_date: "2026-05-19"
  plan_lines: 454
  total_commits: 3
  total_files_modified: 3
  tasks_total: 3
  tasks_completed: 3
---

# Phase 21 Plan 01: README Rewrite Summary

**One-liner:** Rewrote README.md from 395-line mixed Chinese/English to 208-line pure English minimal skeleton covering all v1.3 template analysis and v1.4 nested config features.

## What Was Built

- **README.md** (208 lines, pure English): Complete project README with badges, architecture, feature overview, installation, QuickStart, configuration example, performance benchmarks, error handling, SVG chart screenshots, link index with status markers, and license footer.
- **docs/images/frequency_bar.png** (PNG, 1200x600): Frequency bar chart screenshot generated from sample Dameng SQL logs via template analysis pipeline and converted with rsvg-convert.
- **docs/images/latency_histogram.png** (PNG, 1200x600): Latency histogram screenshot from the same sample logs.

## Task Summary

### Task 1: Header through Feature Overview

- Wrote project header with 6 shields.io badges (crates.io, downloads, CI, license, release, Rust 1.85+)
- Four-domain feature overview: Parsing & Export, Filtering & Field Control, Template Analysis & Charts, Configuration & Performance
- Architecture section with data flow description and Mermaid diagram
- Installation with crates.io and local build options
- **Commit:** `1fac7e5`

### Task 2: QuickStart, Config, Charts, Screenshots, Links

- Generated representative SVG charts from 3 sample DM log files (~2.37M records) using `sqllog2db run`
- Converted SVGs to PNGs via rsvg-convert (librsvg) because ImageMagick had font rendering issues
- Added QuickStart section with 3 core commands and time-range filtering examples
- Added TOML config snippet matching actual v1.4 nested format
- Added Mermaid.js architecture diagram replacing ASCII art
- Added performance benchmark table (CSV synthetic 5.2M rec/s, SQLite 1.1M rec/s, real file 1.55M rec/s)
- Added SVG Charts section with embedded PNG screenshots and Gallery link
- Added Error Handling section with exit code reference
- Added Link index with "(Coming in Phase 23)" and "(Coming v1.6)" status markers
- Added Apache-2.0 license footer
- **Commit:** `0aa9d86`

### Task 3: Verification and Consistency Check

- Verified config snippet matches actual `sqllog2db init` output format (v1.4 nested: `[filter]` top-level, `[filter.include]` sub-table)
- Verified line count 208 (within 200-250 target)
- Verified zero Chinese characters (pure English)
- Verified all deferred doc links have proper status markers
- Verified both PNGs are valid (confirmed by `file` command: "PNG image data, 1200x600, 8-bit/color RGB")
- Verified all 933 tests pass (`cargo test`), cargo clippy passes with no new warnings
- **No file changes needed** — all acceptance criteria satisfied without discrepancy fixes

## Deviations from Plan

None — plan executed exactly as written. One noteworthy implementation detail:

- **ImageMagick-to-librsvg substitution**: The plan specified ImageMagick `convert` for SVG-to-PNG conversion. On macOS, ImageMagick 7's `convert` command (deprecated in IMv7) failed with font rendering errors because the SVGs use `font-family="sans-serif"` and ImageMagick could not resolve system fonts. Installed `librsvg` via Homebrew and used `rsvg-convert` successfully.

## Known Stubs

None. All README content is substantive; no placeholder text or TODO markers.

## Threat Flags

None. All created/modified files are within the plan's documented threat model boundaries (README.md documentation and chart PNGs from sample logs).

## Verification

```
Line count: 208 (target: 200-250)                         OK
No Chinese characters                                       OK
6 badges at top                                             OK
Mermaid architecture diagram                                OK
Config snippet matches actual init output                   OK
QuickStart: 3 core commands                                 OK
Performance table with benchmarks                           OK
Link index with status markers                              OK
2 embedded PNG screenshots                                  OK
Gallery link for remaining chart types                      OK
Apache-2.0 license reference                                OK
No bare links to deferred docs                              OK
file docs/images/frequency_bar.png: PNG image data          OK
file docs/images/latency_histogram.png: PNG image data      OK
cargo clippy --all-targets -- -D warnings: passes           OK
cargo test: 933 passed                                      OK
```

---

## PLAN COMPLETE

**Plan:** 21-readme-01
**Tasks:** 3/3 completed
**Commits:**
- `1fac7e5`: feat(21-readme): write README header through feature overview
- `0aa9d86`: feat(21-readme): add QuickStart, config, performance table, chart screenshots, links
- (Task 3: No changes needed — verification passed without fixes)
