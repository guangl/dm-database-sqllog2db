# Phase 55: CI/CD 基础设施修复 - Pattern Map

**Mapped:** 2026-06-02
**Files analyzed:** 6 (5 modified + 1 created)
**Analogs found:** 5 / 6 (Cross.toml 无现有类比，参考 RESEARCH.md 模板)

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `.github/workflows/ci.yaml` | config (CI) | request-response | `.github/workflows/ci.yaml` 自身（版本修复） | exact — 原文件即类比，仅改 checkout 版本 |
| `.github/workflows/release.yaml` | config (CD) | batch + event-driven | `.github/workflows/release.yaml` 自身（重构） | exact — 现有 matrix/build 结构保留，重构 upload 和 release 阶段 |
| `.github/workflows/bench.yml` | config (CI) | batch | `.github/workflows/bench.yml` 自身（版本修复） | exact — 原文件即类比，修复两个 action 版本 |
| `.github/workflows/lychee.yml` | config (CI) | request-response | `.github/workflows/lychee.yml` 自身（版本修复） | exact — 原文件即类比，仅改 checkout 版本 |
| `.github/workflows/pages.yml` | config (CI/CD) | event-driven | `.github/workflows/pages.yml` 自身（版本修复） | exact — 原文件即类比，仅改 checkout 版本 |
| `Cross.toml` | config (cross-compile) | batch | 无现有类比 | none — 项目首次引入 |

---

## Pattern Assignments

### `.github/workflows/ci.yaml` (config, request-response)

**修改范围：** 仅替换 3 处 `actions/checkout@v6` → `actions/checkout@v4`，其余结构不变。

**当前错误版本（第 21、46、73 行各一处）：**
```yaml
- uses: actions/checkout@v6
```

**修正后版本（3 处均改为）：**
```yaml
- uses: actions/checkout@v4
```

**保留不变的正确 action（第 23-27 行、第 48-53 行等）：**
```yaml
- name: Install Rust
  uses: dtolnay/rust-toolchain@stable

- name: Cache cargo
  uses: Swatinem/rust-cache@v2
```

**保留不变的正确 action（coverage job，第 82-85 行）：**
```yaml
- name: Install cargo-llvm-cov
  uses: taiki-e/install-action@v2
  with:
    tool: cargo-llvm-cov
```

**完整受影响行号位置：**
- 第 21 行：`test` job 的 checkout
- 第 46 行：`lint` job 的 checkout
- 第 73 行：`coverage` job 的 checkout

---

### `.github/workflows/release.yaml` (config, batch + event-driven)

**修改范围：重构**。现有 `release` job 的 matrix 定义、Rust 安装、cross 安装、缓存、构建、产物准备步骤全部保留，重构以下部分：

1. **删除** `release` job 中第 72-89 行的 `Extract changelog` + `Upload to GitHub Release` 步骤
2. **新增** `Upload artifact` 步骤替代原有的直接 release
3. **删除** 第 91-107 行的 `publish` job（D-07）
4. **新增** 独立的 `create-release` job

**原文件保留不变的结构（第 1-70 行核心）：**
```yaml
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
    # 注意：permissions: contents: write 从这里移除（build job 不再需要）
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
      - uses: actions/checkout@v4           # 修复：@v6 → @v4

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
```

**新增：替代原有 `Extract changelog` + `Upload to GitHub Release` 的步骤：**
```yaml
      - name: Upload artifact
        uses: actions/upload-artifact@v4      # D-04：暂存，不直接发布
        with:
          name: ${{ matrix.artifact }}        # 唯一名称（含平台标识，v4 要求）
          path: dist/${{ matrix.artifact }}
          retention-days: 1
```

**新增：`create-release` job（替代原 `publish` job 位置，整体新增）：**
```yaml
  create-release:
    name: Create GitHub Release
    needs: [release]                          # D-05：等待全部 4 个 build job 完成
    runs-on: ubuntu-latest
    permissions:
      contents: write                         # Pitfall 5：job 级别声明，不继承
    steps:
      - uses: actions/checkout@v4

      - name: Download all artifacts          # D-05
        uses: actions/download-artifact@v4
        with:
          path: dist
          pattern: sqllog2db-*               # 匹配所有 4 个平台产物
          merge-multiple: true               # 扁平化到 dist/（不创建子目录）

      - name: Extract changelog               # D-06：保留原 awk 逻辑，移入此 job
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
  # D-07：publish job 已删除（CARGO_REGISTRY_TOKEN 未配置）
```

**关键差异对比（原 vs 新）：**

| 原文件 | 修改后 |
|--------|--------|
| `actions/checkout@v6`（第 37 行） | `actions/checkout@v4` |
| `Extract changelog` 在每个 build job 中（第 72-81 行） | 移入 `create-release` job |
| `softprops/action-gh-release@v3` 在每个 build job 中（第 83-89 行） | 仅在 `create-release` job 中，使用 `softprops/action-gh-release@v2` |
| `permissions: contents: write` 在 `release` job（第 14-15 行） | 从 `release` job 移除，仅 `create-release` job 声明 |
| `publish` job（第 91-107 行）存在 | 删除 |

---

### `.github/workflows/bench.yml` (config, batch)

**修改范围：** 修复 2 处 action 版本。

**当前错误版本（第 20 行）：**
```yaml
- uses: actions/checkout@v6
```

**当前错误版本（第 37-42 行）：**
```yaml
- name: Upload benchmark artifact
  uses: actions/upload-artifact@v7
  with:
    name: bench-results-${{ github.sha }}
    path: bench-results-*.json
    retention-days: 60
```

**修正后版本：**
```yaml
- uses: actions/checkout@v4           # 第 20 行：@v6 → @v4
```

```yaml
- name: Upload benchmark artifact
  uses: actions/upload-artifact@v4    # @v7 → @v4
  with:
    name: bench-results-${{ github.sha }}
    path: bench-results-*.json
    retention-days: 60
```

**保留不变（第 17-18 行）：**
```yaml
    # Bench 失败不阻塞 PR merge（仅作 informational）
    continue-on-error: true
```

---

### `.github/workflows/lychee.yml` (config, request-response)

**修改范围：** 仅替换 1 处 checkout 版本。

**当前错误版本（第 27 行）：**
```yaml
- uses: actions/checkout@v6
```

**修正后版本：**
```yaml
- uses: actions/checkout@v4
```

**保留不变（第 29-55 行）：** `actions/cache@v5`、`lycheeverse/lychee-action@v2` 版本正确，无需改动。

---

### `.github/workflows/pages.yml` (config, event-driven)

**修改范围：** 仅替换 1 处 checkout 版本。

**当前错误版本（第 22 行）：**
```yaml
- uses: actions/checkout@v6
```

**修正后版本：**
```yaml
- uses: actions/checkout@v4
```

**保留不变（第 23-35 行）：** `peaceiris/actions-mdbook@v2`、`peaceiris/actions-gh-pages@v4` 版本正确，无需改动。

---

### `Cross.toml` (config, batch — 新建文件)

**无现有类比。** 参考 RESEARCH.md Pattern 4 和 Standard Stack 表格。

**完整文件内容：**
```toml
# cross-rs 跨编译配置
# 官方镜像：https://github.com/cross-rs/cross/pkgs/container/aarch64-unknown-linux-gnu
# edge = main 分支最新构建（latest = 0.2.5，3 年前，过旧）
[target.aarch64-unknown-linux-gnu]
image = "ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge"
```

**放置位置：** 项目根目录 `/Cross.toml`（与 `Cargo.toml` 同级）。

**兼容性说明：** 项目使用 `rusqlite = { features = ["bundled"] }`（见 `Cargo.toml` 第 49-51 行），SQLite 从源码编译，不依赖系统 libsqlite3，与 `edge` 镜像的 aarch64 cross-compilation toolchain 兼容，无需 `pre-build` 或自定义 Dockerfile。

---

## Shared Patterns

### checkout@v4 统一版本
**修改位置：** 全部 5 个 workflow 文件的 `actions/checkout` 步骤
**统一替换：**
```yaml
# 原文（所有文件均为）
- uses: actions/checkout@v6

# 修改后（所有文件统一）
- uses: actions/checkout@v4
```

**受影响位置汇总：**
- `ci.yaml`：第 21、46、73 行（共 3 处）
- `release.yaml`：第 37 行 + `create-release` job 新增（共 2 处）
- `bench.yml`：第 20 行（共 1 处）
- `lychee.yml`：第 27 行（共 1 处）
- `pages.yml`：第 22 行（共 1 处）

### 保留不变的正确 action 版本
**来源：** CONTEXT.md D-03
```yaml
# 以下 3 个 action 版本正确，所有 workflow 中无需改动
uses: dtolnay/rust-toolchain@stable
uses: Swatinem/rust-cache@v2
uses: taiki-e/install-action@v2
```

### release.yaml 的 awk changelog 提取
**来源：** `release.yaml` 第 73-80 行（当前实现），D-06 决策保留
**移动目标：** 从 `release` matrix job 移入 `create-release` job
```bash
VERSION=${GITHUB_REF#refs/tags/v}
awk "/## \[${VERSION}\]/,/## \[/" CHANGELOG.md | sed '$d' > release_notes.md
if [ ! -s release_notes.md ]; then
  echo "Release version ${VERSION}" > release_notes.md
  echo "" >> release_notes.md
  echo "See [CHANGELOG.md](https://github.com/${{ github.repository }}/blob/main/CHANGELOG.md) for details." >> release_notes.md
fi
```

### GITHUB_TOKEN 权限最小化
**来源：** RESEARCH.md Security Domain；Pitfall 5
```yaml
# build jobs：不声明 contents: write（读权限即可上传 artifact）
# create-release job 必须单独声明：
permissions:
  contents: write
```

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `Cross.toml` | config | batch | 项目首次引入 cross-rs 配置文件，无现有类比；直接使用 RESEARCH.md Pattern 4 模板 |

---

## 关键决策提醒（供 planner 参考）

| 决策 | 影响文件 | 具体操作 |
|------|----------|----------|
| D-01：@v6 → @v4 | ci.yaml(3处), release.yaml(1处), bench.yml(1处), lychee.yml(1处), pages.yml(1处) | 逐一替换，共 7 处 checkout |
| D-02：upload-artifact@v7 → @v4 | bench.yml(1处) | 替换第 37 行 |
| D-04：artifact 暂存方案 | release.yaml | build job 改为 upload-artifact，新增 create-release job |
| D-05：needs 依赖 | release.yaml | create-release job 声明 `needs: [release]` |
| D-06：保留 awk 逻辑 | release.yaml | awk 脚本从 build job 移入 create-release job，内容不变 |
| D-07：删除 publish job | release.yaml | 删除第 91-107 行整个 `publish:` job |
| D-08：新建 Cross.toml | Cross.toml | 根目录新建，内容为单一 target 配置 |

## Metadata

**Analog search scope:** `.github/workflows/` 目录（5 个 workflow 文件）
**Files scanned:** 7（5 workflow + Cargo.toml + cliff.toml）
**Pattern extraction date:** 2026-06-02
