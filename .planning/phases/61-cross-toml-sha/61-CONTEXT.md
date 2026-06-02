# Phase 61: Cross.toml SHA 固定 - Context

**Gathered:** 2026-06-03
**Status:** Ready for planning

<domain>
## Phase Boundary

将 `Cross.toml` 中 `aarch64-unknown-linux-gnu` 镜像的浮动 `:edge` 标签替换为固定 SHA digest 引用（`ghcr.io/cross-rs/aarch64-unknown-linux-gnu@sha256:<digest>`），并在文件中加注释记录该 SHA 对应的日期和更新方法。不修改任何 Rust 源文件，不影响本地编译。

</domain>

<decisions>
## Implementation Decisions

### SHA 获取方式

[auto] Q: "如何获取当前 edge 标签的 SHA digest？" → Selected: "docker manifest inspect" (recommended default)

- **D-01:** 使用 `docker manifest inspect ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge --verbose` 获取 digest（Cross.toml 中已有该命令的说明）。提取 `Digest` 字段作为 SHA。

### 注释格式

[auto] Q: "如何记录 SHA 对应的版本信息？" → Selected: "日期 + 更新命令注释" (recommended default)

- **D-02:** 在 `image` 行上方加注释，格式：
  ```toml
  # Pinned from :edge as of YYYY-MM-DD
  # To update: docker manifest inspect ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge --verbose | grep -i digest
  image = "ghcr.io/cross-rs/aarch64-unknown-linux-gnu@sha256:<digest>"
  ```
  保留原有的 TODO 说明注释（历史背景），替换 `image =` 行本身。

### Claude's Discretion

- 如果 `docker manifest inspect` 在 CI 环境不可用，可用 `skopeo inspect --raw docker://...` 或 GitHub Container Registry API 作为备选，但实现时以 docker 命令为主
- dry-run 验证：`cross build --target aarch64-unknown-linux-gnu --dry-run` 或检查 cross 是否支持 SHA 格式（如不支持 dry-run，文档化该限制）

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 61: Cross.toml SHA 固定" — Goal、Success Criteria（4 条）
- `.planning/REQUIREMENTS.md` §CROSS-01

### 关键文件
- `Cross.toml` — 唯一修改目标（项目根目录），已含 TODO 注释说明修复方式
- `Cargo.toml` — 确认无 cross-compilation 相关配置冲突（可选，仅参考）

### 外部资源
- `https://github.com/cross-rs/cross/pkgs/container/aarch64-unknown-linux-gnu` — cross-rs 镜像仓库，查看当前 digest

</canonical_refs>

<code_context>
## Existing Code Insights

### 当前 Cross.toml 状态
```toml
[target.aarch64-unknown-linux-gnu]
image = "ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge"
```
文件已含详细 TODO 注释，说明修复方式和风险。

### 约束
- CI/CD（`.github/workflows/`）可能引用 cross，修改后需确认构建不破坏
- SHA digest 格式：`@sha256:<64位hex>` — cross 工具支持此格式

</code_context>

<specifics>
## Specific Ideas

- Cross.toml 中保留原有注释（背景说明），仅替换 `image =` 那一行
- 成功标准 3 要求 SHA 注释包含镜像日期或版本——用获取日期的 `YYYY-MM-DD` 格式满足

</specifics>

<deferred>
## Deferred Ideas

- 换用非 cross-rs 镜像或自维护镜像 — 超出本阶段目标，REQUIREMENTS.md Out of Scope 已注明
- 引入自动更新 SHA 的 CI 步骤（Renovate/Dependabot for Docker）— 后续里程碑工程化方向

</deferred>

---

*Phase: 61-cross-toml-sha*
*Context gathered: 2026-06-03*
