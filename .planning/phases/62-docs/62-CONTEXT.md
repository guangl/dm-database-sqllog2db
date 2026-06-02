# Phase 62: 文档完善 - Context

**Gathered:** 2026-06-03
**Status:** Ready for planning

<domain>
## Phase Boundary

三项文档任务：(1) README.md 补充 `stats` 子命令用法示例（含 `--from`/`--to`）和 v1.15 CI/CD 修复说明；(2) 新建 CHANGELOG.md，采用 Keep a Changelog 格式，覆盖 v1.0 至 v1.15；(3) config.toml init 模板补全过滤器各子字段的行内注释。不修改任何 Rust 源文件。

</domain>

<decisions>
## Implementation Decisions

### CHANGELOG 生成方式

[auto] Q: "如何生成 CHANGELOG 内容？" → Selected: "git cliff 生成基础 + 手动补历史" (recommended default)

- **D-01:** 使用 `git cliff` 生成 v1.13 至 v1.15 的 CHANGELOG 内容（`cliff.toml` 已配置 Keep a Changelog 格式）。v1.0 至 v1.12 的历史版本通过 `git log --oneline` 按版本 tag 分段，手动整理 Added/Changed/Fixed 条目。
- **D-02:** CHANGELOG.md 文件头使用 `cliff.toml` 中定义的 header（已包含 Keep a Changelog 链接说明）。

### README 结构

[auto] Q: "README 如何更新？" → Selected: "保持现有结构，追加新功能段落" (recommended default)

- **D-03:** README 不重写，在"功能特性"章节中新增 `stats` 子命令的用法示例（`sqllog2db stats -c config.toml --from 2024-01-01 --to 2024-01-31`），以及在"配置与性能"或独立"变更说明"段落中补充 v1.15 CI/CD 修复内容（aarch64 cross-build、SHA 固定上下文）。

### config.toml 行内注释补全

[auto] Q: "如何补全 filter 子字段行内注释？" → Selected: "行内注释紧跟注释掉的字段" (recommended default)

- **D-04:** 对 `config.toml` init 模板中注释掉的过滤器字段（`filter.include`、`filter.exclude` 下的 `users`、`ips`、`sessions`、`threads`、`statements`、`apps`、`tags`），在同行添加 `# <字段描述>` 注释，格式与 `[stats]` 节已有注释一致（即 `# field = val  # Description`）。
- **D-05:** 修改 `src/cli/init.rs` 中的模板字符串（生成 config 的源头），而非直接修改 `config.toml`——确保 `sqllog2db init` 生成的文件包含新注释。

### Claude's Discretion

- CHANGELOG 版本范围以 git tag 为准；如某版本无 tag，用最近 commit 日期估算
- v1.0–v1.12 历史条目的详尽程度：主要功能变更和 breaking change 必须记录，patch 修复可按需合并
- README 中 `stats` 示例的位置：插入"功能特性 → 配置与性能"章节末尾，不破坏现有结构

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 62: 文档完善" — Goal、Success Criteria（4 条）
- `.planning/REQUIREMENTS.md` §DOC-01、DOC-02、DOC-03

### 关键文件
- `README.md` — 主要修改目标，当前已有完整结构（功能特性、架构、性能数据）
- `cliff.toml` — git-cliff 配置，定义 CHANGELOG 格式（Keep a Changelog）
- `src/cli/init.rs` — config.toml 模板生成的源文件（修改这里才能影响 `sqllog2db init` 输出）
- `config.toml` — 参考当前模板输出（已有 stats 注释，但 filter 子字段无行内注释）

### git tag 参考
- `git tag --sort=version:refname` — 获取版本历史，用于 CHANGELOG 分版本整理

</canonical_refs>

<code_context>
## Existing Code Insights

### config.toml 当前状态（init 生成）
- `[stats]` 节：`from`/`to`/`top` 字段已有行内注释（满足成功标准）
- `[filter.include]` / `[filter.exclude]` 字段：仅注释掉，无行内描述（缺口所在）
- `[filter.indicators]` / `[filter.sql]` 字段：同上

### README 当前状态
- 已有 `stats` 命令在功能列表中（"简洁的 CLI：init / validate / run / stats"）
- 缺少 `stats --from/--to` 的完整用法示例
- 无 v1.15 CI/CD 修复的专项说明

### CHANGELOG 状态
- `CHANGELOG.md` 不存在（新建任务）
- `cliff.toml` 存在且配置完整

### 测试约束
- Phase 47 新增了关于 init 模板注释存在性的断言——修改模板后必须确保这些测试仍通过
- 使用 `cargo test` 验证

</code_context>

<specifics>
## Specific Ideas

- 生成 CHANGELOG 初稿命令：`git cliff --output CHANGELOG.md`（v1.13+ 部分自动化），然后手动补历史
- init 模板在代码中以字符串字面量形式存在（`src/cli/init.rs`），需定位并修改对应字符串

</specifics>

<deferred>
## Deferred Ideas

- 新建 CONTRIBUTING.md 或 API 文档 — 超出本阶段范围
- 将 README 翻译为英文 — 超出范围

</deferred>

---

*Phase: 62-docs*
*Context gathered: 2026-06-03*
