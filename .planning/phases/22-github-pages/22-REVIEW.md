---
phase: 22-github-pages
reviewed: 2026-05-19T08:30:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - site/book.toml
  - site/src/index.md
  - site/src/SUMMARY.md
  - site/theme/custom.css
  - .github/workflows/pages.yml
findings:
  critical: 0
  warning: 3
  info: 5
  total: 8
status: issues_found
---

# Phase 22: Code Review Report — GitHub Pages (mdBook + Deployment)

**Reviewed:** 2026-05-19T08:30:00Z
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

Reviewed the mdBook documentation site infrastructure and GitHub Pages deployment pipeline for sqllog2db. The 5 files include: mdBook configuration (`book.toml`), landing page (`index.md`), book navigation (`SUMMARY.md`), custom theme styling (`custom.css`), and GitHub Actions workflow (`pages.yml`).

**Key concerns:**
- The asciicast `demo.cast` file is located outside mdBook's `src/` directory, so it won't be copied to the build output, resulting in a broken embedded player on the deployed site.
- Two un-pinned `"latest"` version dependencies (CDN asciinema-player and mdBook) may cause silent breakage on future updates.
- Three of the four SVG charts contain rendering artifacts or incomplete data that degrade the landing page's professional appearance.

No critical security or correctness issues were found. The deployment pipeline follows standard patterns and securely manages credentials.

---

## Warnings

### WR-01: Asciicast `demo.cast` file outside mdBook source directory — broken player on deploy

**File:** `site/src/index.md:627-629`
**Issue:** The asciinema player is embedded with `<asciinema-player src="asciicast/demo.cast">` and the markdown link is `[demo.cast](asciicast/demo.cast)`. Both resolve relative to the page URL. mdBook copies only files from the `src/` subdirectory to the output, but the `demo.cast` file is at `site/asciicast/demo.cast` (outside `src/`). The file at `site/src/asciicast/demo.cast` does not exist. On deployment, the embedded player will attempt to load `https://guangl.github.io/sqllog2db/asciicast/demo.cast` and receive a 404, breaking the demo section.

**Fix:** Move the asciicast file into the mdBook source directory so it is included in the build output:
```bash
mv site/asciicast site/src/asciicast
```
The relative path `asciicast/demo.cast` in the markdown will then correctly resolve to `site/book/asciicast/demo.cast` in the build output.

---

### WR-02: CDN `@latest` version pin for asciinema-player (script + CSS)

**File:** `site/src/index.md:624-625`
**Issue:** Both the asciinema-player script and its CSS stylesheet are loaded from jsdelivr CDN using the `@latest` version tag:
```html
<script src="https://cdn.jsdelivr.net/npm/asciinema-player@latest/dist/bundle/asciinema-player.min.js"></script>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/asciinema-player@latest/dist/bundle/asciinema-player.css">
```
If the asciinema-player project releases a breaking change (e.g., syntax changes for the `<asciinema-player>` custom element, attribute renames, or CSS breaking changes), the embedded player will break on the next page load without any code change in this repository. This also means different visitors may see different behavior depending on when the CDN cache expires.

**Fix:** Pin to a specific major version tag (e.g., `@3`), matching the current `@latest` at the time of deployment:
```html
<script src="https://cdn.jsdelivr.net/npm/asciinema-player@3/dist/bundle/asciinema-player.min.js"></script>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/asciinema-player@3/dist/bundle/asciinema-player.css">
```

---

### WR-03: Unpinned mdBook version in GitHub Actions workflow

**File:** `.github/workflows/pages.yml:22`
**Issue:** The `peaceiris/actions-mdbook` action is configured with `mdbook-version: "latest"`. A future mdBook major release may introduce breaking changes to the TOML configuration schema, CLI flags, or output directory structure, causing the deployment pipeline to fail unexpectedly with no code changes from this repository.

**Fix:** Pin to a specific mdBook version that is known to work with the current configuration:
```yaml
mdbook-version: "0.4.45"
```
Or use a minor version range to receive patch updates while avoiding major breakage:
```yaml
mdbook-version: "~0.4"
```
(The `~` prefix semantics depend on the action's version resolution logic; if the action does not support semver ranges, use an explicit version string.)

---

## Info

### IN-01: Trend line chart rendered with only a single data point

**File:** `site/src/index.md:551-553`
**Issue:** The "SQL Execution Trend by Hour" SVG chart contains a polyline with only one coordinate pair (`points="369,89 "`) and a single circle marker. A trend line chart with one data point cannot show a trend — the line is not rendered at all. Combined with the empty second x-axis label (IN-02), this chart appears visually broken and fails to demonstrate the trend analysis feature.

**Fix:** Regenerate the trend chart with multi-bucket data (at least 6-12 hours of data). The plotters library used by the tool should receive a representative dataset.

---

### IN-02: Empty x-axis tick label in trend chart

**File:** `site/src/index.md:547-549`
**Issue:** The second x-axis `<text>` element has no content between the opening and closing tags:
```svg
<text x="909" y="530" dy="0.76em" text-anchor="middle" font-family="sans-serif" font-size="8.870967741935484" opacity="1" fill="#000000" transform="rotate(90, 909, 530)">

</text>
```
An empty label at the second tick position leaves the reader guessing what time period the data covers.

**Fix:** Populate the label with the corresponding hour value, or remove the unused tick element.

---

### IN-03: Duplicate SVG grid lines in latency histogram

**File:** `site/src/index.md:289-304`
**Issue:** Multiple `<line>` elements with identical start/end coordinates appear consecutively:
```svg
<line opacity="0.1" stroke="#000000" stroke-width="1" x1="80" y1="539" x2="80" y2="45"/>
<line opacity="0.1" stroke="#000000" stroke-width="1" x1="80" y1="539" x2="80" y2="45"/>
<line opacity="0.1" stroke="#000000" stroke-width="1" x1="80" y1="539" x2="80" y2="45"/>
```
(Repeated 5+ times for the left axis, and 6+ times for the right axis.) This is a plotters generation artifact. While it does not affect rendering (same visual result), it adds unnecessary bytes to the page and indicates a suboptimal SVG output configuration.

**Fix:** Deduplicate the SVG generation to avoid writing identical overlapping elements. This is a plotters configuration issue — review the SVG backend settings for the chart renderer.

---

### IN-04: Large inline SVGs embedded in markdown harm maintainability

**File:** `site/src/index.md:88-614`
**Issue:** The index.md file is 644 lines long, of which approximately 530 lines are inline SVG markup spanning four charts. The SVGs use hardcoded pixel dimensions (1200x600 or 1000x600), and any chart updates require regenerating the entire SVG string. This makes the markdown difficult to navigate and edit.

**Fix:** Extract each SVG into a separate file under `site/src/images/` or `site/src/charts/` (e.g., `frequency.svg`, `latency.svg`, `trend.svg`, `user-pie.svg`). Reference them using standard `<img>` tags:
```markdown
<img src="charts/frequency.svg" alt="Top 10 SQL Templates by Frequency" width="100%">
```
mdBook automatically copies image files from `src/` to the build output. This also allows the browser to cache SVG files independently from the HTML page.

---

### IN-05: Inline SVGs may overflow on mobile viewports

**File:** `site/theme/custom.css`
**Issue:** All four SVG charts have fixed dimensions (width="1200" height="600"). The custom CSS does not include responsive sizing rules for inline SVGs. On narrow viewports (mobile/tablet), the SVGs will overflow the content area because there is no `max-width: 100%` constraint for inline SVGs.

**Fix:** Add the following rule to `site/theme/custom.css`:
```css
.content svg {
  max-width: 100%;
  height: auto;
}
```

---

### IN-06: Missing concurrency control in deployment workflow

**File:** `.github/workflows/pages.yml:11`
**Issue:** The workflow has no `concurrency` configuration. If two pushes to `main` occur in quick succession (both triggering the `paths: ["site/**"]` filter), two deployments will run concurrently. The `peaceiris/actions-gh-pages` action with `force_orphan: true` uses force-push, so the last deployment to complete will overwrite the first — potentially causing a brief window where an earlier deployment's content is replaced before the later one finishes.

**Fix:** Add a concurrency group to ensure sequential deployments:
```yaml
concurrency:
  group: pages
  cancel-in-progress: true
```

---

_Reviewed: 2026-05-19T08:30:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
