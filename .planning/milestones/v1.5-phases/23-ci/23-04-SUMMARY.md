---
phase: 23-ci
plan: 04
subsystem: infra
tags: [ci, lychee, link-checker, github-actions]

requires:
  - phase: 21
    provides: "README.md, CHANGELOG.md"
  - phase: 22
    provides: "site/src/index.md"
  - phase: 23-01
    provides: "docs/quickstart.md"

provides:
  - ".github/workflows/lychee.yml — link checker CI workflow"
affects: []

tech-stack: {added: ["lycheeverse/lychee-action@v2"], patterns: []}
key-files:
  created: [".github/workflows/lychee.yml"]

requirements-completed: ["SUPP-05"]
duration: 3min
completed: 2026-05-19
---

# Phase 23 Plan 04: lychee Link Checker CI

**GitHub Actions workflow checking all markdown links on push/PR**

## Accomplishments

- Scans README.md, CHANGELOG.md, docs/*.md, site/**/*.md
- External links: max-retries 3, timeout 30s
- Internal links: strict failure blocks CI
- Rate-limited domains excluded (crates.io)
- Cache enabled for faster re-runs

## Deviations: None
