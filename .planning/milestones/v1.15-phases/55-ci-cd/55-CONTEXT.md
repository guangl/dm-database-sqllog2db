# Phase 55: CI/CD 基础设施修复 - Context

**Gathered:** 2026-06-02
**Status:** Ready for planning

<domain>
## Phase Boundary

修复 GitHub Actions workflow 文件中损坏的 action 版本，重构 release workflow 消除并行 job 竞争条件，添加 Cross.toml 支持 aarch64-linux 跨编译。

**不包括**：benchmark CI 门控（Phase 56）、e2e 测试（Phase 57）、代码拆分（Phase 58）

</domain>

<decisions>
## Implementation Decisions

### Action 版本修复

- **D-01:** 修复范围覆盖全部 5 个 workflow 文件（`ci.yaml`、`release.yaml`、`lychee.yml`、`pages.yml`、`bench.yml`），全部从 `@v6` → `@v4`
- **D-02:** `bench.yml` 中 `actions/upload-artifact@v7` 也一并修复为 `@v4`
- **D-03:** 其余已有 action（`dtolnay/rust-toolchain@stable`、`Swatinem/rust-cache@v2`、`taiki-e/install-action@v2`）版本正确，无需改动

### Release Workflow 架构重构

- **D-04:** 采用「artifact 暂存 + 统一发布」方案：4 个 build job 用 `actions/upload-artifact@v4` 上传编译产物，最终独立的 `create-release` job 下载全部文件后统一创建 GitHub Release
- **D-05:** `create-release` job 添加 `needs: [release]`（即等待所有 build job 完成），使用 `actions/download-artifact@v4` 下载全部产物
- **D-06:** Changelog 提取逻辑保留现有 awk 提取方式（从 CHANGELOG.md 提取对应版本段落，无内容时 fallback 到简单说明）
- **D-07:** 删除 `publish` job（crates.io 发布），理由：`CARGO_REGISTRY_TOKEN` secret 未配置，保留会导致 release CI 全局失败

### Cross.toml

- **D-08:** 在项目根目录创建 `Cross.toml`，为 `aarch64-unknown-linux-gnu` target 配置正确的 Docker 镜像（planner/researcher 确认当前 cross-rs 推荐镜像版本）

### Claude's Discretion

- `softprops/action-gh-release` 最终是否保留由 planner 根据「artifact 暂存」方案决定（可能改用 `gh release create` + `gh release upload`，或保留 softprops 在 create-release job 中使用）
- Cross.toml 中 Docker 镜像的具体 tag（edge vs 固定版本）由 researcher 查阅 cross-rs 最新推荐

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求定义
- `.planning/ROADMAP.md` §"Phase 55: CI/CD 基础设施修复" — 成功标准（4 条），requirements 映射
- `.planning/REQUIREMENTS.md` §"CI/CD 基础设施" — CICD-01/02/03/04 原文

### 当前 Workflow 文件（均需修改）
- `.github/workflows/ci.yaml` — CI workflow（test/lint/coverage 三个 job）
- `.github/workflows/release.yaml` — Release workflow（4 matrix build job + publish job）
- `.github/workflows/bench.yml` — Benchmark workflow（也有损坏的 action 版本）
- `.github/workflows/lychee.yml` — Link checker workflow
- `.github/workflows/pages.yml` — GitHub Pages 部署 workflow

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- 现有 `release.yaml` 的 matrix 定义（4 个平台配置）可直接复用，只需重构 upload 部分
- 现有 changelog 提取 awk 脚本（release.yaml 中的 shell 步骤）可移入 create-release job

### Established Patterns
- `taiki-e/install-action@v2` 安装 cross 的模式已在 release.yaml 中建立，保持不变
- `Swatinem/rust-cache@v2` 缓存模式在所有 workflow 中统一使用

### Integration Points
- `create-release` job 必须等待所有 4 个 build job 完成（`needs: [release]`），确保所有平台产物都上传后再创建 GitHub Release

</code_context>

<specifics>
## Specific Ideas

- 成功标准明确要求 `actions/checkout@v4` 和 `actions/upload-artifact@v4`，这两个版本号已锁定
- 成功标准明确要求「独立 create-release job 先于 upload-artifact job 运行」→ D-04/D-05 已体现
- D-07（删除 publish job）是本次讨论中最具影响力的决策，避免因 secret 缺失导致 release 全局失败

</specifics>

<deferred>
## Deferred Ideas

- benchmark workflow 的 `continue-on-error` 配置 → Phase 56 处理（BENCH-01）
- crates.io 自动发布 → 未来单独配置，届时需要设置 `CARGO_REGISTRY_TOKEN` secret
- 多平台 e2e CI matrix → v1.15 后续阶段

</deferred>

---

*Phase: 55-CI/CD 基础设施修复*
*Context gathered: 2026-06-02*
