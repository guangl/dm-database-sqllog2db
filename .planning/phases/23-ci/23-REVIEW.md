---
phase: 23-ci
reviewed: 2026-05-19T12:00:00Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - docs/quickstart.md
  - docs/config-reference.md
  - site/src/index.md
  - .github/workflows/lychee.yml
findings:
  critical: 0
  warning: 4
  info: 4
  total: 8
status: issues_found
---

# Phase 23: Code Review Report

**Reviewed:** 2026-05-19T12:00:00Z
**Depth:** standard
**Files Reviewed:** 4
**Status:** issues_found

## Summary

Reviewed four files for phase 23-ci: two documentation files (QuickStart guide and Config Reference), the mdBook site landing page, and the lychee link checker CI workflow. The YAML workflow is correct and well-structured. Documentation files contain several accuracy issues and inconsistencies that could confuse users. The most significant concerns are a unit mismatch between latency bucket configuration (ms) and chart output labels (us), and a CDN floating version dependency on the site page.

---

## Warnings

### WR-01: Latency unit mismatch between config input and chart output

**File:** `docs/quickstart.md:215` and `site/src/index.md:289`

**Issue:** The QuickStart guide defines `latency_buckets` values in milliseconds (ms) as confirmed by the Config Reference table which states "Latency histogram bucket boundaries (ms)". However, the SVG chart on the site landing page displays its axis as "(us, log scale)" — microseconds. A user who configures `latency_buckets = [1, 5, 10]` expecting 1ms, 5ms, 10ms thresholds will see the chart axis labeled in microseconds and will not be able to reconcile the two. If the tool internally converts ms to us before charting, the documentation should explain this. If it does not convert, the chart label is wrong.

**Fix:** Unify the unit across all three sources. Either change the chart label from "(us)" to "(ms)" and adjust axis values accordingly, or update the Config Reference to say "us" and add a note about the unit conversion. The quickstart scenario showing both sides should explicitly state which unit the user is working in.

---

### WR-02: CDN floating version tag for asciinema-player

**File:** `site/src/index.md:624-625`

**Issue:** The asciinema-player script and stylesheet are loaded from `cdn.jsdelivr.net/npm/asciinema-player@latest/dist/bundle/`. The `@latest` tag means the resolved version changes whenever the package publishes a new release. If a future major version (e.g., v3.0) introduces breaking changes to the player API, the script element attributes, or the .cast file format, the embedded demo will silently break. This is a stability risk for a published site.

**Fix:** Pin to a specific version. For example:
```html
<script src="https://cdn.jsdelivr.net/npm/asciinema-player@3.8.1/dist/bundle/asciinema-player.min.js"></script>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/asciinema-player@3.8.1/dist/bundle/asciinema-player.css">
```

---

### WR-03: `--group-by user` naming inconsistency with data model

**File:** `docs/quickstart.md:189`

**Issue:** The `sqllog2db stats --group-by user` flag uses lowercase `user`, while the filter fields and likely the underlying data model use `USERNAME` (uppercase, as documented in `docs/config-reference.md:72` and the QuickStart's own SQLite query examples at lines 139-152). A user who tries `--group-by USERNAME` (matching the field name they learned from other docs) or `--group-by user` (matching the quickstart) may get an error or zero results if the CLI argument is case-sensitive and mismatched.

**Fix:** Either:
1. If the CLI flag accepts both forms, document that explicitly.
2. If it only accepts one form, ensure all documentation uses the same form. The QuickStart should match the convention established in the Config Reference and SQLite examples.

---

### WR-04: `start_ts` / `end_ts` filter fields lack section placement clarity

**File:** `docs/config-reference.md:72-79`

**Issue:** The filter fields table lists `start_ts` and `end_ts` alongside `USERNAME`, `APPGROUP`, etc. The visual header for this table is `[filter.include] and [filter.exclude]`, which implies all listed fields live inside those subtables. However, time-range filters (`start_ts`, `end_ts`) semantically belong at the `[filter]` level, not inside include or exclude blocks — they are a global time window, not an AND/OR filter rule. Additionally, the TOML example at lines 54-68 does not show `start_ts` or `end_ts` at all, so users have no concrete placement guide. The `max_record_limit` description correctly notes it is "at `[filter]` level", but the same clarification is missing for the timestamp fields.

**Fix:** Add a `[filter]`-level TOML example showing `start_ts` and `end_ts`, and update the table descriptions to explicitly state their section placement (e.g., "at `[filter]` level — not under include or exclude"). Alternatively, split the table into two: one for `[filter]`-level fields and one for include/exclude sub-fields.

---

## Info

### IN-01: Pie chart includes zero-percentage user slices

**File:** `site/src/index.md:600-610`

**Issue:** The SVG pie chart example shows user segments with `0.0%` share (XINGLIN, NGYH, GYS). These segments produce vanishingly thin polygon slices (lines 572-575 repeat identical coordinates) that serve only as visual noise. If the tool's chart generator produces zero-percentage slices in practice, this is a usability issue in the chart generation. As an example in documentation, including these slices sets a poor expectation for output quality.

**Fix:** Either regenerate the example chart with the zero-percentage users excluded, or document that `top_n` controls how many segments are shown (preventing 0.0% entries from appearing).

---

### IN-02: Trend line chart shows only a single data point

**File:** `site/src/index.md:551-553`

**Issue:** The trend line chart SVG contains only one data point at coordinate `369,89` — a single circle with no connecting trend line. This does not demonstrate the "trend" (time-series analysis) feature at all. A visitor viewing this chart will come away thinking the tool can only plot one snapshot.

**Fix:** Replace with an SVG that shows at least 5-10 data points across multiple hours/days to demonstrate trend analysis.

---

### IN-03: CSV exporter `delimiter` field missing from TOML example

**File:** `docs/config-reference.md:147-162`

**Issue:** The `delimiter` field is documented in the CSV exporter table (line 162, default `,`) but the adjacent TOML config example (lines 147-154) does not show it. Users who want to change the delimiter (e.g., to semicolon for European locales) have to guess the correct TOML syntax for a char value.

**Fix:** Add `delimiter = ","` (or a commented-out variant) to the TOML example block so users can see the syntax.

---

### IN-04: QuickStart references CLI subcommands absent from Config Reference

**File:** `docs/quickstart.md:166,189,192,196,252,305`

**Issue:** The QuickStart guide frequently references `sqllog2db stats` and `sqllog2db digest` CLI subcommands (Scenarios 3 and 4, plus Troubleshooting). The Config Reference document does not document CLI subcommands at all — it focuses exclusively on TOML config file options. A reader who uses the Config Reference as their primary documentation will not know these commands exist. This is a content completeness gap between the two documentation files.

**Fix:** Either:
1. Add a CLI commands reference section to `docs/config-reference.md` or a separate `docs/cli-commands.md`, and cross-link from the QuickStart.
2. If the `stats` and `digest` subcommands do not actually exist in the binary, remove these references from the QuickStart to prevent user errors.

---

## Structural Findings (fallow)

No structural pre-pass was provided for this review.

---

_Reviewed: 2026-05-19T12:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
