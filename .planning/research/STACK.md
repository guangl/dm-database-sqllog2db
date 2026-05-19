# Stack Research

**Domain:** Rust CLI documentation tooling and GitHub Pages
**Researched:** 2026-05-18
**Confidence:** HIGH

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| **mdBook** | 0.5.2+ | Static site generator for project documentation | Written in Rust (no Node.js dependency for this Rust project), Rust project own docs use it (The Book, Rust Reference), native Markdown with search, syntax highlighting, theming, and integrated toolchain (`mdbook serve` with hot-reload). Outputs plain HTML to `book/` -- trivial to deploy to GitHub Pages. |
| **GitHub Actions** | N/A | CI/CD deployment | Already has CI and release workflows -- adding a docs deploy workflow reuses existing patterns. Native GitHub Pages deployment via `peaceiris/actions-gh-pages@v4` or the built-in `actions/deploy-pages`. |
| **GitHub Pages** | N/A | Static site hosting | Free, automatic per-repo hosting. Zero operational overhead. Custom domain support. Integrates directly with Actions. |
| **lychee** | 0.24.2 | Link checker for Markdown docs | Written in Rust, fast async link checking, supports Markdown/HTML/reST, GitHub Action available (`lycheeverse/lychee-action`), `.lycheeignore` for known-broken links. Native Rust tool that fits project's language ecosystem. |

### Supporting Libraries

| Technology | Version | Purpose | When to Use |
|------------|---------|---------|-------------|
| **asciinema** | 3.2.0 | Terminal session recording for CLI demos | Already installed on dev machine. Lightweight asciicast format (~8% of video file size). Embeddable web player for GitHub Pages. Best for recording live CLI sessions with real output. |
| **markdownlint-cli2** | 0.22.1 | Markdown style checking | Node.js dependency but best-in-class rule set (MD001-MD060). GitHub Action (`DavidAnson/markdownlint-cli2-action`). Config-driven, per-directory overrides. The `--fix` flag auto-corrects common issues. |
| **cargo doc** | Rust toolchain bundled | API-level Rust documentation | Already in CI (`cargo doc --no-deps` with `RUSTDOCFLAGS: -D warnings`). Generates `target/doc/` -- can be deployed as a subdirectory under GitHub Pages for reference docs alongside the mdBook guide. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `cargo doc --no-deps` | Generate Rust API docs | Already configured in CI lint job. Enables doc lint checking. |
| `lychee` | Check Markdown link health | Run in CI on every PR to catch broken links before merge. |
| `markdownlint-cli2` | Enforce Markdown consistency | Configure `.markdownlint-cli2.yaml` with project-specific rule overrides. |
| `asciinema` | Record terminal demo sessions | Record `.cast` files, optionally convert to GIF for README, or embed web player in GitHub Pages. |
| `git-cliff` | Generate CHANGELOG from conventional commits | Optional: auto-generates CHANGELOG.md from git history. Native Rust. Supports custom templates. |

## Installation

```bash
# Core -- mdBook via Cargo
cargo install mdbook

# Documentation CI tools
cargo install lychee

# Markdown linting (requires Node.js)
brew install markdownlint-cli2

# Terminal recording (already installed)
# asciinema is available at /opt/homebrew/bin/asciinema 3.2.0

# Optional: CHANGELOG generation
cargo install git-cliff

# Optional: terminal GIFs (requires ffmpeg + ttyd)
brew install vhs
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| **mdBook** | **VitePress** | When project also uses Vue.js and wants interactive SPA components. Adds Node.js dependency to a Rust project -- unnecessary overhead for documentation-only sites. |
| **mdBook** | **Hugo** | For larger sites with blog/landing page hybrid needs (e.g., corporate product sites). Overkill for a focused documentation site for one CLI tool. Go template complexity is not worth it. |
| **mdBook** | **Jekyll** | When using GitHub Pages default and want zero-build-step deployment. But imposes Ruby dependency, limited theming, slower builds, no built-in search without plugins. |
| **mdBook** | **plain HTML** | When avoiding all build tools. But loses search, navigation, syntax highlighting, hot-reload, and maintainability. Not worth the trade-off for a documentation site. |
| **mdBook** | **Docusaurus** | Meta's documentation framework. Requires React and heavy Node.js toolchain. Excellent for large OSS projects with multiple languages, but over-engineered for a single CLI tool. |
| **lychee** | **markdown-link-check** | markdown-link-check (Node.js) is slower and has no Rust alternative for a Rust project. lychee is async, faster, and fits the Rust ecosystem. |
| **lychee** | **htmltest** | Go-based HTML checker. Fine alternative, but not a Rust tool. lychee is simpler, purpose-built for Markdown. |
| **markdownlint-cli2** | **markdownlint-cli** | markdownlint-cli2 is actively maintained by the same author, supports per-directory config, multiple output formatters (JUnit, SARIF, JSON), and has a GitHub Action. markdownlint-cli is older. |
| **markdownlint-cli2** | **rumdl** | Rust-native Markdown linter (rumdl 0.1.94 on crates.io). Would avoid Node.js dependency. However, rumdl has fewer rules, smaller community, and less CI tooling than markdownlint-cli2. Worth re-evaluating if rumdl matures. |
| **asciinema** | **VHS** | VHS produces animated GIFs directly and supports scripting via `.tape` files. However, asciinema is already installed, produces smaller files (text protocol vs image), and the embeddable web player looks better on docs pages. VHS is better for README.md GIF badges and social sharing. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| **Sphinx (Read the Docs)** | Python dependency, heavy configuration, mdBook is simpler and Rust-native. | mdBook |
| **docsify** | Runtime-rendered (not static), poor SEO, requires JavaScript to view content. | mdBook (static HTML) |
| **pandoc+manually generated docs** | No local dev server, no hot-reload, no search, no navigation structure. | mdBook |
| **screenshot-based demo images** | Static, unmaintainable, need re-screenshotting after every CLI change. | asciinema (text-based, always current, selectable) |
| **VHS for all demos** | GIF files are larger (pixel data) and cannot be searched or selected. VHS requires ffmpeg + ttyd dependencies. | asciinema for docs pages, VHS for README only if visual GIF demo is needed |
| **Jekyll default GitHub Pages** | GitHub Pages' built-in Jekyll is limited to a whitelist of plugins, slow builds, Ruby dependency for local dev. | mdBook with peaceiris/actions-gh-pages |

## Stack Patterns by Variant

**If you want the simplest possible setup (no Node.js):**
- Use mdBook + lychee (both Rust-native)
- Skip markdownlint-cli2 -- use markdownlint-rs (0.3.15) or rumdl (0.1.94) as Rust-native alternatives
- Record asciinema demos only (no GIF conversion needed)
- Deploy with `peaceiris/actions-gh-pages@v4`
- This is the **recommended approach** for this project -- minimal new dependencies

**If you want a polished landing page with visual demos:**
- Use mdBook for documentation
- Add a separate landing page (`index.html`) served from GitHub Pages root, linking to mdBook's output at `/docs/`
- Use VHS for animated GIF demos on the landing page
- Record longer usage videos with asciinema embedded in the mdBook guide

**If you want to keep everything in pure Rust (zero new language runtimes):**
- mdBook (Rust) + lychee (Rust) + rumdl (Rust crate-based Markdown linting)
- asciinema (Rust) for demos
- This eliminates all Node.js/Python/Ruby dependencies

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| mdBook 0.5.2 | Rust 1.88+ | mdBook latest stable via `cargo install mdbook` |
| peaceiris/actions-gh-pages@v4 | Any runner (linux/macos/windows) | Pin to `@v4` major tag, or specific hash for maximum stability |
| lychee 0.24.2 | Any platform | Available via `cargo install lychee` or `brew install lychee` |
| markdownlint-cli2 0.22.1 | Node.js 18+ | Via npm (`npm install -g markdownlint-cli2`) or Homebrew |
| asciinema 3.2.0 | macOS, Linux, FreeBSD | Already installed locally. NOT available on Windows. |
| asciinema player | Any modern browser | Embed via `<script src="...">` -- lightweight JS widget |
| peaceiris/actions-gh-pages@v4 | GITHUB_TOKEN or deploy key | Requires `contents: write` permission in workflow |

## GitHub Pages Site Architecture

The recommended structure:

```
sqllog2db/
├── .github/workflows/
│   ├── ci.yaml                     # existing -- tests, lint, coverage
│   ├── release.yaml                # existing -- build, crates.io publish
│   └── docs.yaml                   # NEW -- build mdBook + deploy to GitHub Pages
├── book/                           # GENERATED -- mdBook output (in .gitignore)
│   └── (static HTML site)
├── src.md/                         # NEW -- mdBook source directory (not src/ which is Rust source)
│   ├── SUMMARY.md                  # NEW -- mdBook chapter structure
│   ├── intro.md                    # symlinked or copied from project README
│   ├── quickstart.md
│   ├── architecture.md
│   ├── faq.md
│   ├── demo.cast                   # asciinema recording
│   └── images/                     # screenshots and diagrams
├── book.toml                       # NEW -- mdBook configuration
├── .lycheeignore                    # NEW -- lychee ignore patterns
├── .markdownlint-cli2.yaml          # NEW -- markdownlint configuration
└── assets/                         # static files (images, diagrams) if needed
```

**NOTE:** The mdBook source directory should NOT be `src/` since that conflicts with Rust source. Use `src.md/` or `docs-md/` as the source directory (configured via `[book] src = "src.md"` in `book.toml`).

Deployment URL scheme:
```
https://guangl.github.io/sqllog2db/     -> mdBook documentation site
https://guangl.github.io/sqllog2db/api/ -> cargo doc output (optional, deferred)
```

## Sources

- mdBook official documentation (rust-lang.github.io/mdBook/) -- installation, configuration, SUMMARY.md structure [HIGH confidence]
- peaceiris/actions-gh-pages@v4 GitHub Marketplace -- deployment action capabilities [HIGH confidence]
- peaceiris/actions-mdbook@v2 -- mdBook setup in CI [HIGH confidence]
- asciinema v3.2.0 GitHub repo (github.com/asciinema/asciinema) -- terminal recording, asciicast format [HIGH confidence]
- VHS 0.11.0 Homebrew formula (github.com/charmbracelet/vhs) -- terminal GIF generation [MEDIUM confidence, verified via Homebrew sources]
- markdownlint-cli2 0.22.1 documentation (github.com/DavidAnson/markdownlint-cli2) -- linting rules, configuration, CI integration [HIGH confidence]
- lychee 0.24.2 documentation (github.com/lycheeverse/lychee) -- link checking, GitHub Action [HIGH confidence]
- crates.io search results -- mdbook 0.5.2, lychee 0.24.2, rumdl 0.1.94, markdownlint-rs 0.3.15 [HIGH confidence]
- Homebrew analytics -- markdownlint-cli2 868 installs/30d, VHS 786 installs/30d [MEDIUM confidence, indication of ecosystem adoption]
- Actual asciinema 3.2.0 installation confirmed on dev machine at /opt/homebrew/bin/asciinema [HIGH confidence]

---
*Stack research for: sqllog2db v1.5 documentation and GitHub Pages*
*Researched: 2026-05-18*
