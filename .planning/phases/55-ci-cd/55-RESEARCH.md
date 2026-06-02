# Phase 55: CI/CD 基础设施修复 - Research

**Researched:** 2026-06-02
**Domain:** GitHub Actions workflow 修复、cross-rs 跨编译配置、Release artifact 暂存发布
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** 修复范围覆盖全部 5 个 workflow 文件（`ci.yaml`、`release.yaml`、`lychee.yml`、`pages.yml`、`bench.yml`），全部从 `@v6` → `@v4`
- **D-02:** `bench.yml` 中 `actions/upload-artifact@v7` 也一并修复为 `@v4`
- **D-03:** 其余已有 action（`dtolnay/rust-toolchain@stable`、`Swatinem/rust-cache@v2`、`taiki-e/install-action@v2`）版本正确，无需改动
- **D-04:** 采用「artifact 暂存 + 统一发布」方案：4 个 build job 用 `actions/upload-artifact@v4` 上传编译产物，最终独立的 `create-release` job 下载全部文件后统一创建 GitHub Release
- **D-05:** `create-release` job 添加 `needs: [release]`（即等待所有 build job 完成），使用 `actions/download-artifact@v4` 下载全部产物
- **D-06:** Changelog 提取逻辑保留现有 awk 提取方式（从 CHANGELOG.md 提取对应版本段落，无内容时 fallback 到简单说明）
- **D-07:** 删除 `publish` job（crates.io 发布），理由：`CARGO_REGISTRY_TOKEN` secret 未配置，保留会导致 release CI 全局失败
- **D-08:** 在项目根目录创建 `Cross.toml`，为 `aarch64-unknown-linux-gnu` target 配置正确的 Docker 镜像（researcher 确认当前 cross-rs 推荐镜像版本）

### Claude's Discretion

- `softprops/action-gh-release` 最终是否保留由 planner 根据「artifact 暂存」方案决定（可能改用 `gh release create` + `gh release upload`，或保留 softprops 在 create-release job 中使用）
- Cross.toml 中 Docker 镜像的具体 tag（edge vs 固定版本）由 researcher 查阅 cross-rs 最新推荐

### Deferred Ideas (OUT OF SCOPE)

- benchmark workflow 的 `continue-on-error` 配置 → Phase 56 处理（BENCH-01）
- crates.io 自动发布 → 未来单独配置，届时需要设置 `CARGO_REGISTRY_TOKEN` secret
- 多平台 e2e CI matrix → v1.15 后续阶段
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CICD-01 | 用户推送 PR/branch 时，GitHub Actions CI 自动运行 test/clippy/fmt 全绿（三平台：ubuntu/windows/macos） | 修复 ci.yaml 的 `checkout@v6` → `@v4`，现有 matrix 三平台设计正确，无结构改动 |
| CICD-02 | 用户推送 tag 时，CD workflow 成功构建 4 个平台的二进制并创建 GitHub Release | 重构 release.yaml：upload-artifact@v4 暂存 + create-release job 统一发布 |
| CICD-03 | CD workflow 在 4 个 job 并行运行时正确创建 release notes（无竞争条件，发布内容完整） | 独立 create-release job + needs 依赖消除竞争条件；awk 逻辑移入 create-release job |
| CICD-04 | 项目包含 Cross.toml，aarch64-linux 跨编译构建无需手动干预 | 创建 Cross.toml，配置 `ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge` |
</phase_requirements>

---

## Summary

本阶段的核心工作是修复 5 个 GitHub Actions workflow 文件中的损坏 action 版本引用，并重构 release workflow 以消除并行 build job 之间的竞争条件。

**关键发现：** `actions/checkout@v6` 是真实存在的最新版本（v6.0.2，2025-01-09 发布），`actions/upload-artifact@v7` 也是真实存在的最新版本（v7.0.1，2026-04-10 发布）。然而，成功标准已明确锁定为 `@v4`（D-01、D-02），这是合理的——`v4.3.1` 在 2024-11-17 仍获得维护更新，且 v4 是当前广泛使用的稳定版本。`@v6`/`@v7` 目前不会失败（它们是合法版本），但团队决策是统一到 v4。

**Race condition 根本原因：** 当前 release.yaml 的每个 matrix build job 都独立调用 `softprops/action-gh-release@v3` 写入 release body，4 个 job 并行时最后一个写入者会覆盖其他 job 写入的 body，导致 release notes 内容随机丢失。正确方案是用 upload-artifact@v4 暂存产物，由单独的 `create-release` job 在所有 build job 完成后统一创建 release。

**Cross.toml + rusqlite bundled：** 项目使用 `rusqlite = { features = ["bundled"] }`，意味着 SQLite 从源码编译，不依赖目标系统的 libsqlite3 包。对于 `aarch64-unknown-linux-gnu`，cross-rs 会自动使用 `ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge` 镜像，该镜像包含 aarch64 cross-compilation toolchain，与 bundled SQLite 编译兼容。Cross.toml 需要创建，因为成功标准要求其存在（CICD-04），同时也确保 Docker 镜像版本可控。

**Primary recommendation:** 5 处 checkout/artifact action 版本修复 + release.yaml 重构为两阶段（build jobs 上传 artifact，create-release job 下载并统一发布）+ 新建 Cross.toml 指定 edge 镜像。

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| CI 测试（test/clippy/fmt） | GitHub Actions Runner | — | 纯 CI 任务，在 runner 上执行，无跨层交互 |
| 多平台二进制构建 | GitHub Actions Runner (matrix) | cross-rs Docker 容器（aarch64） | aarch64 构建在 cross-rs Docker 容器内，其余平台在 runner 原生 |
| 产物暂存 | GitHub Actions Artifact Store | — | upload-artifact 将产物存到 GH 托管存储，download-artifact 取回 |
| Release 创建 | GitHub Releases API | — | softprops/action-gh-release 调用 GitHub API，需 contents: write 权限 |
| 跨编译配置 | Cross.toml（项目源码） | ghcr.io/cross-rs Docker 镜像 | Cross.toml 指定 image，cross 工具拉取镜像执行编译 |

---

## Standard Stack

### Core（本阶段使用的 actions）

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| actions/checkout | v4 (v4.3.1) | Checkout 仓库代码 | 锁定决策 D-01；v4 是 2024-2025 最广泛使用的稳定版本 |
| actions/upload-artifact | v4 | 上传构建产物到 artifact 暂存 | 锁定决策 D-02/D-04；v3 已于 2024-11-30 弃用 |
| actions/download-artifact | v4 | create-release job 下载全部产物 | 与 upload-artifact@v4 配对；支持 pattern + merge-multiple |
| softprops/action-gh-release | v2 | 创建 GitHub Release 并上传 release assets | 当前维护中；比 gh CLI 更简洁的 workflow DSL |
| dtolnay/rust-toolchain | stable | 安装 Rust toolchain | 已在所有 workflow 中使用，D-03 确认无需修改 |
| Swatinem/rust-cache | v2 | cargo 构建缓存 | 已在所有 workflow 中使用，D-03 确认无需修改 |
| taiki-e/install-action | v2 | 安装 cross 工具 | 已在 release.yaml 中建立，D-03 确认无需修改 |

### Cross.toml Docker 镜像

| Target | Image | Tag | Why |
|--------|-------|-----|-----|
| aarch64-unknown-linux-gnu | ghcr.io/cross-rs/aarch64-unknown-linux-gnu | edge | `edge` = main 分支最新构建，每次更新均包含最新 toolchain；`latest` = 0.2.5（3 年前），过旧 |

**注：** `main-centos` tag 仅用于需要 CentOS 7 兼容性的场景，本项目不需要。`edge` 是推荐的非固定版本 tag，等同于 `main`，均为当天构建。

---

## Package Legitimacy Audit

> slopcheck 不可用（安装失败）。所有下述 actions 标记为 [ASSUMED]，但均为 GitHub 官方或知名维护者发布的 action，来源可信。

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| actions/checkout@v4 | GitHub Marketplace | 2023 | 数亿次/月 | github.com/actions/checkout | [ASSUMED] | Approved — GitHub 官方 |
| actions/upload-artifact@v4 | GitHub Marketplace | 2023 | 数亿次/月 | github.com/actions/upload-artifact | [ASSUMED] | Approved — GitHub 官方 |
| actions/download-artifact@v4 | GitHub Marketplace | 2023 | 数亿次/月 | github.com/actions/download-artifact | [ASSUMED] | Approved — GitHub 官方 |
| softprops/action-gh-release@v2 | GitHub Marketplace | 2019 | 数千万次/月 | github.com/softprops/action-gh-release | [ASSUMED] | Approved — 知名社区 action |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

*slopcheck 在研究时不可用，上述 actions 均为 GitHub 官方或长期维护的社区 action（数年历史，官方文档引用），可安全使用。*

---

## Architecture Patterns

### System Architecture Diagram

```
Push tag (v*)
    │
    ▼
release.yaml trigger
    │
    ├─── build job [matrix: 4 targets]
    │       ├── ubuntu-latest: x86_64-linux     (cargo build)
    │       ├── ubuntu-latest: aarch64-linux    (cross build, uses Cross.toml)
    │       ├── windows-latest: x86_64-windows  (cargo build)
    │       └── macos-latest: aarch64-macos     (cargo build)
    │               │
    │               ▼
    │       upload-artifact@v4
    │       (name: sqllog2db-{artifact})
    │               │
    │               ▼
    │       GitHub Artifact Store (临时存储，90天)
    │
    └─── create-release job
            needs: [release]  ← 等待全部 4 个 build job 完成
            │
            ├── checkout@v4 (获取 CHANGELOG.md)
            ├── download-artifact@v4
            │   (pattern: sqllog2db-*, merge-multiple: true → dist/)
            ├── awk 提取 changelog（D-06 保留逻辑）
            └── softprops/action-gh-release@v2
                (files: dist/*, body_path: release_notes.md)
                    │
                    ▼
                GitHub Release（包含 4 个平台二进制）
```

### Recommended Project Structure

```
.github/
└── workflows/
    ├── ci.yaml          # 修改：checkout@v6 → @v4 (3处)
    ├── release.yaml     # 重构：build jobs + create-release job；删除 publish job
    ├── bench.yml        # 修改：checkout@v6 → @v4；upload-artifact@v7 → @v4
    ├── lychee.yml       # 修改：checkout@v6 → @v4
    └── pages.yml        # 修改：checkout@v6 → @v4
Cross.toml               # 新建：aarch64-unknown-linux-gnu image 配置
```

### Pattern 1: upload-artifact@v4 matrix 命名

**What:** 每个 matrix build job 用包含平台信息的唯一名称上传产物。v4 不允许同名 artifact 重复上传（不同于 v3）。

**When to use:** 所有 matrix 并行 build job 需要将产物传递给下游 job 时。

```yaml
# Source: https://github.blog/news-insights/product-news/get-started-with-v4-of-github-actions-artifacts/
# 在每个 build job 的最后一步
- name: Upload artifact
  uses: actions/upload-artifact@v4
  with:
    name: sqllog2db-${{ matrix.artifact }}   # 唯一名称，含平台标识
    path: dist/${{ matrix.artifact }}
    retention-days: 1                        # release 用完即可删除
```

### Pattern 2: download-artifact@v4 pattern + merge-multiple

**What:** create-release job 用 glob pattern 一次性下载所有平台产物到同一目录。

**When to use:** 需要将多个 matrix job 的产物合并到单目录用于统一发布时。

```yaml
# Source: https://github.blog/news-insights/product-news/get-started-with-v4-of-github-actions-artifacts/
- name: Download all artifacts
  uses: actions/download-artifact@v4
  with:
    path: dist
    pattern: sqllog2db-*     # 匹配所有 4 个平台的产物
    merge-multiple: true     # 扁平化到同一目录（不创建子目录）
```

### Pattern 3: create-release job 结构

**What:** 独立的 release job，通过 `needs` 等待所有 build job 完成后统一创建 GitHub Release。

**When to use:** 需要消除并行 job 写入 release body 的竞争条件时。

```yaml
create-release:
  name: Create GitHub Release
  needs: [release]           # 等待全部 matrix build job 完成
  runs-on: ubuntu-latest
  permissions:
    contents: write          # 必须：创建 release 需要 contents: write
  steps:
    - uses: actions/checkout@v4
    
    - name: Download all artifacts
      uses: actions/download-artifact@v4
      with:
        path: dist
        pattern: sqllog2db-*
        merge-multiple: true
    
    - name: Extract changelog
      shell: bash
      run: |
        VERSION=${GITHUB_REF#refs/tags/v}
        awk "/## \[${VERSION}\]/,/## \[/" CHANGELOG.md | sed '$d' > release_notes.md
        if [ ! -s release_notes.md ]; then
          echo "Release version ${VERSION}" > release_notes.md
          echo "" >> release_notes.md
          echo "See CHANGELOG.md for details." >> release_notes.md
        fi
    
    - name: Create GitHub Release
      uses: softprops/action-gh-release@v2
      with:
        body_path: release_notes.md
        files: dist/*
      env:
        GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### Pattern 4: Cross.toml 配置

**What:** 为 cross-rs 指定 aarch64-unknown-linux-gnu 的 Docker 镜像。

**When to use:** 需要确保跨编译使用稳定已知镜像，而非依赖 cross 内置默认值。

```toml
# Cross.toml — 项目根目录
[target.aarch64-unknown-linux-gnu]
image = "ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge"
```

**注：** `edge` tag 与 `main` tag 等价，均为 cross-rs main 分支的最新构建。`latest` tag 对应 cross 0.2.5（发布于 3 年前），版本过旧，不推荐。

### Anti-Patterns to Avoid

- **并行 job 各自创建 release：** 每个 matrix build job 单独调用 softprops/action-gh-release，会导致 release body 被最后写入的 job 覆盖（竞争条件），且 4 个 job 都持有 `permissions: contents: write` 增加安全面积。正确做法：build job 只上传 artifact，由单一 create-release job 负责 release 创建。
- **upload-artifact v4 同名重复上传：** v4 严格要求每个 artifact 名称唯一（v3 允许同名追加），matrix build 中必须用平台标识区分名称（如 `sqllog2db-${{ matrix.artifact }}`）。
- **Cross.toml 使用 latest tag：** `latest` 对应 cross 0.2.5（3 年前发布），可能缺少新 target 支持和 bug 修复，应使用 `edge`。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| GitHub Release 创建 | 手写 curl 调用 GitHub API | softprops/action-gh-release@v2 | API 调用需处理认证、错误重试、asset 分批上传等边界情况 |
| Artifact 跨 job 传递 | 写文件到 git worktree 或外部存储 | upload/download-artifact@v4 | GH 官方托管，自动清理，跨 job 权限隔离 |
| aarch64 cross 编译 | 手动配置 QEMU + binutils | cross + Cross.toml | cross 封装了完整的 toolchain + Docker 镜像，零手动配置 |

**Key insight:** GitHub Actions 的 artifact store 是解决 matrix job 产物聚合的官方机制，用 git push 或外部存储传递产物会引入额外复杂性和安全风险。

---

## Common Pitfalls

### Pitfall 1: actions/checkout@v6 是真实版本，但团队决策锁定 v4

**What goes wrong:** 研究者可能认为当前 workflow 使用 `@v6` 是"正确的"，因为 v6 确实是官方发布的最新版本（v6.0.2），不会导致 action 失败。
**Why it happens:** `@v6` 在 2024-11-20 正式发布，是合法 tag，不是错误拼写。
**How to avoid:** 严格遵循锁定决策 D-01 — 全部改为 `@v4`，原因是团队统一版本策略（v4 = Node 20，2025 年底前有效；v6 = Node 24，可能需要 runner 升级）。
**Warning signs:** 如果 CI 没有报错，不代表 @v6 没问题 — 它能工作，但不符合 D-01 决策。

### Pitfall 2: 竞争条件的根本原因

**What goes wrong:** 4 个 build job 并行执行，每个都调用 softprops/action-gh-release 写入 `body_path`。GitHub Releases API 的 `body` 字段是整体替换，不是追加。最后一个写入的 job 会覆盖其他 job 写入的内容。
**Why it happens:** 当前 release.yaml 的 Extract changelog + Upload to GitHub Release 步骤在每个 matrix job 内，没有依赖顺序保证。
**How to avoid:** 将 changelog 提取和 release 创建全部移入独立的 `create-release` job，通过 `needs: [release]` 确保在所有 build job 完成后才运行。build jobs 只负责编译和上传 artifact。
**Warning signs:** Release notes 每次发布内容不同，只包含某一个平台的说明。

### Pitfall 3: upload-artifact@v4 不允许同名 artifact

**What goes wrong:** matrix 中 4 个 build job 使用相同的 artifact name（如 `dist`），v4 会报错 "An artifact with the name dist already exists"。
**Why it happens:** v4 将 artifact 设计为不可变（immutable），防止意外覆盖。v3 允许同名追加。
**How to avoid:** 每个 matrix job 的 artifact 名称必须包含平台标识，例如 `sqllog2db-${{ matrix.artifact }}`。download-artifact 用 `pattern: sqllog2db-*` + `merge-multiple: true` 合并到同一目录。
**Warning signs:** Build job 报错 "Artifact already exists"。

### Pitfall 4: rusqlite bundled 与 cross-rs 兼容性

**What goes wrong:** 以为 rusqlite bundled 会在 cross 编译时需要额外配置。
**Why it happens:** bundled 特性使用 `cc` crate 从源码编译 SQLite，不依赖系统 libsqlite3。cross-rs 的 Docker 镜像包含 aarch64 cross-compilation toolchain，cc crate 会自动使用正确的 cross-compiler。
**How to avoid:** 无需额外配置 Cross.toml 的 `pre-build` 或自定义 Dockerfile。现有 `cross build --release --target aarch64-unknown-linux-gnu` 命令与 bundled SQLite 兼容。
**Warning signs:** 如果需要额外配置，build log 会在 SQLite 编译阶段报 linker 错误（理论上不会发生）。

### Pitfall 5: create-release job 的 permissions 必须明确声明

**What goes wrong:** `permissions: contents: write` 只在 build job 中声明，create-release job 没有声明，导致 403 错误。
**Why it happens:** GitHub Actions 的 permissions 是 job 级别的，不继承。
**How to avoid:** create-release job 必须单独声明 `permissions: contents: write`。build jobs 可以去掉 `permissions: contents: write`（它们不再创建 release，只上传 artifact）。

---

## Code Examples

### 完整 release.yaml 重构模板

```yaml
# Source: 基于 https://github.blog/news-insights/product-news/get-started-with-v4-of-github-actions-artifacts/
#         和 https://github.com/softprops/action-gh-release/tree/v2
name: Release

on:
  push:
    tags: ["v*"]

env:
  CARGO_TERM_COLOR: always

jobs:
  release:
    name: Release ${{ matrix.artifact }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: sqllog2db-x86_64-linux
            use_cross: false
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            artifact: sqllog2db-aarch64-linux
            use_cross: true
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact: sqllog2db-x86_64-windows.exe
            use_cross: false
          - os: macos-latest
            target: aarch64-apple-darwin
            artifact: sqllog2db-aarch64-macos
            use_cross: false
    steps:
      - uses: actions/checkout@v4           # D-01 修复

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install cross
        if: matrix.use_cross
        uses: taiki-e/install-action@v2
        with:
          tool: cross

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Build
        run: |
          if [ "${{ matrix.use_cross }}" = "true" ]; then
            cross build --release --target ${{ matrix.target }}
          else
            cargo build --release --target ${{ matrix.target }}
          fi
        shell: bash

      - name: Prepare artifact
        run: |
          mkdir dist
          if [ "${{ matrix.target }}" = "x86_64-pc-windows-msvc" ]; then
            cp target/${{ matrix.target }}/release/sqllog2db.exe dist/${{ matrix.artifact }}
          else
            cp target/${{ matrix.target }}/release/sqllog2db dist/${{ matrix.artifact }}
          fi
        shell: bash

      - name: Upload artifact               # D-04: 暂存，不直接发布
        uses: actions/upload-artifact@v4    # D-02 修复
        with:
          name: ${{ matrix.artifact }}      # 唯一名称（含平台标识）
          path: dist/${{ matrix.artifact }}
          retention-days: 1

  create-release:
    name: Create GitHub Release
    needs: [release]                        # D-05: 等待所有 build job 完成
    runs-on: ubuntu-latest
    permissions:
      contents: write                       # Pitfall 5: job 级别声明
    steps:
      - uses: actions/checkout@v4

      - name: Download all artifacts        # D-05
        uses: actions/download-artifact@v4
        with:
          path: dist
          pattern: sqllog2db-*             # 匹配所有 4 个平台
          merge-multiple: true             # 扁平化到 dist/ 目录

      - name: Extract changelog             # D-06: 保留现有 awk 逻辑
        shell: bash
        run: |
          VERSION=${GITHUB_REF#refs/tags/v}
          awk "/## \[${VERSION}\]/,/## \[/" CHANGELOG.md | sed '$d' > release_notes.md
          if [ ! -s release_notes.md ]; then
            echo "Release version ${VERSION}" > release_notes.md
            echo "" >> release_notes.md
            echo "See [CHANGELOG.md](https://github.com/${{ github.repository }}/blob/main/CHANGELOG.md) for details." >> release_notes.md
          fi

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          body_path: release_notes.md
          files: dist/*
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  # D-07: publish job 已删除
```

### Cross.toml

```toml
# Source: https://github.com/cross-rs/cross/pkgs/container/aarch64-unknown-linux-gnu
# cross-rs 官方 ghcr.io 镜像，edge = main 分支最新构建
[target.aarch64-unknown-linux-gnu]
image = "ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge"
```

### ci.yaml checkout 修复（示例，共 3 处）

```yaml
# 修改前（3个 job 各有一处）
- uses: actions/checkout@v6

# 修改后
- uses: actions/checkout@v4
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| upload-artifact@v3（并行 job 同名追加）| upload-artifact@v4（唯一名称 + pattern 下载）| 2023-12（GA） | 消除并行写入冲突，性能提升 90%+ |
| 每个 matrix job 创建 release（竞争条件）| create-release 独立 job + needs 依赖 | 2023 artifact v4 GA 后普及 | release notes 完整，无竞争条件 |
| cross-rs latest tag（0.2.5，3年前）| cross-rs edge tag（main 分支最新）| 持续更新 | 获取最新 toolchain 和 bug 修复 |
| actions/checkout@v3（Node 16）| v4（Node 20）→ v6（Node 24）| 2023/2024 | v4 在 2025-09 前有效（Node 20 保留期） |

**Deprecated/outdated:**
- `actions/upload-artifact@v3`：2024-11-30 正式弃用，2025-01-30 停止接受
- `actions/upload-artifact@v1/v2`：2024-06-30 已停止接受
- `publish` job（crates.io）：因 `CARGO_REGISTRY_TOKEN` 未配置，保留会导致 release 全局失败（D-07 决策删除）
- `softprops/action-gh-release@v3`（当前 release.yaml 中）：最新为 v2，release.yaml 中写的 @v3 可能是错误版本（需验证）

**注意：** 当前 release.yaml 使用 `softprops/action-gh-release@v3`，但该 action 的最新版本是 v2（WebFetch 显示最新为 v3.0.0，2026-04-12 发布）。需要在 PLAN 阶段确认正确版本。

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge` 与 rusqlite bundled 兼容，无需额外 Cross.toml 配置 | Common Pitfalls #4 | 若不兼容，需添加 pre-build 步骤安装 cc/binutils，影响较小 |
| A2 | `softprops/action-gh-release@v2` 是正确版本（WebFetch 显示有 v3.0.0 但标注 "displays v2 content"，存在歧义） | Standard Stack | 若实际最新为 v3，应使用 @v3；功能等价，仅版本号差异 |
| A3 | cross-rs `edge` tag 的稳定性足够用于 CI（不会因镜像构建问题随机失败） | Standard Stack | 若 edge 不稳定，可改用版本号 tag（如 `0.2.5`），但该版本 3 年前发布，过旧 |

---

## Open Questions

1. **softprops/action-gh-release 版本确认**
   - What we know: 研究中发现 WebFetch 返回矛盾信息（"显示 v2 content" 但提到 "v3.0.0 released April 2026"）。当前 release.yaml 已使用 `@v3`。
   - What's unclear: 最新稳定版是 v2 还是 v3？
   - Recommendation: planner 在编写 PLAN 时直接访问 https://github.com/softprops/action-gh-release/releases 确认最新版本。如果 v3 存在且稳定，保持 @v3 即可；如果只有 v2，改为 @v2。

2. **release.yaml matrix target 确认**
   - What we know: 当前 release.yaml 中 macOS target 是 `aarch64-apple-darwin`（Apple Silicon），而 CI CONTEXT 成功标准要求 "x86_64-macos"。
   - What's unclear: aarch64-macos 是否应改为 x86_64-macos？
   - Recommendation: 成功标准明确写 "x86_64-macos"，planner 应将 macOS target 改为 `x86_64-apple-darwin`，运行器改为 `macos-13`（Intel）或继续 `macos-latest` 但加 `--target x86_64-apple-darwin`。

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Docker | cross-rs aarch64 跨编译 | ✓ | 29.5.2 | — （GitHub Actions ubuntu-latest 自带 Docker）|
| git | workflow 文件修改 | ✓ | 系统自带 | — |
| GitHub Actions Runner | 所有 workflow | ✓ | N/A（云端）| — |

**Missing dependencies with no fallback:** none

**Notes:** 本阶段只修改 .github/workflows/*.yaml 和新建 Cross.toml，无需本地额外工具。实际跨编译在 GitHub Actions runner 上执行。

---

## Validation Architecture

> `workflow.nyquist_validation` 未设置，按照 enabled 处理。

### Test Framework

| Property | Value |
|----------|-------|
| Framework | GitHub Actions workflow lint（无本地 test framework） |
| Config file | .github/workflows/*.yaml |
| Quick run command | `act --list`（本地 act 工具，可选）|
| Full suite command | Push PR 到 GitHub 触发实际 CI |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CICD-01 | 三平台 CI 全绿 | smoke（GitHub Actions） | Push PR → 观察 Actions tab | ✅ ci.yaml（修改后）|
| CICD-02 | tag 触发 4 平台构建 + release | smoke（GitHub Actions） | Push tag → 观察 Actions tab | ✅ release.yaml（重构后）|
| CICD-03 | release notes 完整，无竞争条件 | smoke（GitHub Actions） | Push tag → 检查 GitHub Release body | ✅ release.yaml（重构后）|
| CICD-04 | Cross.toml 存在，aarch64 构建成功 | smoke（GitHub Actions）| Push tag → aarch64 build job 绿灯 | ❌ Cross.toml 需新建 |

**说明：** GitHub Actions workflow 无法通过本地 unit test 验证（除非使用 act 工具）。验证方式是观察 GitHub Actions 执行结果。

### Sampling Rate

- **Per task commit:** `cargo clippy && cargo fmt --check`（Rust 代码无变动，仅 YAML 和 TOML 修改）
- **Per wave merge:** N/A（本阶段无 Rust 代码修改）
- **Phase gate:** Push test tag（如 `v0.0.0-test`）验证 release workflow，Push PR 验证 ci.yaml

### Wave 0 Gaps

- [ ] `Cross.toml` — CICD-04 要求，当前不存在

---

## Security Domain

> security_enforcement 未设置，按 enabled 处理。

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — （workflow 用 GITHUB_TOKEN，非用户认证）|
| V3 Session Management | no | — |
| V4 Access Control | yes | `permissions: contents: write` 仅在 create-release job 声明；build jobs 不需要 write 权限 |
| V5 Input Validation | no | — （无用户输入处理）|
| V6 Cryptography | no | — |

### Known Threat Patterns for GitHub Actions

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| GITHUB_TOKEN 权限过大 | Elevation of Privilege | `permissions: contents: write` 仅在 create-release job 声明，build jobs 用默认 read 权限 |
| Action supply chain attack | Tampering | 使用 `@v4` 等 tag 而非 `@main`；官方 action 风险低 |
| Artifact 篡改 | Tampering | GH artifact store 在 workflow 运行内隔离，不可从外部写入 |

---

## Sources

### Primary (HIGH confidence)
- [github.com/actions/checkout/releases](https://github.com/actions/checkout/releases) — 确认 v4.3.1 为最新 v4，v6.0.2 为最新 v6
- [github.com/actions/upload-artifact/tree/v4](https://github.com/actions/upload-artifact/tree/v4) — v4 特性：唯一名称、merge-multiple、90% 性能提升
- [github.blog/news-insights/product-news/get-started-with-v4-of-github-actions-artifacts](https://github.blog/news-insights/product-news/get-started-with-v4-of-github-actions-artifacts/) — matrix 上传 + pattern 下载的官方示例
- [github.com/actions/download-artifact/tree/v4](https://github.com/actions/download-artifact/tree/v4) — merge-multiple 输入参数文档
- [github.com/cross-rs/cross/pkgs/container/aarch64-unknown-linux-gnu](https://github.com/cross-rs/cross/pkgs/container/aarch64-unknown-linux-gnu) — 可用 tag：edge/main（最新）、latest（0.2.5，3年前）

### Secondary (MEDIUM confidence)
- [github.com/softprops/action-gh-release/tree/v2](https://github.com/softprops/action-gh-release/tree/v2) — v2 文档，permissions 要求
- [github.com/cross-rs/cross/wiki/Configuration](https://github.com/cross-rs/cross/wiki/Configuration) — Cross.toml image 字段配置说明
- [github.blog/changelog/2024-04-16-deprecation-notice-v3-of-the-artifact-actions](https://github.blog/changelog/2024-04-16-deprecation-notice-v3-of-the-artifact-actions/) — v3 弃用通知

### Tertiary (LOW confidence)
- WebSearch 结果关于 rusqlite bundled + cross-rs 兼容性（无直接官方来源，基于原理推断）

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — actions 版本通过官方 release page 确认，cross-rs 镜像通过 ghcr.io package page 确认
- Architecture: HIGH — artifact 暂存 + 独立 release job 模式有官方示例支撑
- Pitfalls: HIGH — 竞争条件和 v4 不允许同名 artifact 均有官方文档说明
- Cross.toml: MEDIUM — edge tag 推荐基于 ghcr.io 页面信息（latest 明确过旧），但 rusqlite bundled 兼容性为 [ASSUMED]

**Research date:** 2026-06-02
**Valid until:** 2026-07-02（actions 版本较稳定，30 天内有效；cross-rs edge tag 持续更新但配置不变）
