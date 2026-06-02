---
phase: 55-ci-cd
plan: "02"
subsystem: ci-cd
tags:
  - ci-cd
  - release
  - cross-compile
  - github-actions
dependency_graph:
  requires:
    - ".github/workflows/release.yaml (55-01 修复的 action 版本)"
  provides:
    - ".github/workflows/release.yaml (完整重构：artifact 暂存 + 统一发布)"
    - "Cross.toml (aarch64-linux 跨编译配置)"
  affects:
    - "GitHub Actions release workflow"
    - "aarch64-unknown-linux-gnu 跨编译构建"
tech_stack:
  added:
    - "Cross.toml (cross-rs 跨编译配置)"
    - "ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge 镜像配置"
  patterns:
    - "artifact 暂存 + 独立 create-release job（消除并行竞争条件）"
    - "needs: [release] 依赖链（串行化 release 创建步骤）"
    - "permissions: contents: write 最小化（仅 create-release job）"
key_files:
  created:
    - "Cross.toml"
  modified:
    - ".github/workflows/release.yaml"
decisions:
  - "D-04 落实：upload-artifact@v4 暂存产物，create-release job 统一下载发布"
  - "D-05 落实：needs: [release] 确保 4 个 build job 全部完成后才创建 release"
  - "D-06 落实：awk changelog 提取逻辑从 build job 移入 create-release job"
  - "D-07 落实：publish job 删除，避免 CARGO_REGISTRY_TOKEN 缺失导致 release 失败"
  - "D-08 落实：Cross.toml 使用 edge tag（比 latest 的 0.2.5 更新，3 年前发布）"
  - "softprops/action-gh-release 版本选择 @v2（研究阶段已审核，Package Legitimacy Approved）"
metrics:
  duration: "~4 minutes"
  completed_date: "2026-06-02"
  tasks_completed: 4
  files_changed: 2
---

# Phase 55 Plan 02: Release Workflow 重构与 Cross.toml 摘要

**一句话：** artifact 暂存 + 独立 create-release job 完全消除了 4 个并行 build job 竞争写入 release notes 的竞争条件，Cross.toml 为 aarch64-linux 跨编译配置 cross-rs 官方 edge 镜像。

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | 新建 Cross.toml（aarch64-linux 跨编译镜像配置） | 6498dbd | Cross.toml |
| 2 | 重构 release matrix job（仅暂存 artifact） | 81e6a80 | .github/workflows/release.yaml |
| 3 | 添加独立 create-release job | 2c2c9ef | .github/workflows/release.yaml |
| 4 | 综合验证 + cargo 质量门禁 | （无代码修改，验证通过） | — |

## What Was Built

### Cross.toml（Task 1）

新建项目根目录的 `Cross.toml`，为 `aarch64-unknown-linux-gnu` target 配置官方 cross-rs Docker 镜像：

- 镜像：`ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge`
- `edge` tag = main 分支最新构建，优于 `latest`（0.2.5，3 年前）
- 无需 `pre-build` 段：`rusqlite = { features = ["bundled"] }` 与 edge 镜像兼容

### release.yaml 重构（Tasks 2 + 3）

**原架构问题：** 4 个 matrix build job 各自调用 `softprops/action-gh-release@v3` 写入 release body，并行时最后写入者覆盖前者内容（CICD-03 竞争条件）。同时 `publish` job 因 `CARGO_REGISTRY_TOKEN` 未配置导致 release 全局失败。

**新架构：**

```
Push tag (v*)
    ↓
4 个并行 build job（release matrix）
    - checkout@v4（D-01 修复）
    - 无 permissions: contents: write（最小权限 T-55-04）
    - 编译 → 产物暂存 upload-artifact@v4（唯一名称）
    ↓
create-release job（needs: [release]，串行等待）
    - permissions: contents: write（仅此处声明）
    - download-artifact@v4（pattern: sqllog2db-*，merge-multiple）
    - awk changelog 提取（D-06 保留逻辑）
    - softprops/action-gh-release@v2（统一创建 Release）
```

**关键变更：**
- `actions/checkout@v6` → `@v4`（D-01）
- release job 移除 `permissions: contents: write`（T-55-04 最小权限）
- 删除 `Extract changelog` 和 `Upload to GitHub Release` 步骤（移入 create-release）
- 新增 `Upload artifact` 步骤（upload-artifact@v4，唯一名称，retention-days: 1）
- 删除整个 `publish` job（D-07，T-55-09）
- 新增独立 `create-release` job（消除竞争条件，CICD-03）
- `softprops/action-gh-release@v3` → `@v2`（研究阶段确认 @v2 为当前稳定版）

## Deviations from Plan

无 — 计划执行完全按照 PLAN.md 中的任务描述进行，无需自动修复或架构调整。

## Verification Results（Task 4）

所有源断言通过：

- `test -f Cross.toml` → 退出码 0
- `grep -c '[target.aarch64-unknown-linux-gnu]' Cross.toml` → `1`
- `grep -c 'ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge' Cross.toml` → `1`
- `grep -c 'pre-build' Cross.toml` → `0`
- `grep 'actions/checkout@v6' .github/workflows/release.yaml` → 无匹配
- `grep 'actions/upload-artifact@v7' .github/workflows/release.yaml` → 无匹配
- `grep 'softprops/action-gh-release@v3' .github/workflows/release.yaml` → 无匹配
- `grep -E 'publish:|cargo publish|CARGO_REGISTRY_TOKEN' .github/workflows/release.yaml` → 无匹配
- `grep -v '^#' .github/workflows/release.yaml | grep -c 'contents: write'` → `1`（仅 create-release）
- `grep -c 'actions/checkout@v4' .github/workflows/release.yaml` → `2`
- YAML 语法验证 → valid
- `cargo clippy --all-targets -- -D warnings` → 通过（无 Rust 代码修改）
- `cargo fmt --all -- --check` → 通过

## Known Stubs

无。本 plan 修改的是 CI/CD workflow 配置文件，无 stub 数据或占位符。

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: T-55-04 已缓解 | .github/workflows/release.yaml | build job 的 contents:write 权限已移除，仅 create-release 持有 |
| threat_flag: T-55-05 已缓解 | .github/workflows/release.yaml | 单一 create-release job 写入 release body，消除并发写入竞争 |
| threat_flag: T-55-09 已缓解 | .github/workflows/release.yaml | publish job 已删除，CARGO_REGISTRY_TOKEN 缺失不再导致 release 失败 |

## Self-Check: PASSED

- Cross.toml: FOUND
- .github/workflows/release.yaml: FOUND（含 create-release job）
- commit 6498dbd: FOUND
- commit 81e6a80: FOUND
- commit 2c2c9ef: FOUND
