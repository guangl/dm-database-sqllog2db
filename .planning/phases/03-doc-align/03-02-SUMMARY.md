---
phase: 03-doc-align
plan: "02"
subsystem: docs/readme
tags: [docs, readme, watch, init-interactive, quiet-verbose]
requirements_completed: [DOC-04]

dependency_graph:
  requires: [03-01]
  provides: [readme-watch-entry, readme-init-interactive-example, readme-quiet-verbose-example]
  affects: [README.md]

tech_stack:
  added: []
  patterns: [README 功能特性条目格式（粗体标题 + em dash + 中文描述）, 快速入门独立 bash fenced code block 格式]

key_files:
  modified:
    - README.md

decisions:
  - "D-05: 功能特性 CLI 条目从四个命令扩展为五个命令，加入 watch（持续监听）"
  - "D-06: 新增持续监听功能特性条目，描述目录监听/500ms防抖/增量处理/Ctrl+C摘要四要素"
  - "D-07: 快速入门追加 watch/init --interactive/quiet+verbose 三段示例，与现有 stats 示例风格一致"

metrics:
  duration: "346s (~6m)"
  tasks_completed: 2
  files_modified: 1
  completed_date: "2026-06-07T07:22:29Z"
---

# Phase 03 Plan 02: README 补充 watch/init --interactive/quiet+verbose 说明 Summary

README 功能特性列出 5 命令并描述 watch 行为，快速入门含 watch / init --interactive / quiet+verbose 三段示例，DOC-04 三项子需求全部覆盖，三道质量门禁（fmt / clippy / test）全绿。

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | D-05 + D-06 — 更新 CLI 条目为 5 命令并新增"持续监听"功能特性条目 | 7d18b52 | README.md |
| 2 | D-07 — 快速入门追加 watch / init --interactive / quiet+verbose 三段示例 | ccca7c5 | README.md |

## What Was Built

**README.md** 三处更新：

1. **功能特性 CLI 条目（D-05）** — 第 41 行从"四个命令"升级为"五个命令"：
   - 旧：`init`（生成配置）、`validate`（校验）、`run`（执行导出）、`stats`（统计分析）四个命令
   - 新：`init`（生成配置）、`validate`（校验）、`run`（执行导出）、`stats`（统计分析）、`watch`（持续监听）五个命令

2. **持续监听功能特性条目（D-06）** — 在 CLI 条目之后、`## 架构` 之前新增：
   - `- **持续监听**：`watch` 子命令监听配置目录下的新 `.log` 文件，500ms 防抖后自动触发增量处理，Ctrl+C 退出并打印本次运行摘要（处理次数、总行数、运行时长）。`
   - 包含四个关键能力：目录监听、500ms 防抖、增量处理、Ctrl+C 摘要

3. **快速入门三段示例（D-07）** — 插入在最后一个 stats 代码块之后、"详细用法参见"链接之前：
   - 段落 1（watch）：说明句 + bash 代码块（`sqllog2db watch -c config.toml`）
   - 段落 2（init --interactive）：说明句 + bash 代码块（`sqllog2db init --interactive`）
   - 段落 3（quiet/verbose）：说明句 + bash 代码块（`sqllog2db run -c config.toml --quiet` / `--verbose`）

## Quality Gates

| Gate | Result |
|------|--------|
| `cargo fmt -- --check` | PASS — 无 diff |
| `cargo clippy --all-targets -- -D warnings` | PASS — 无 warning |
| `cargo test --all-features` | PASS — 全部通过（2 ignored：macOS FSEvents 已知限制）|

## Self-Check

对照 `must_haves.truths` 逐条验证：

- [x] README"功能特性 → 配置与性能"节的 CLI 条目列出 5 个命令（init/validate/run/stats/watch），不再是 4 个 — VERIFIED (行 41)
- [x] README 功能特性区域新增一条"持续监听"条目，描述 watch 行为（目录监听、500ms 防抖、增量处理、Ctrl+C 摘要）— VERIFIED (行 42)
- [x] README 快速入门章节包含 watch 用法示例（`sqllog2db watch -c config.toml`）— VERIFIED (行 133)
- [x] README 快速入门章节包含 init --interactive 用法示例（`sqllog2db init --interactive`）— VERIFIED (行 139)
- [x] README 快速入门章节包含 --quiet / --verbose 进度选项说明 + 示例 — VERIFIED (行 142-148)
- [x] 新增段落使用与现有功能特性条目（粗体标题 + em dash + 中文描述）和快速入门（独立 bash fenced code block）一致的风格 — VERIFIED
- [x] 现有非 watch 相关段落、解析与导出/过滤与字段控制/架构/性能/安装等章节保持不变 — VERIFIED

## Self-Check: PASSED

所有 `must_haves.truths` 全部满足。

## README Diff Summary

**功能特性区域变更（第 41-42 行）：**
- 第 41 行：`四个命令` → `、`watch`（持续监听）五个命令`
- 第 42 行（新增）：`- **持续监听**：...`

**快速入门区域变更（第 130-148 行之间新增 19 行）：**
- 原第 129 行（现第 149 行）"详细用法参见"之前插入三段示例
- 每段格式：1 行说明句 + 空行 + bash fenced code block + 空行

**未变动章节：** 解析与导出、过滤与字段控制、架构、安装、性能、配置、CHANGELOG 等所有其他章节。

## Deviations from Plan

None — 计划按原文执行，无偏差。

## Known Stubs

None.

## Threat Flags

None — 本计划仅修改 README.md 文档，无新增网络端点、认证路径、文件访问或 schema 变更。
