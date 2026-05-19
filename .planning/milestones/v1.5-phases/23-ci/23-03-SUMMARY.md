---
phase: 23-ci
plan: 03
subsystem: docs
tags: [asciicast, demo, terminal-recording, asciinema]

requires: []
provides:
  - "site/asciicast/demo.cast — terminal recording of init→validate→run workflow"
  - "Landing page asciinema-player embed (site/src/index.md updated)"
affects: []

tech-stack: {added: ["asciinema"], patterns: []}
key-files:
  created: ["site/asciicast/demo.cast"]
  modified: ["site/src/index.md"]

requirements-completed: ["SUPP-04"]
duration: 5min
completed: 2026-05-19
---

# Phase 23 Plan 03: Asciicast Terminal Demo

**Recorded terminal demo: sqllog2db init → validate → run workflow**

## Accomplishments

- Recorded 3-step workflow: init -o config.toml → validate -c config.toml → run
- Embedded asciinema-player in landing page (site/src/index.md)
- Demo file saved at site/asciicast/demo.cast (1227 bytes)

## Deviations: None

## Note

The asciicast was recorded in headless mode (TTY not available). For a more polished recording, re-record interactively with `asciinema rec --overwrite site/asciicast/demo.cast`.
