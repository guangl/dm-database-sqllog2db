# Stack Research

**Domain:** CI/CD and engineering quality for a Rust CLI project
**Researched:** 2026-06-02
**Confidence:** HIGH (all actions versions verified via GitHub releases pages)

## Recommended Stack

### Core GitHub Actions (CI Workflow)

| Action | Version | Purpose | Why Recommended |
|--------|---------|---------|-----------------|
| `actions/checkout` | `v6` | Repo checkout | Current stable (v6.0.1, Nov 2025). Requires Actions runner ≥ v2.327.1 (all GitHub-hosted runners qualify). |
| `dtolnay/rust-toolchain` | `stable` (ref-based, no version pin) | Install Rust stable + components | De-facto standard for Rust CI. Supports `components: clippy, rustfmt`. Use `@stable` tag, not `@v1` — dtolnay's action uses rev-based selection. |
| `Swatinem/rust-cache` | `v2` | Cache `~/.cargo` and `target/` | Cuts cold-cache build time by 60–80%. v2.9.1 is current (Apr 2026). Smart key invalidation on `Cargo.lock` / `rust-toolchain` changes. |
| `taiki-e/install-action` | `v2` | Install `cargo-llvm-cov`, `cross` | Zero-friction binary installation from GitHub Releases. Supports tool-name shorthand (`@cargo-llvm-cov`). Current: v2.x (actively maintained). |
| `actions/upload-artifact` | `v4` | Upload bench results, coverage | v4 is minimum required for GitHub Pages artifacts; v3 deprecated Jan 2025. v7 exists but requires workflow changes — v4 is safe, well-supported. |

### Core GitHub Actions (CD / Release Workflow)

| Action | Version | Purpose | Why Recommended |
|--------|---------|---------|-----------------|
| `actions/checkout` | `v6` | Repo checkout on tag push | Same as CI. |
| `dtolnay/rust-toolchain` | `stable` | Install Rust + target | Use `targets: ${{ matrix.target }}` to add cross-compile target. |
| `Swatinem/rust-cache` | `v2` | Cache deps per target | Include `key: ${{ matrix.target }}` to avoid cache key collisions between targets. |
| `taiki-e/install-action` | `v2` | Install `cross` for aarch64 | `tool: cross` installs cross-rs for Docker-based cross-compilation. |
| `softprops/action-gh-release` | `v2` | Upload binaries to GitHub Release | v3 (Node 24) released Apr 2025 but v2 remains recommended — it is the last stable Node 20 line (v2.6.2) and the most widely tested. Current workflow uses v3; downgrade to v2 is safe and more conservative. |

### Cross-Compilation Toolchain

| Target | Runner | Tool | Notes |
|--------|--------|------|-------|
| `x86_64-unknown-linux-gnu` | `ubuntu-latest` | native `cargo build` | No cross-compile needed. rusqlite `bundled` feature compiles SQLite from source via `cc` crate — works natively. |
| `aarch64-unknown-linux-gnu` | `ubuntu-latest` | `cross` (Docker-based) | cross-rs provides pre-built Docker images with correct sysroot. Required because ubuntu-latest is x86_64. |
| `x86_64-pc-windows-msvc` | `windows-latest` | native `cargo build` | MSVC runner has Visual C++ build tools. rusqlite `bundled` uses `cc` crate which calls MSVC's `cl.exe` — works without extra setup. |
| `aarch64-apple-darwin` | `macos-latest` | native `cargo build` | GitHub's `macos-latest` runner is now Apple Silicon (M1/M2). Add `x86_64-apple-darwin` as a second macOS target if Intel support is needed. |

### Code Coverage

| Tool | Version | Purpose | Why |
|------|---------|---------|-----|
| `cargo-llvm-cov` | latest (via install-action) | LLVM-based line coverage | The only production-ready Rust coverage tool for CI. Installed via `taiki-e/install-action@cargo-llvm-cov`. Requires `llvm-tools-preview` component. `--fail-under-lines 70` enforces the gate. |

### Benchmark CI

| Tool | Version | Purpose | Why |
|------|---------|---------|-----|
| `benchmark-action/github-action-benchmark` | `v1` (v1.22.1) | Store/compare criterion results over time | Reads criterion's `--output-format bencher` JSON, stores history in gh-pages branch, posts PR comments on regression. Free, no external service needed. Recommended over bencher.dev for self-hosted projects without SaaS budget. |

## Existing Workflow Issues to Fix

The current `.github/workflows/` files have several version problems that need correction in v1.15:

| File | Issue | Fix |
|------|-------|-----|
| `ci.yaml` | `actions/upload-artifact@v7` — v7 requires ESM/Node24 changes | Downgrade to `@v4` which is stable and widely supported |
| `ci.yaml` | `actions/checkout@v6` — actually correct and current | Keep |
| `release.yaml` | `softprops/action-gh-release@v3` — Node 24 runtime, may not be needed yet | Acceptable but `@v2` (v2.6.2) is safer for compatibility |
| `bench.yml` | `actions/upload-artifact@v7` — same v7 issue | Downgrade to `@v4` |
| `bench.yml` | Calls `scripts/collect_bench_results.sh` which may not exist | Verify script exists or replace with github-action-benchmark |

## Installation / Setup

```bash
# No new Cargo.toml dependencies needed for CI/CD infrastructure
# All tooling is GitHub Actions actions or external CLI tools installed in CI

# To test CI locally before pushing:
cargo test --verbose
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
cargo doc --no-deps
cargo bench --no-run   # just compile benchmarks

# Coverage locally (requires cargo-llvm-cov):
cargo install cargo-llvm-cov
cargo llvm-cov --fail-under-lines 70
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `dtolnay/rust-toolchain` | `actions-rust-lang/setup-rust-toolchain` | `setup-rust-toolchain` adds problem matchers and Rust-specific annotations. Use if you want PR annotations pointing directly to clippy warnings. Slightly heavier. |
| `Swatinem/rust-cache` | Manual `actions/cache` with `~/.cargo` key | Manual cache gives more control over key strategy. Use if rust-cache's automatic key logic causes stale-cache bugs (rare). |
| `softprops/action-gh-release` (v2) | `gh release create` (GitHub CLI) | `gh` CLI is already available on all GitHub-hosted runners. Use for simpler workflows where body generation from markdown is not needed. Less config, more scripting. |
| `cross` for aarch64-linux | `cargo zigbuild` | `cargo zigbuild` uses Zig's C compiler as cross-linker, no Docker needed. Lighter but less battle-tested with rusqlite `bundled` C compilation. Prefer `cross` for this project. |
| `benchmark-action/github-action-benchmark` | `bencher.dev` (SaaS) | bencher.dev has better statistical regression detection (change-point algorithm) and a dashboard. Use if free tier is sufficient and you want zero-maintenance setup. Requires API key secret. |
| Native macOS Intel target | `x86_64-apple-darwin` added to matrix | Add only if users report Intel Mac issues. Current `macos-latest` = Apple Silicon covers the majority. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `actions/upload-artifact@v3` | Deprecated January 2025; removed from GitHub Marketplace | `@v4` |
| `actions/upload-artifact@v7` | Requires ESM output and Node 24 runtime; introduced breaking interface changes | `@v4` (stable, widely tested) |
| `cargo-tarpaulin` | Older coverage tool, slower, less accurate than llvm-cov, known false negatives with integration tests | `cargo-llvm-cov` |
| `cargo-dist` | Generates opinionated CI infrastructure and `dist.toml` config. Valuable for crates.io-first projects with many targets. Overkill when you already have hand-crafted workflows and want full control. | Hand-crafted release.yaml (current approach) |
| `release-plz` | Automates `CHANGELOG.md` and version bumps via PR. Useful for libraries; adds overhead for a single-maintainer CLI tool where manual tagging is fine. | Manual `git tag v1.x.x && git push --tags` |
| `x86_64-unknown-linux-musl` target | rusqlite `bundled` requires C compilation; musl cross-compilation with SQLite has known segfault issues in cross-rs containers. | `x86_64-unknown-linux-gnu` (glibc, works reliably) |

## Cross-Compilation Matrix (Final Recommendation)

```yaml
matrix:
  include:
    # Linux x86_64 — native, glibc, most portable for Linux servers
    - os: ubuntu-latest
      target: x86_64-unknown-linux-gnu
      use_cross: false

    # Linux ARM64 — cross-compiled via Docker; covers servers, Raspberry Pi, AWS Graviton
    - os: ubuntu-latest
      target: aarch64-unknown-linux-gnu
      use_cross: true

    # Windows x86_64 — native MSVC; rusqlite bundled works with cl.exe
    - os: windows-latest
      target: x86_64-pc-windows-msvc
      use_cross: false

    # macOS ARM64 — native on M1/M2 runner; covers modern Macs
    - os: macos-latest
      target: aarch64-apple-darwin
      use_cross: false
```

This is exactly what the current `release.yaml` has — the matrix is correct. The main issues are action version pinning and the missing `${{ matrix.os }}` in artifact naming for Windows `.exe` files.

## Version Compatibility Notes

| Component | Rust Version Constraint | Notes |
|-----------|------------------------|-------|
| `cargo-llvm-cov` | Requires `llvm-tools-preview` component | Install via `dtolnay/rust-toolchain` with `components: llvm-tools-preview` |
| `cross` | Works with stable Rust | Requires Docker on the runner; ubuntu-latest has Docker pre-installed |
| `criterion 0.7` (current) | Rust 1.65+ | Already in Cargo.toml as dev-dependency |
| `rusqlite 0.39` bundled | C compiler on runner | Works on all 4 targets above; confirmed no musl issues with gnu targets |

## Sources

- [actions/checkout releases](https://github.com/actions/checkout/releases) — v6.0.1 confirmed current stable (Nov 2025)
- [dtolnay/rust-toolchain README](https://github.com/dtolnay/rust-toolchain) — ref-based versioning, `@stable` recommended
- [Swatinem/rust-cache releases](https://github.com/swatinem/rust-cache/releases) — v2.9.1 confirmed (Apr 2026)
- [taiki-e/install-action releases](https://github.com/taiki-e/install-action/releases) — v2.x active, cargo-llvm-cov support confirmed
- [softprops/action-gh-release releases](https://github.com/softprops/action-gh-release/releases) — v3.0.0 (Node 24), v2.6.2 (Node 20 last stable)
- [actions/upload-artifact releases](https://github.com/actions/upload-artifact/releases) — v4.x stable, v3 deprecated Jan 2025, v7 current
- [benchmark-action/github-action-benchmark releases](https://github.com/benchmark-action/github-action-benchmark/releases) — v1.22.1 (May 2026)
- [cross-rs/cross GitHub](https://github.com/cross-rs/cross) — Docker-based cross-compilation, aarch64-linux-gnu supported
- WebSearch: rusqlite bundled + MSVC/cross-compile CI — confirmed gnu targets work; musl has known issues (avoid)

---
*Stack research for: CI/CD and engineering quality — Rust CLI (sqllog2db v1.15)*
*Researched: 2026-06-02*
