---
phase: 55-ci-cd
fixed_at: 2026-06-02T00:00:00Z
review_path: .planning/phases/55-ci-cd/55-REVIEW.md
iteration: 1
findings_in_scope: 9
fixed: 9
skipped: 0
status: all_fixed
---

# Phase 55: Code Review Fix Report

**Fixed at:** 2026-06-02T00:00:00Z
**Source review:** .planning/phases/55-ci-cd/55-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 9
- Fixed: 9
- Skipped: 0

## Fixed Issues

### CR-01 + IN-02: Remove broken "Add cargo to PATH" step and redundant export PATH

**Files modified:** `.github/workflows/ci.yaml`
**Commit:** 41d4a29
**Applied fix:** Removed the entire "Add cargo to PATH" step (which ran bash syntax
in PowerShell on Windows without a `shell:` key). Also removed the redundant
`export PATH="$HOME/.cargo/bin:$PATH"` line from the "Run tests" step. The
`dtolnay/rust-toolchain@stable` action already manages PATH on all three platforms.
The "Run tests" step now simply runs `cargo test --verbose` directly.

---

### CR-02: Document supply-chain risk of floating `edge` tag in Cross.toml

**Files modified:** `Cross.toml`
**Commit:** 41d4a29
**Applied fix:** Added a prominent multi-line TODO comment documenting the exact
risks of the `edge` floating tag (non-reproducible builds, supply-chain risk),
and instructions for how to pin to an immutable SHA digest including the
`docker manifest inspect` command to retrieve the current digest. The image tag
itself remains `edge` as a documented temporary measure until the digest can be
pinned.

---

### WR-01: Add `permissions: contents: read` to ci.yaml and bench.yml

**Files modified:** `.github/workflows/ci.yaml`, `.github/workflows/bench.yml`
**Commit:** 41d4a29
**Applied fix:** Added a top-level `permissions: contents: read` block to both
files, immediately after the `on:` trigger block. This applies the principle of
least privilege and ensures neither workflow receives write access by default.

---

### WR-02: Document data-collection-only benchmark job in bench.yml

**Files modified:** `.github/workflows/bench.yml`
**Commit:** 41d4a29
**Applied fix:** Added a comment block on the `benchmark` job explaining that it
is data-collection-only, that no regression gate is enforced, and pointing toward
`bencher` or `criterion-compare-action` as options for adding regression detection
in the future.

---

### WR-03: Add `permissions: contents: read` to release.yaml matrix build job

**Files modified:** `.github/workflows/release.yaml`
**Commit:** 41d4a29
**Applied fix:** Added `permissions: contents: read` to the `release` matrix job
(the build jobs), between `runs-on` and `strategy`. The `create-release` job
already had `permissions: contents: write` and was left unchanged.

---

### WR-04: Remove redundant GITHUB_SHA env from bench.yml

**Files modified:** `.github/workflows/bench.yml`
**Commit:** 41d4a29
**Applied fix:** Removed the entire `env:` block from the "Collect benchmark
results" step. `GITHUB_SHA` is already a default GitHub Actions environment
variable and the script already falls back to `git rev-parse HEAD` when absent.

---

### IN-01: Remove CARGO_TERM_COLOR from lychee.yml and pages.yml

**Files modified:** `.github/workflows/lychee.yml`, `.github/workflows/pages.yml`
**Commit:** 41d4a29
**Applied fix:** Removed the top-level `env:` block (containing only
`CARGO_TERM_COLOR: always`) from both files. Neither workflow invokes cargo —
lychee.yml runs lychee-action and pages.yml runs mdbook.

---

### IN-03: Update lychee cache key to use hashFiles for stable primary hit

**Files modified:** `.github/workflows/lychee.yml`
**Commit:** 41d4a29
**Applied fix:** Changed the cache `key:` from `lychee-${{ github.sha }}` (commit-
specific, never hits on subsequent runs) to
`lychee-${{ hashFiles('README.md', 'CHANGELOG.md') }}`. The primary key now hits
whenever the checked documents haven't changed between commits, reducing redundant
cache misses. The `restore-keys: lychee-` fallback was kept unchanged.

---

_Fixed: 2026-06-02T00:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
