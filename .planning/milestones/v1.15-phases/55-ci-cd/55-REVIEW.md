---
phase: 55-ci-cd
reviewed: 2026-06-02T00:00:00Z
depth: standard
files_reviewed: 6
files_reviewed_list:
  - .github/workflows/ci.yaml
  - .github/workflows/bench.yml
  - .github/workflows/lychee.yml
  - .github/workflows/pages.yml
  - .github/workflows/release.yaml
  - Cross.toml
findings:
  critical: 2
  warning: 4
  info: 3
  total: 9
status: issues_found
---

# Phase 55: Code Review Report

**Reviewed:** 2026-06-02T00:00:00Z
**Depth:** standard
**Files Reviewed:** 6
**Status:** issues_found

## Summary

Reviewed five GitHub Actions workflows and one Cross.toml cross-compilation config. The workflows cover CI (test/lint/coverage), benchmarking, link checking, GitHub Pages deployment, and release builds. Overall structure is reasonable, but two correctness blockers were found: a Windows shell mismatch that silently fails, and an unpinned mutable `edge` container image that makes cross-compilation non-reproducible and supply-chain-unsafe. Four warnings cover missing `permissions:` declarations (over-broad token grants), a broken benchmark cache strategy, a redundant GITHUB_SHA env var, and unused `CARGO_TERM_COLOR` env in non-Rust workflows. Three info items cover the lychee cache key pattern, the redundant export PATH line, and dead env configuration.

---

## Critical Issues

### CR-01: `Add cargo to PATH` step runs in PowerShell on Windows without `shell: bash`

**File:** `.github/workflows/ci.yaml:29-30`
**Issue:** The step has no `shell:` key, so on `windows-latest` it executes in the default shell (PowerShell). The command `echo "$HOME/.cargo/bin" >> $GITHUB_PATH` is bash syntax. In PowerShell, `$HOME` is a string literal (`$HOME`) not the home directory, `>>` exists but appends UTF-16 with BOM to `$GITHUB_PATH`, and the resulting path is wrong. This means the PATH is not set correctly on Windows, and if `dtolnay/rust-toolchain` ever stops setting PATH itself, `cargo` commands will silently fail or use a stale binary. The step is redundant in practice (dtolnay/rust-toolchain already adds cargo to PATH), but its broken syntax makes it a lurking failure.

**Fix:**
```yaml
- name: Add cargo to PATH
  if: runner.os != 'Windows'
  shell: bash
  run: echo "$HOME/.cargo/bin" >> "$GITHUB_PATH"
```
Or simply remove the step entirely since `dtolnay/rust-toolchain@stable` already manages PATH on all three platforms.

---

### CR-02: `Cross.toml` pins the `edge` (mutable) image tag — non-reproducible and supply-chain unsafe

**File:** `Cross.toml:5`
**Issue:** The cross-rs container image is referenced as:
```toml
image = "ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge"
```
`edge` is a floating tag that resolves to the latest `main`-branch build of cross-rs. It can change between two identical invocations, meaning:
1. Builds are non-reproducible — the aarch64 release binary can differ between two tag pushes even with identical source.
2. Supply-chain risk — a compromised or broken `edge` image lands in the release pipeline without any review gate.
3. The comment in the file (`latest = 0.2.5，3 年前，过旧`) acknowledges that `latest` is stale but uses `edge` as the solution. The correct fix is to pin to a specific SHA digest or a versioned tag (e.g., `0.2.5` for a known stable, or a specific `-YYYYMMDD` snapshot tag from the GHCR registry).

**Fix:**
```toml
[target.aarch64-unknown-linux-gnu]
# Pin to a specific immutable digest instead of the floating `edge` tag.
# Run: docker manifest inspect ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge --verbose | grep digest
image = "ghcr.io/cross-rs/aarch64-unknown-linux-gnu@sha256:<specific-digest>"
```
Or use the most recent versioned release tag from https://github.com/cross-rs/cross/pkgs/container/aarch64-unknown-linux-gnu.

---

## Warnings

### WR-01: No `permissions:` declaration on `ci.yaml` and `bench.yml` — defaults to repo-wide token grant

**File:** `.github/workflows/ci.yaml:1`, `.github/workflows/bench.yml:1`
**Issue:** Neither workflow declares top-level or job-level `permissions:`. GitHub Actions defaults to the repository's default token permission setting, which is often `write-all`. For workflows that trigger on `pull_request` from forks, GitHub scopes the token to read-only automatically — but for `push` to `main` the token retains whatever the repository default is. If the org/repo has `write-all` as default (common), CI and bench workflows receive an unnecessarily powerful token. The principle of least privilege requires explicit `permissions:`.

**Fix:** Add a top-level permissions block to both files:
```yaml
permissions:
  contents: read
```
For `bench.yml`, `actions/upload-artifact` does not need any additional permissions beyond `contents: read`. For `ci.yaml`, `cargo-llvm-cov` and test jobs also only need `contents: read`.

---

### WR-02: Benchmark cache strategy in `bench.yml` never produces a cache hit — cache is always stale

**File:** `.github/workflows/bench.yml:39`
**Issue:** The benchmark artifact is uploaded with `name: bench-results-${{ github.sha }}`, meaning every commit creates a new artifact name. There is no mechanism to compare the current run against a previous baseline — the artifact is uploaded but never downloaded and compared. As a result, performance regressions are never detected; the job only collects data without acting on it. Coupled with `continue-on-error: true`, benchmark failures (including real regressions) are silently swallowed.

This is distinct from the lychee cache — for benchmarks, the expected pattern is to download the previous artifact, compute delta, and fail/warn if regression exceeds a threshold.

**Fix:** Either add a comparison step using a tool like `bencher` or `criterion-compare-action`, or document explicitly in the workflow that this is data-collection-only (no regression gate). Leaving it as-is creates false confidence that benchmarks are being enforced.

---

### WR-03: `release.yaml` matrix `release` job has no `permissions:` block — reads from a write-capable token during builds

**File:** `.github/workflows/release.yaml:11`
**Issue:** The `release` matrix job (which does the actual cross-platform builds and uploads artifacts) has no `permissions:` declaration. The `create-release` job correctly declares `permissions: contents: write`, but the build jobs inherit whatever the repo default is. Build jobs only need `contents: read` and nothing else; having implicit write access during a build step that handles external tool invocations (`cross`, cargo) is over-broad.

**Fix:**
```yaml
  release:
    name: Release ${{ matrix.artifact }}
    runs-on: ${{ matrix.os }}
    permissions:
      contents: read
    strategy:
      ...
```

---

### WR-04: `GITHUB_SHA` env var set redundantly in `bench.yml` — already a default environment variable

**File:** `.github/workflows/bench.yml:33-34`
**Issue:** The `Collect benchmark results` step sets:
```yaml
env:
  GITHUB_SHA: ${{ github.sha }}
```
`GITHUB_SHA` is already a [default environment variable](https://docs.github.com/en/actions/writing-workflows/choosing-what-your-workflow-does/store-information-in-variables#default-environment-variables) automatically set by GitHub Actions for every step. Re-declaring it is harmless but signals a misunderstanding of the runtime environment and creates noise. The `collect_bench_results.sh` script already accounts for the case where it's absent (`SHA="${GITHUB_SHA:-$(git rev-parse HEAD)}"`) making this doubly redundant.

**Fix:** Remove the redundant `env:` block from the `Collect benchmark results` step.

---

## Info

### IN-01: `CARGO_TERM_COLOR: always` declared in `lychee.yml` and `pages.yml` — env var has no effect

**File:** `.github/workflows/lychee.yml:20`, `.github/workflows/pages.yml:13`
**Issue:** Both workflows declare `CARGO_TERM_COLOR: always` at the top-level `env:` block, but neither workflow invokes `cargo`. `lychee.yml` only runs `lychee-action`; `pages.yml` only runs `mdbook`. The env var is completely inert and creates confusion about whether cargo is involved.

**Fix:** Remove `CARGO_TERM_COLOR: always` from both workflow files.

---

### IN-02: Redundant `export PATH` inside the test step after the PATH step already appended it

**File:** `.github/workflows/ci.yaml:35`
**Issue:** The `Run tests` step (line 33-36) contains:
```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --verbose
```
The preceding `Add cargo to PATH` step is supposed to persist the PATH change via `$GITHUB_PATH`. If that step worked correctly, the explicit `export PATH=...` inside the test step is redundant. If the `$GITHUB_PATH` step is broken (see CR-01), then this `export PATH` is a fragile workaround that only works for the single step in which it's set. Mixing both strategies is inconsistent.

**Fix:** Remove the `export PATH` line from the test step and fix CR-01 properly (or remove the `Add cargo to PATH` step entirely and rely on `dtolnay/rust-toolchain`'s PATH setup).

---

### IN-03: Lychee cache key includes `github.sha` — primary cache key never hits; only restore-key ever matches

**File:** `.github/workflows/lychee.yml:32-34`
**Issue:**
```yaml
key: lychee-${{ github.sha }}
restore-keys: lychee-
```
The primary `key` is commit-specific, so it never hits on the next run (different SHA). The cache is always restored via the fallback `restore-keys: lychee-` prefix. This is a common rolling-cache pattern and functionally works, but it means the `key:` field adds no value over just using `restore-keys`. The pattern is legitimate but non-obvious.

**Fix:** Either keep as-is (the pattern is functional) or use a stable key with periodic invalidation:
```yaml
key: lychee-${{ hashFiles('README.md', 'CHANGELOG.md') }}
restore-keys: lychee-
```
This at least makes the primary key hit when documents haven't changed.

---

_Reviewed: 2026-06-02T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
