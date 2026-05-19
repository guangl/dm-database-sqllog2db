# Project Research Summary

**Project:** sqllog2db -- v1.5 Documentation and GitHub Pages
**Domain:** Rust CLI tool documentation -- documentation overhaul, static site generation, GitHub Pages landing page
**Researched:** 2026-05-18
**Confidence:** MEDIUM (high confidence on individual research areas, but tension exists between documentation scope approaches)

## Executive Summary

sqllog2db is a mature Rust CLI tool for parsing DaMeng (达梦) database SQL logs, now at v1.4 with significant features (template analysis, SVG chart generation, complex filtering) that are completely undocumented in its README. The v1.5 milestone is a documentation-only release with no code changes. Research confirms this is critically needed: the current README references v0.x-era flat configuration, links to 4 non-existent files, and omits all v1.3/v1.4 features. Rust CLI users have clear documentation expectations (QuickStart, --help embedding, CHANGELOG, LICENSE, badges) that are currently unmet.

**The central tension in the research is the scope of the GitHub Pages landing page.** STACK.md and ARCHITECTURE.md recommend mdBook (Rust-native, full-featured static site generator) as the long-term solution, building a multi-page site with Chinese-first navigation, search, and integrated charts. FEATURES.md and PITFALLS.md argue this is over-engineering for v1.5 and recommend deferring the full mdBook site to v2+, keeping v1.5 to a simple single-page landing page plus README overhaul.

**Recommended resolution:** Use mdBook as the SSG (Rust-native, zero Node.js, well-documented deployment patterns) but deploy a minimal v1.5 site (single landing page + 2-3 supporting pages). Do NOT build the full multi-page site with SUMMARY.md navigation yet. The key deliverable for v1.5 is the README overhaul -- it reaches users on GitHub, crates.io, and every `cargo install` path. The landing page is secondary. The key risks are (1) documentation drift between README and the new pages site, (2) over-engineering the landing page with JS framework dependencies, and (3) creating a "three-body problem" where README, docs/, and pages all contain overlapping content that drifts apart.

## Key Findings

### Recommended Stack

From [STACK.md](STACK.md): The recommended stack is **mdBook** (v0.5.2+) for the static site, **GitHub Actions** for CI/CD, **GitHub Pages** for hosting, and **lychee** (v0.24.2) for link checking. The "simplest possible setup" variant (no Node.js, pure Rust) is the recommended approach.

**Core technologies:**
- **mdBook 0.5.2+**: Static site generator -- Rust-native, built-in search/syntax highlighting/theming, outputs plain HTML to `book/` for trivial Pages deployment. Already the Rust ecosystem standard (used by The Rust Book, Rust Reference).
- **GitHub Actions + peaceiris/actions-gh-pages@v4**: CI/CD deployment -- reuses existing CI/release workflow patterns. `force_orphan: true` ensures clean single-commit `gh-pages` branch history.
- **GitHub Pages**: Static site hosting -- free, automatic, zero operational overhead, direct Actions integration.
- **lychee 0.24.2**: Link checker -- Rust-native, async, GitHub Action available (`lycheeverse/lychee-action`), `.lycheeignore` for known-broken links.
- **asciinema 3.2.0**: Terminal session recording -- already installed on dev machine, lightweight asciicast format (~8% of video), embeddable web player.

**Key alternative considered:** VitePress/Hugo/Jekyll/Docusaurus all add non-Rust runtime dependencies (Node.js, Ruby, Go) and are explicitly NOT recommended. `markdownlint-cli2` requires Node.js but is recommended for linting quality; if pure-Rust is desired, `rumdl` (v0.1.94) is an alternative with fewer rules.

### Expected Features

From [FEATURES.md](FEATURES.md): Features are organized by priority for the v1.5 documentation milestone. The primary audience is Chinese DaMeng DBAs.

**P1 -- Must have (current README gaps):**
- **README full rewrite** -- current README references v0.x-era config, omits v1.3 (template analysis) and v1.4 (nested config) features entirely
- **CHANGELOG.md** -- missing v1.0-v1.4 entries (only exists from v0.10.7), create via Keep a Changelog format
- **LICENSE file** -- missing from repo root, enterprise users will filter the project out
- **Project badges** -- CI status, crates.io version, license (shields.io, 4-6 badges max)
- **QuickStart examples** -- 3-5 copy-paste commands covering `init` + `run` + `digest` + `stats` + `validate`

**P2 -- Should have (differentiators):**
- **GitHub Pages basic landing page** -- single page with project hero, install, feature overview, performance data
- **Performance benchmarks** -- table showing 5.2M/s synthetic CSV throughput + 1.55M/s on 1.1GB real file + constant memory curve
- **Architecture / data flow diagram** -- Mermaid.js diagram explaining streaming parser -> pipeline -> exporter architecture
- **docs/quickstart.md** -- more detailed QuickStart than README

**P3 -- Nice to have:**
- SVG chart gallery on landing page (4 real generated charts)
- Full configuration reference (`docs/config-reference.md`)
- Asciicast embedded demo (30-second recording of `sqllog2db run`)

**Deferred (v1.6+):**
- Full multi-page mdBook site with navigation
- Chinese-English bilingual documentation
- WebAssembly/Playground demo
- CONTRIBUTING.md, SECURITY.md, FAQ page
- Custom domain

### Architecture Approach

From [ARCHITECTURE.md](ARCHITECTURE.md): Three architecture sections were researched (v1.2 code improvements, v1.3 template analysis/charts, and v1.5 documentation). For v1.5, the architecture is **two independent documentation systems**: (1) `docs/` -- project Markdown documentation (versioned with code), and (2) `site/` -- mdBook source for the GitHub Pages static site (build artifact, only rendered HTML is deployed).

**Major components:**
1. **`docs/` directory** -- QuickStart, architecture guide, configuration reference, FAQ. Plain Markdown, versioned with code, excluded from crate via Cargo.toml.
2. **`site/` directory** -- mdBook project with `book.toml`, `src/SUMMARY.md`, and individual page Markdown. Source is in main branch; built output (`site/book/`) is gitignored.
3. **GitHub Actions workflows** -- `pages.yaml` (new, path-filtered deploy) + new `docs` job in `ci.yaml` (validate mdBook build on every PR).

**Key architectural decisions:**
- `site/` is separate from `docs/` to avoid namespace collision (mdBook expects `src/SUMMARY.md` at its root)
- Symlinks from `site/src/` to root CHANGELOG.md / CONTRIBUTING.md avoid content duplication
- Branch-based deployment via `peaceiris/actions-gh-pages@v4` (not native `actions/deploy-pages`) for simplicity
- `paths` filter on `site/**` avoids rebuilding docs on pure code commits
- Chinese-first navigation labels (target audience is DM DBAs)

### Critical Pitfalls

From [PITFALLS.md](PITFALLS.md): 10 documented pitfalls. Top 5 for roadmap:

1. **Documentation Drift (P1)** -- Current README is 2 major versions behind. Documentation MUST be validated against actual CLI behavior before publishing. Mitigation: run `cargo run -- init` and `cargo run -- --help` to verify every config example and command output in the documentation.

2. **Over-engineering the Landing Page (P4)** -- Heavy Risk. CLI tool documentation does NOT need React/VitePress/Docusaurus. Set a hard rule: "the landing page must be buildable without `npm install`." mdBook (Rust binary) or plain HTML satisfy this. Full multi-page mdBook site is deferred.

3. **Three-body Problem of Documentation (P5)** -- Maintaining README + docs/ + GitHub Pages as three content sources creates sync burden. Solution: README is the single source of truth. Landing page should NOT duplicate README content -- it should enhance (visual showcase, benchmarks, hero section) and redirect to README for details.

4. **Stale Config Examples (P3)** -- Landing page config examples that hard-code deprecated formats. Mitigation: only show the minimal config on the landing page; point to `cargo run -- init` output as the canonical config reference. Or better: don't show full config on the landing page at all.

5. **Link Rot (P2)** -- Current README links to 4 non-existent files (docs/quickstart.md, docs/architecture.md, CONTRIBUTING.md, SECURITY.md). Mitigation: (a) fix or remove all broken links in Phase 1, (b) add lychee CI check in Phase 3 to prevent future rot.

**Additional notable pitfalls:** Cargo.toml missing `documentation` field (P6 -- fix after Pages deploy), crates.io README compatibility (P9 -- use absolute raw.githubusercontent.com URLs for images), no doc maintenance workflow (P8 -- establish PR checklist for doc updates).

## Implications for Roadmap

Based on combined research, the v1.5 milestone should be structured in 3 phases with careful dependency ordering:

### Phase 1: README Overhaul + Root Documents

**Rationale:** Highest impact with lowest cost. README is the single source of truth and reaches users on GitHub, crates.io, and every search result. All other documentation references it. No build tools or deployment infrastructure needed.

**Delivers:**
- Full README rewrite synchronized with v1.3/v1.4 features (configuration, template analysis, SVG charts, filtering)
- CHANGELOG.md (Keep a Changelog format, backfill v1.0-v1.4)
- LICENSE file (MIT or Apache-2.0)
- shields.io badges (CI, crates.io version, license)
- QuickStart section with 5 copy-paste commands

**Addresses FEATURES.md:** P1 items (README, CHANGELOG, LICENSE, badges, QuickStart)

**Avoids PITFALLS.md:**
- P1 (Drift): must validate each example against actual `--help` and `init` output
- P2 (Link Rot): fix links to existing files only; delete phantom references
- P9 (crates.io): use absolute raw.githubusercontent.com URLs for images

**Research flag:** Standard patterns -- README/CHANGELOG/LICENSE are well-documented conventions. No deeper research needed.

### Phase 2: GitHub Pages Landing Page + CI Deployment

**Rationale:** After README is the source of truth, the landing page extends it visually. Phase 2 depends on Phase 1 because the landing page references README content and links. The scope for v1.5 is a minimal single-page landing page, NOT a full multi-page mdBook site.

**Delivers:**
- Single-page landing page deployed to `guangl.github.io/sqllog2db/`
- Project hero section (name, description, install command, feature highlights)
- Architecture/Data flow diagram (Mermaid.js)
- Performance benchmark display (table + constant-memory curve)
- SVG chart gallery (4 real generated charts as thumbnails)
- `pages.yaml` workflow with path-filtered deployment
- mdBook validation job in `ci.yaml`
- `.gitignore` update for `site/book/`
- Cargo.toml `documentation` field update after deployment

**Uses STACK.md:** mdBook (minimal mode -- single page), GitHub Actions, peaceiris/actions-gh-pages@v4

**Implements ARCHITECTURE.md:** `site/` directory, `pages.yaml` workflow, CI validation job

**Avoids PITFALLS.md:**
- P3 (Stale Config): landing page shows minimal config, not full reference -- points to README
- P4 (Over-engineering): mdBook single binary, zero npm install; defer multi-page site
- P5 (Three-body): landing page does NOT duplicate README content; enhances and redirects
- P6 (Cargo.toml metadata): update immediately after deploy
- P7 (CI traps): use verified action template; test on PR artifact first

**Research flag:** Needs `/gsd:plan-phase --research-phase 2` during roadmap creation to resolve the mdBook scope tension. Specifically: (a) decide whether v1.5 landing page uses full mdBook with subset of pages vs. a standalone HTML file, (b) verify pages.yaml triggers and permissions against actual GitHub Actions behavior, (c) confirm symlink strategy works in Actions checkout environment.

### Phase 3: docs/ Directory + CI Quality Gates

**Rationale:** After README (Phase 1) and landing page (Phase 2) are live, fill in the deeper reference content and add automated quality checks. This phase institutionalizes documentation maintenance.

**Delivers:**
- `docs/quickstart.md` (more detailed than README)
- `docs/configuration.md` (full config reference with annotated examples)
- `docs/faq.md` (preemptive FAQ based on anticipated questions)
- `lychee` link checker in CI (initial setup as `continue-on-error`, graduate to blocking later)
- Asciicast demo recording (embedded in README or landing page)
- Documentation maintenance checklist added to project conventions

**Addresses FEATURES.md:** P3 items (config reference, asciicast) + P2 item (docs/quickstart)

**Avoids PITFALLS.md:**
- P8 (No maintenance workflow): CI link checks + PR checklist for doc updates
- P10 (Missing API docs): add docs.rs link to landing page footer

**Research flag:** Minimal research needed. Standard patterns for docs/ directory content. The asciicast recording needs practical testing (recording flow, embed code generation).

### Phase Ordering Rationale

1. **Phase 1 before Phase 2** -- README must be the established source of truth before the landing page references it. If README has stale content, the landing page inherits stale redirects. Also, quick feedback cycle: README is a single file edited in-place with zero build steps.
2. **Phase 2 before Phase 3** -- Landing page and CI infrastructure must exist before adding deeper reference docs. The CI checks (lychee) are most valuable once the docs surface area is established.
3. **Phase 2 scope constraint is critical** -- The biggest risk is Phase 2 expanding into a full mdBook multi-page site. This must be resisted at the roadmap level. The full mdBook site is v1.6+ territory.
4. **Pitfall avoidance drives ordering** -- Phase 1 addresses the most critical pitfall (README drift-from-code). Phase 2 must avoid the over-engineering trap. Phase 3 prevents future drift from accumulating.

### Research Flags

**Phases needing deeper research (`/gsd:plan-phase --research-phase`):**
- **Phase 2:** The mdBook vs. simple-HTML decision needs resolution. If mdBook is used, the scope of which pages to include in v1.5 vs. defer needs precise definition. Also need to verify the pages.yaml `actions-mdbook` + `gh-pages` workflow actually works end-to-end (especially `force_orphan: true` with path-restricted triggers).
- **Phase 3 (optional):** asciinema recording + embedding approach needs practical validation (recording quality, player embed code, file size tradeoffs).

**Phases with standard patterns (skip research-phase):**
- **Phase 1:** README updates, CHANGELOG, LICENSE, badges -- all well-documented conventions. No research needed. Use the FEATURES.md competitive analysis as style reference.
- **Phase 3 (docs/ content):** Standard Markdown documentation. Follow patterns from fd/bat/hyperfine READMEs.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All sources verified against official documentation (mdBook, peaceiris/actions-gh-pages, lychee, asciinema) or actual on-device installation (asciinema). Stack choice is clear and well-supported. |
| Features | HIGH | Direct analysis of 10+ Rust CLI tools (ripgrep, fd, bat, ruff, hyperfine, zoxide, bottom, eza, xh, gping). Feature priority derived from industry patterns and current sqllog2db README gaps. |
| Architecture | HIGH | v1.5 architecture based on direct codebase inspection (Cargo.toml, .github/workflows, README, .gitignore) and verified against mdBook/peaceiris official docs. The `site/` vs `docs/` separation is well-reasoned. |
| Pitfalls | HIGH | Based on actual README/source inspection of THIS project (not generic advice). All 10 pitfalls have concrete examples from sqllog2db's current state (broken links, stale config, missing Cargo.toml fields). |

**Overall confidence:** HIGH for individual research areas; MEDIUM for the integration of these findings into a roadmap due to the unresolved mdBook scope tension between STACK.md/ARCHITECTURE.md and FEATURES.md/PITFALLS.md.

### Gaps to Address

1. **mdBook scope for v1.5** -- STACK.md and ARCHITECTURE.md recommend mdBook with full site structure. FEATURES.md and PITFALLS.md recommend deferring full mdBook to v2+. **Resolution needed during Phase 2 planning:** Decide exact page count for v1.5 landing page (1 page? 3 pages?). If mdBook is kept as builder, what is the minimum viable SUMMARY.md? If rejected, what replaces it (plain HTML? Zola?).

2. **Language choice** -- ARCHITECTURE.md assumes Chinese-first for site navigation ("language = zh"). PITFALLS.md notes that existing init templates are bilingual (Chinese + English comments), which is a good pattern. **Decision needed:** Should the landing page be Chinese-only, English-only, or bilingual? The target audience is Chinese DM DBAs, but the GitHub ecosystem is English. This affects SUMMARY.md headings and site hierarchy.

3. **Symlink vs. copy for shared content** -- ARCHITECTURE.md recommends symlinks from `site/src/` to root CHANGELOG.md. PITFALLS.md notes symlinks may not work in all CI runners (some Windows actions). **Verify during Phase 2 implementation:** Do symlinks work in the GitHub Actions ubuntu-latest runner with actions/checkout@v6? Fallback: add a copy-script step in CI.

4. **Benchmark data freshness** -- PITFALLS.md notes that performance tables in docs are manually maintained. Current benchmark data (5.2M/s CSV, 1.55M/s real file) is from v1.2 era. **Action:** Re-run benchmarks during Phase 1 to get current v1.4 data for the README and landing page.

5. **Asciinema recording quality** -- asciicast format is recommended, but actual recording quality (color fidelity, terminal size, timing) needs practical validation. **Practical check during Phase 3.**

## Sources

### Primary (HIGH confidence)
- Direct source inspection: `src/features/filters.rs`, `src/config.rs`, `src/cli/run.rs`, `src/exporter/*.rs` (architecture verification)
- Direct source inspection: `README.md`, `Cargo.toml`, `.github/workflows/ci.yaml`, `.github/workflows/release.yaml` (current state assessment)
- mdBook User Guide: `rust-lang.github.io/mdBook/` (mdBook project structure, configuration, themes)
- peaceiris/actions-gh-pages@v4: GitHub Marketplace (deployment action capabilities)
- GitHub Pages documentation: docs.github.com/en/pages (deployment configuration)
- Rust CLI README analysis: ripgrep, fd, bat, ruff, hyperfine, zoxide, bottom (feature expectations)

### Secondary (MEDIUM confidence)
- asciinema v3.2.0: confirmed on-device at /opt/homebrew/bin/asciinema (recording tool availability)
- charts-rs v0.4.2: confirmed via `cargo search`
- hdrhistogram v7.5.4: confirmed via `cargo info`
- markdownlint-cli2 0.22.1: Homebrew analytics (868 installs/30d -- ecosystem adoption indicator)

### Tertiary (LOW confidence)
- rumdl 0.1.94: Rust-native md linter alternative -- not evaluated in depth; worth revisiting if Node.js dependency becomes a concern
- VHS terminal GIF generation: not tested on this machine; asciinema is already installed and preferred for docs

---
*Research completed: 2026-05-18*
*Ready for roadmap: yes (with Phase 2 scope decision needed during planning)*
