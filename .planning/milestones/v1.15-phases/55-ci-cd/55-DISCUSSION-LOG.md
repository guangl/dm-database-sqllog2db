# Phase 55: CI/CD 基础设施修复 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-02
**Phase:** 55-CI/CD 基础设施修复
**Areas discussed:** release 架构重构, action 版本修复范围, publish job 处理方式

---

## Release 架构重构

| Option | Description | Selected |
|--------|-------------|----------|
| A：create-release job 独立先跑 | 1 个 create-release job 只负责提取 changelog + 创建 release body；4 个 build job 添加 needs: [create-release]，只上传文件（不再传 body_path）。最少改动，保留现有 softprops/action-gh-release | |
| B：artifact 暂存 + 统一发布 | 4 个 build job 用 upload-artifact 上传二进制；最后 1 个 release job 下载全部文件 + 创建 release。最干净但改动较大 | ✓ |

**User's choice:** B — artifact 暂存 + 统一发布
**Notes:** 无

---

## create-release 的 changelog 提取

| Option | Description | Selected |
|--------|-------------|----------|
| CHANGELOG.md 文本提取（当前方式） | 用 awk 从 CHANGELOG.md 提取对应版本的段落，无笔记符时 fallback 到简单说明。保留现有逻辑 | ✓ |
| GitHub 自动生成 release notes | 不提取 CHANGELOG.md，使用 GitHub 内置的 auto-generate release notes（基于 PR 标题 + commit） | |

**User's choice:** CHANGELOG.md 文本提取（当前方式）
**Notes:** 无

---

## Action 版本修复范围

| Option | Description | Selected |
|--------|-------------|----------|
| 全部 workflow 文件 | 顺手把 lychee.yml、pages.yml、bench.yml 也一起修成 v4，避免后续独立修复 | ✓ |
| 只修 ci.yaml + release.yaml | 仅修 Phase 55 需求明确提及的两个文件 | |

**User's choice:** 全部 workflow 文件
**Notes:** 包括 bench.yml 的 `actions/upload-artifact@v7` → `@v4`

---

## Publish Job 处理方式

| Option | Description | Selected |
|--------|-------------|----------|
| 删除 publish job | 目前项目未配置 CARGO_REGISTRY_TOKEN secret，保留只会导致 release CI 失败 | ✓ |
| 加 continue-on-error: true | 保留但不阻塞 release，secret 未配置时静默失败 | |
| 保留原样（假设 secret 已配置） | secret 未设置则 release CI 全局失败 | |

**User's choice:** 删除 publish job
**Notes:** 无

---

## Claude's Discretion

- `softprops/action-gh-release` 是否在最终 release job 中继续使用，或改用 `gh release create` + `gh release upload`
- Cross.toml 中 Docker 镜像的具体 tag（edge vs 固定版本）

## Deferred Ideas

- benchmark workflow 的 `continue-on-error` 配置 → Phase 56（BENCH-01）
- crates.io 自动发布 → 未来单独配置，届时需设置 `CARGO_REGISTRY_TOKEN` secret
- 多平台 e2e CI matrix → v1.15 后续阶段
