---
phase: 45-ci
plan: 02
subsystem: infra
tags: [github-actions, criterion, benchmark, ci, shell]

# Dependency graph
requires: []
provides:
  - GitHub Actions benchmark workflow (.github/workflows/bench.yml)
  - Criterion estimates.json 收集脚本 (scripts/collect_bench_results.sh)
affects: [future-phases-using-benchmark-artifacts]

# Tech tracking
tech-stack:
  added: [actions/upload-artifact@v4]
  patterns: [criterion-artifact-collection, informational-ci-job]

key-files:
  created:
    - .github/workflows/bench.yml
    - scripts/collect_bench_results.sh
  modified: []

key-decisions:
  - "同时触发 PR + push to main（与 ci.yaml 触发条件一致，PR 时对比，push 记录永久基线）"
  - "retention-days: 60（落在 RESEARCH.md 推荐范围 30-90 内，平衡存档需求与 artifact 配额）"
  - "continue-on-error: true 防止 bench 失败阻塞 PR merge（job 级别，非 step 级别）"
  - "单 runner ubuntu-latest 而非 matrix（CI bench 不需要跨 OS 对比，per CLAUDE.md 简单优先）"
  - "复用 ci.yaml 已验证 action 版本（checkout@v6, rust-toolchain@stable, rust-cache@v2）"

patterns-established:
  - "Pattern: 用 find + awk -F/ 提取路径段（NF-3/NF-2），避免硬编码 criterion 路径深度"
  - "Pattern: jq -s 'map({(.key): ...}) | add // {}' 将流式 JSON 合并为 object"
  - "Pattern: GITHUB_SHA 作为环境变量注入 shell 脚本（避免脚本内 git rev-parse 依赖 CI 环境）"

requirements-completed: [BENCH-02]

# Metrics
duration: 10min
completed: 2026-05-24
---

# Phase 45 Plan 02: Benchmark CI Workflow Summary

**GitHub Actions benchmark workflow + criterion estimates.json 收集脚本，PR 和 push to main 自动触发 cargo bench 并上传含 mean_ns/stddev_ns 的 JSON artifact（retention 60 天，continue-on-error 不阻塞 merge）**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-05-24T15:06:00Z
- **Completed:** 2026-05-24T15:16:17Z
- **Tasks:** 2
- **Files modified:** 2（均为新增）

## Accomplishments

- 新建 `scripts/collect_bench_results.sh`：递归 find target/criterion/**/new/estimates.json，用 jq + awk 合并为含 timestamp/commit_sha/mean_ns/stddev_ns 的单一 JSON 文件
- 新建 `.github/workflows/bench.yml`：PR + push to main 触发，timeout 30 分钟 + continue-on-error 防阻塞，artifact retention 60 天
- 零侵入：ci.yaml 未修改，scripts/ 为新增目录，与现有 CI 并行运行

## Task Commits

Each task was committed atomically:

1. **Task 1: 新建 scripts/collect_bench_results.sh** - `a36c8d6` (feat)
2. **Task 2: 新建 .github/workflows/bench.yml** - `cc0deb0` (feat)

## Files Created/Modified

- `scripts/collect_bench_results.sh` — criterion estimates.json 收集脚本，set -euo pipefail，输出含 timestamp/commit_sha/benchmarks 的 JSON 文件
- `.github/workflows/bench.yml` — Benchmark workflow，PR + push to main 触发，cargo bench + 收集 + upload-artifact@v4

## Decisions Made

- **触发条件**：同时监听 pull_request 和 push to main（与 ci.yaml 一致），PR 时可对比当前 vs 基线，push 后记录每次合并的永久基线
- **retention-days**：选 60 天，RESEARCH.md 推荐 30-90 内，平衡历史归档与 GitHub artifact 存储配额
- **continue-on-error**：设置在 job 级别（非 step 级别），确保 bench 失败时整个 workflow 仍显示 success，不阻塞 PR merge
- **单 runner**：ubuntu-latest 单 runner 而非 matrix，性能基准不需要跨 OS 对比（遵循 CLAUDE.md 简单优先）
- **action 版本**：复用 ci.yaml 已验证版本，新增 actions/upload-artifact@v4（GitHub 官方 action 当前稳定大版本）

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- python3 yaml 模块未安装（YAML 合法性验证），改用 `yq` CLI 验证（yq 已通过 homebrew 安装），所有字段值均正确

## User Setup Required

None - no external service configuration required.

## Reviewer Notes（首次 PR 后验证）

首次 PR 触发后请到 GitHub Actions → Benchmark workflow 确认：
1. job 运行完成（即使 continue-on-error，step 应全部绿色）
2. PR / commit 页面 Artifacts 区域出现 `bench-results-<sha>` 链接
3. 下载解压后包含 `bench-results-<short_sha>.json`
4. `jq '.benchmarks | keys'` 验证含 csv_export / sqlite_export / filters 等 key

此 CI 端验证在 PR 实际触发后人工确认，不阻塞本 plan 完成。

## Next Phase Readiness

- BENCH-02 达成：每个 PR 自动产出 benchmark artifact，供历史对比，无需开发者手动跑 bench
- 后续可在此基础上增加 benchmark 对比（如 benchmark-action 比较 PR vs base）

---
*Phase: 45-ci*
*Completed: 2026-05-24*
