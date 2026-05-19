---
phase: 22-github-pages
fixed_at: 2026-05-19T10:00:00Z
review_path: .planning/phases/22-github-pages/22-REVIEW.md
iteration: 1
findings_in_scope: 8
fixed: 6
skipped: 2
status: partial
---

# Phase 22: Code Review Fix Report

**Fixed at:** 2026-05-19T10:00:00Z
**Source review:** .planning/phases/22-github-pages/22-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 8
- Fixed: 6
- Skipped: 2

## Fixed Issues

### WR-01: Move asciicast demo.cast into mdBook source directory

**Files modified:** `site/src/asciicast/demo.cast` (moved from `site/asciicast/demo.cast`)

**Applied fix:** Moved `site/asciicast/` directory to `site/src/asciicast/` so that mdBook copies the `demo.cast` file to the build output. The relative path `asciicast/demo.cast` in index.md now correctly resolves.

### WR-02: Pin asciinema-player CDN from @latest to @3

**Files modified:** `site/src/index.md`

**Applied fix:** Changed CDN version from `@latest` to `@3.8.1` (resolved pinned version) in both the asciinema-player script src and CSS link href. This prevents silent breakage from future major releases.

### WR-03: Pin mdBook version from "latest" to explicit version

**Files modified:** `.github/workflows/pages.yml`

**Applied fix:** Changed `mdbook-version: "latest"` to `mdbook-version: "0.4.45"` to prevent unexpected breakage from future mdBook releases.

### IN-05: Add CSS rule for SVG responsiveness

**Files modified:** `site/theme/custom.css`

**Applied fix:** Added `.content svg { max-width: 100%; height: auto; }` rule to ensure inline SVGs scale properly on narrow viewports and do not overflow the content area.

### IN-06: Add concurrency group to deployment workflow

**Files modified:** `.github/workflows/pages.yml`

**Applied fix:** Added `concurrency: { group: pages, cancel-in-progress: true }` to the workflow to ensure sequential deployments and prevent race conditions when multiple pushes to main occur in quick succession.

### IN-02: Empty x-axis tick label in trend chart

**Files modified:** `site/src/index.md`

**Applied fix:** Removed the empty `x-axis <text>` element and its orphaned tick mark `<polyline>` at x=909 from the trend line chart SVG. The second tick position had no corresponding data point, so the label and tick were removed rather than left empty.

### IN-03: Duplicate SVG grid lines in latency histogram

**Files modified:** `site/src/index.md`

**Applied fix:** Removed 4 duplicate `<line>` elements for the left axis (x=80, was 5 identical lines) and 5 duplicate `<line>` elements for the right axis (x=1179, was 6 identical lines) in the latency histogram SVG. Kept one copy of each unique grid line.

## Skipped Issues

### IN-01: Trend line chart rendered with only a single data point

**File:** `site/src/index.md:551-553`

**Reason:** requires chart regeneration (multi-bucket data needs plotters library re-run), not a straightforward SVG markup edit.

**Original issue:** The trend line chart SVG contains a polyline with only one coordinate pair and a single circle marker, making the line invisible.

### IN-04: Large inline SVGs embedded in markdown harm maintainability

**File:** `site/src/index.md:88-614`

**Reason:** requires significant restructuring (extract SVGs to separate files, update markdown references), not a straightforward edit. Chart regeneration tools would need to output SVG files directly.

**Original issue:** Approximately 530 lines of inline SVG markup across four charts makes the markdown difficult to navigate and edit.

---

_Fixed: 2026-05-19T10:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
