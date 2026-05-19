# Pitfalls Research

**Domain:** 文档完善 & GitHub Pages 落地页建设（为已有成熟 CLI 工具追加文档）
**Researched:** 2026-05-18
**Confidence:** HIGH

## Critical Pitfalls

### Pitfall 1: 文档与代码脱节（Documentation Drift）

**What goes wrong:**
文档中的功能描述、配置示例、性能数据与当前代码行为不匹配。用户按照文档操作得到错误结果或报错，失去信任。最典型的是 README 中展示旧版扁平配置格式，而实际代码已在 v1.4 中改为嵌套格式。

**Why it happens:**
- 文档在功能实现时撰写，但后续重构未同步更新
- 对已有项目"追加文档"时，容易只关注新写的内容，忽略对旧有 README 的全面审查
- 团队（或个人）认为"文档可以后面再补"，结果"后面"永远不会来
- 回顾 sqllog2db：README 仍然展示 `[features.replace_parameters]` 扁平格式，完全不反映 v1.4 的 5 个顶层配置字段；模板分析（v1.3）和图表功能（v1.3）在 README 中完全缺失

**How to avoid:**
- 在文档撰写开始前，先对 README 做完整的 diff 审查：逐段对比实际 CLI 行为（`--help`、`init` 生成的默认配置、`show-config` 输出）
- 自动化验证：编写脚本从 `sqllog2db init` 生成真实配置，与文档中的示例做结构性对比
- 在 doc phase 中把"更新 README"列为独立的 checklist 项，与"创建缺失文档"并列
- 建立文档审查 checklist：运行 `cargo run -- --help` 逐条验证 README 中的 CLI 示例

**Warning signs:**
- README 中的 `init` 输出示例与 `cargo run -- init` 实际输出不一致
- `--help` 输出中的子命令在 README 中找不到对应章节
- README 引用的配置字段在 `src/config/` 中已删除（如旧 `[pipeline.*]` 字段）

**Phase to address:**
Phase 1（README 全面更新）— 作为 README 重建的一部分，必须逐段与代码对照验证

---

### Pitfall 2: 链接腐烂（Link Rot）

**What goes wrong:**
README 和文档中引用的内部链接指向不存在的文件，用户点击后看到 404 页面。在 sqllog2db 中，README 的"快速链接"区域引用了 5 个文件其中 4 个是断链：`./docs/quickstart.md`、`./docs/architecture.md`、`./CONTRIBUTING.md`、`./SECURITY.md`（仅有 `./CHANGELOG.md` 存在）。

**Why it happens:**
- 预先在 README 中写好链接占位，期望后续创建对应文件，但文件从未被创建
- 文件路径重构（如 `docs/ -> docs/guide/`）后未更新所有引用
- 文档托管到 GitHub Pages 后路径变化，但旧链接未迁移

**How to avoid:**
- 对 README 中所有内部链接做可操作性验证：`for link in $(extract_links README.md); do [ -f "$link" ] || echo "BROKEN: $link"; done`
- 只链接已存在的文件，不提前声明"计划中的文档"
- 建立 CI check：在 PR 中自动化检查 README 链接有效性（使用 `lychee` 或 `broken-link-checker`）
- 如果决定不创建某些文档（如 `SECURITY.md`），必须从 README 中移除对应链接，不能留空

**Warning signs:**
- README 中的 `./` 或 `./docs/` 链接序列，用 `ls` 检查后发现目标不存在
- GitHub 仓库的文件浏览中看不到 README 引用的文档文件

**Phase to address:**
Phase 1（README 更新）— 链接修复创建/删除同步；Phase 3（CI 集成）— 添加 lychee 自动化链接检查

---

### Pitfall 3: 落地页配置示例快速过时（Stale Config Examples）

**What goes wrong:**
GitHub Pages 落地页展示的配置示例（完整的 config.toml）在代码重构后变为错误示例。用户复制落地页的配置但程序报错——因为格式已经改变。这是 Pitfall 1 在落地页上的特化表现，更具破坏性，因为落地页通常是新用户的第一接触点。

**Why it happens:**
- 落地页的配置示例是硬编码的静态代码片段，不是从实际代码生成
- 配置模型重构时，开发者记得更新 README 和 init 模板，但忘记同步更新独立落地页
- 配置示例在落地页上写死（作为 Markdown/HTML 代码块），修改成本高于预期

**How to avoid:**
- 从源代码生成配置示例：提取 `sqllog2db init` 生成的默认配置（通过 `--stdout` 或文件），直接注入到落地页中
- 如果使用静态站点生成器（如 Zola、Hugo），把配置示例作为包含文件（include），避免硬编码
- 在 Phase 2 落地页 PR 的 checklist 中加入"配置示例与 `cargo run -- init` 输出一致"的验证步骤
- 最简单的规避：落地页**不展示完整配置示例**，只展示关键配置变化（或直接引用 README 中的完整配置）

**Warning signs:**
- 落地页中的配置示例包含已废弃的配置字段（如 `[pipeline.*]`）
- 落地页配置与 `cargo run -- init` 生成的模板格式不同

**Phase to address:**
Phase 2（GitHub Pages 落地页）— 落地页设计时必须解决配置示例的来源问题

---

### Pitfall 4: 落地页过度工程化（Over-engineering the Landing Page）

**What goes wrong:**
为一个 CLI 工具构建过于复杂的落地页：使用 Next.js/SvelteKit/React 等重型前端框架构建纯展示页面，引入 JS 运行时依赖，需要复杂构建管道，增加维护负担。结果：初始构建投入大、构建产物大、后续更新无人敢碰。

**Why it happens:**
- 开发者将"精美展示"等同为"用最新前端框架"
- 低估纯静态页面（HTML + CSS + 少量 JS）在表现力上的能力
- 认为"既然要做就做好"，选择自己更熟悉的框架而非最适合的工具
- 社区中存在 showcase 偏见：复杂的页面设计被认为"更专业"

**How to avoid:**
- 对于 CLI 工具的 GitHub Pages，最高性价比的选择：**纯 Markdown + GitHub Pages 原生 Jekyll**（零配置，自动部署）或 **Zola**（Rust 原生、单二进制、零 JS 运行时）
- 按以下优先级决策工具：GitHub Pages 默认 Jekyll（零成本，零学习） -> Zola（Rust 生态，零 JS） -> 简单 HTML/CSS -> 最不优先：JS 框架
- 设一个硬性原则："落地页必须能在没有 `npm install` 的情况下构建"
- 从 sqllog2db 的实际受众考虑：这是一个面向中文达梦 DBA 的工具，落地页核心目的是快速传达功能，不是视觉竞赛
- 如果决定用框架，先评估：Dependabot alerts、npm audit 修复、框架版本升级——这些维护成本是否值得一个 3 页的文档站点

**Warning signs:**
- 方案讨论中出现 `npm create`、`yarn add`、`next.config.js` 等术语
- 落地页功能列表出现"动画"、"交互式演示"等超出文档范畴的需求
- 构建工具需要 Node.js、Ruby 以外的运行时

**Phase to address:**
Phase 0（方案选择阶段）— 在 Phase 2 开始前先决策工具栈，避免实现后返工

---

### Pitfall 5: 多文档源的同步维护负担（Three-body Problem of Documentation）

**What goes wrong:**
项目同时维护 README（GitHub/根目录）、GitHub Pages 站点（独立部署）、crates.io 文档（自动拉取 README）三个内容源。每次功能更新需要在三个地方分别修改，导致：
- 开发者只更新了 README 但忘记更新落地页
- 落地页上的信息与 README 不一致（用户不知道该信哪个）
- 维护成本随内容源数量线性增长

**Why it happens:**
- README 被 crates.io 自动拉取（`readme = "README.md"`，同一个文件），但 GitHub Pages 是独立部署的——这是两个源，不是三个
- 本质问题不是"文件数量"而是"内容冗余"：落地页与 README 有大量重叠内容
- GitHub Pages 的独立存在鼓励了添加 README 中没有的额外内容，但又没有机制保证 README 同步更新

**How to avoid:**
- **最重要的策略：README 是单一事实来源（single source of truth）**。GitHub Pages 作为 README 的增强展示，不应包含 README 中不存在的内容
- GitHub Pages 不重复 README 的全部内容，聚焦于：项目 hero 展示（screenshot/graphic）、功能亮点、性能数据可视化、指向 README 的"详细文档"引导
- 详细的 CLI 用法、配置参考、开发指南保持在 README 中，落地页只做摘要和跳转
- 避免在落地页中维护独立的 FAQ、配置参考、Benchmark 数据——这些已经（或应该）在 README 中
- 在规划时明确界定：README 负责"完整参考"，落地页负责"首次印象 + 引流"

**Warning signs:**
- 落地页中包含与 README 冗余且格式不一致的"快速开始"章节
- 落地页中有独立的 Benchmark 表格，与 README 中的版本不同（数值有差异）
- 在落地页上发现"这个内容 README 里没有"的情况——这可能是一件好事，但要走单向同步流程

**Phase to address:**
Phase 0（文档架构决策）— 在规划阶段界定 README 与落地页的职责边界

---

### Pitfall 6: Cargo.toml 元数据缺失导致 crates.io 展示不全

**What goes wrong:**
crates.io 页面上没有"Documentation"按钮，用户从 crates.io 安装后找不到文档入口。sqllog2db 当前 `Cargo.toml` 中 `homepage` 指向 GitHub 仓库而非 GitHub Pages，且没有 `documentation` 字段。

**Why it happens:**
- `Cargo.toml` 元数据在项目初始化时设置，后续从未更新
- `documentation` 字段在 GitHub Pages 建立前不存在，建立后忘记添加
- 开发者认为 `homepage` + `repository` 已足够，忽略 `documentation` 的独立价值

**How to avoid:**
- 在 GitHub Pages 部署后立即更新 `Cargo.toml`：
  ```toml
  homepage = "https://guangl.github.io/sqllog2db"  # 改为 GitHub Pages URL
  documentation = "https://guangl.github.io/sqllog2db"
  ```
- 保留 `repository` 指向 GitHub 仓库不变
- 将此操作列入 Phase 2 的 deployment checklist（"部署后更新 Cargo.toml 元数据"）
- 注意：更新在下一次 `cargo publish` 时才生效——确保在下一次发版前做此变更

**Warning signs:**
- `cargo metadata` 输出的 `documentation` 字段为空或缺失
- crates.io 页面上的 crates.io 信息区域没有"Documentation"按钮

**Phase to address:**
Phase 2（GitHub Pages 落地页）— 部署后立即更新 Cargo.toml 元数据；在下一次发版前验证

---

### Pitfall 7: GitHub Pages CI 工作流的隐性陷阱

**What goes wrong:**
GitHub Pages 部署工作流配置不当导致部署失败、部署了空内容、或产物路径与 Pages 设置不匹配。

**Common specific failures:**
1. 使用错误的 `publish_dir` 导致部署空目录或错误分支
2. 未设置正确的 permissions，`GITHUB_TOKEN` 默认无 `contents: write` 和 `pages: write` 权限
3. 静态站点生成器的输出目录与 Pages 期望的根目录不匹配（如 Zola 输出到 `public/`，但 GitHub Pages action 期望其他路径）
4. 自定义域名（CNAME）配置在每次构建产物中被覆盖丢失，域名重置为 `<username>.github.io`
5. 仓库 Settings > Pages > Source 设置为 "GitHub Actions" 但 workflow 的 `upload-pages-artifact` + `deploy-pages` 组合使用不当
6. `.nojekyll` 文件缺失导致 Jekyll 处理非 Jekyll 站点时跳过 `_` 开头的文件和目录

**How to avoid:**
- 使用官方推荐的 `actions/upload-pages-artifact@v3` + `actions/deploy-pages@v4` 组合（而非第三方 action），文档完善、权限清晰
- 在仓库 Settings > Pages > Source 中确认设置为 "GitHub Actions"
- 添加显式 `permissions` 块：
  ```yaml
  permissions:
    contents: read
    pages: write
    id-token: write
  ```
- 如果使用自定义域名，在 SSG 的静态资源目录中包含 `CNAME` 文件（如 Zola 的 `static/CNAME`），确保每次部署都保留
- 对于非 Jekyll 的 SSG（如 Zola、Hugo），确保根目录有 `.nojekyll` 文件，或通过 `actions/upload-pages-artifact` 的配置包含它
- 在 PR 中预演部署（使用 `pull_request` 触发 + link to artifact）来验证配置正确性

**Warning signs:**
- 部署后页面空白或 HTTP 404（通常是 `publish_dir` 路径错误）
- 部署后自定义域名不生效（CNAME 文件被覆盖）
- Actions 日志显示权限拒绝但无明确 error message
- 页面显示为目录列表而非 HTML 渲染（缺少 `index.html` 或 `publish_dir` 指向了错误路径）

**Phase to address:**
Phase 2（GitHub Pages 落地页）— 不要依赖记忆配置，使用经过验证的 action 模板；先在一个测试仓库或 fork 上验证工作流

---

### Pitfall 8: 不预设文档维护流程——"一次写好，永不更新"

**What goes wrong:**
文档和落地页被视为"一次性交付物"。里程碑完成后，后续代码变更不再同步更新文档。三个月后文档全面过时，需要重建。

**Why it happens:**
- 文档更新没有被纳入后续开发工作流（PR checklist 中没有"更新文档"项）
- 认为"文档更新很简单，随时都能做"——但永远排不到优先级
- 项目缺少"文档质量"的度量指标（没有类似 test coverage 的"docs freshness"检查）

**How to avoid:**
- 在 v1.5 完成后，建立一个文档维护流程（写入 CONTRIBUTING.md 和项目约定）：
  - 每个 PR 必须评估是否需要更新相关文档（README、docs/、落地页对应的片段）
  - 配置变更 PR 必须更新默认配置模板 + README 配置示例
  - 新增功能必须在合并前或合并后立即更新文档
- 在 CI 中添加文档检查：
  - `lychee`（或 `broken-link-checker`）检查所有内部/外部链接可访问性
  - `cargo doc --no-deps -D warnings`（CI 已有）确保 API 文档不 broken
- 在 README 中添加"最后更新"标记（如 `*Last updated: 2026-05-18*`），方便读者判断时效性
- 考虑每季度或每次 release 前运行一次文档审计

**Warning signs:**
- 新功能 PR 不包含任何文档变更
- README 中的"最后更新"标记与实际代码变更间隔超过 3 个月
- 配置变更后 CI 通过但 README 中的配置示例未同步更新

**Phase to address:**
Phase 3（CI 与维护流程）— 将文档维护流程制度化；落地页上线后的第一个 feature phase 就会考验这个流程

---

### Pitfall 9: 忽略 crates.io 自动拉取 README 的兼容性

**What goes wrong:**
修改 README 时使用了 GitHub Pages 独有的资源（相对路径图片、内嵌 SVG 等），导致 crates.io 上的 README 渲染异常——图片显示为断链、格式错乱。

**Why it happens:**
- crates.io 从 GitHub raw 源拉取 README（`readme = "README.md"`），不执行任何 Jekyll 或 SSG 处理
- README 中使用的 `![Chart](./docs/chart.svg)` 在 GitHub 仓库中正常，但 crates.io 的 raw 环境无法解析相对路径
- GitHub Pages 的专属样式或语法（如 Jekyll frontmatter、Liquid tag）会破坏 crates.io 渲染

**How to avoid:**
- 保持 README 的纯 Markdown 兼容性：不使用任何非标准 Markdown 扩展
- 图片使用绝对 URL（`https://raw.githubusercontent.com/guangl/sqllog2db/main/docs/chart.svg`）而非相对路径
- 不在 README 中使用 Jekyll frontmatter、Liquid tag 等 GitHub Pages 专属语法
- 在部署前用 `cargo publish --dry-run` 检查 README 的预期渲染效果
- 简单的守则："README 必须能在 GitHub 仓库页面和 crates.io 上都能正确渲染"

**Warning signs:**
- README 中包含 `---` 开头的 YAML frontmatter
- README 使用 `{% %}` 或 `{{ }}` 模板语法
- 图片使用相对路径而非 raw 完整 URL

**Phase to address:**
Phase 1（README 更新）— 图片链接格式在修改 README 时同步修正为绝对 URL

---

### Pitfall 10: 缺少 API 文档集成（`cargo doc` 与落地页分离）

**What goes wrong:**
项目的 API 文档（通过 `cargo doc` 生成）完全独立于落地页。落地页展示用户文档和 CLI 用法，`cargo doc` 展示内部 Rust API——两者各自为政，没有相互引导。贡献者不知道如何查阅 API 文档。

**Why it happens:**
- `cargo doc` 是 Rust 项目的标准 API 文档生成工具，天然面向库使用者/贡献者
- CLI 工具的项目文档（README、落地页）面向终端用户，与 API 文档的受众不同
- 两者没有自动关联机制，`cargo doc` 的输出也不在 GitHub Pages 部署范围内

**How to avoid:**
- 不必将 `cargo doc` 纳入落地页部署（会增加复杂度），但应在落地页和 CONTRIBUTING.md 中显式引导
- 最简方案：在落地页添加"API Reference"链接指向 `https://docs.rs/dm-database-sqllog2db/latest/`（docs.rs 自动为 crates.io 包构建 API 文档，零维护）
- 或者在落地页部署时同时发布 API 文档：`cargo doc --no-deps` 的输出与 SSG 产物一起发布到 GitHub Pages 的 `/api/` 子路径

**Warning signs:**
- CONTRIBUTING.md 中没有任何关于如何查阅 API 文档的指引
- 落地页没有指向 docs.rs 或 API 文档的链接
- 项目有外部贡献者但无法找到内部模块文档

**Phase to address:**
Phase 2（落地页）— 添加 API 文档引导链接（docs.rs）；Phase 3（CI）— 可选项：将 API 文档纳入 Pages 部署

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| 落地页用纯 HTML/CSS 手写 | 零框架依赖，快速上线 | 难以维护和扩展，新内容需手写 HTML | 落地页内容极少（单页 3 节以下）。sqllog2db 功能丰富，不推荐 |
| 落地页复制 README 内容 | 快速填充页面，外观丰富 | 两份内容难同步，永远存在差异 | **从不接受**——违反单一事实来源原则 |
| 先部署落地页，后续再更新 Cargo.toml 元数据 | 减少初期工作 | 用户从 crates.io 找不到文档入口 | 可接受，但必须在同一个 milestone 内的下一 phase 立即修复，不能跨 milestone |
| 只更新落地页不更新 README | 快速响应更急迫的需求 | 三体问题恶化，用户困惑 | **从不接受** |
| 无 CI 链接检查 | 节省 CI 资源 | 链接腐烂不被发现直到用户报告 | 仅作为 Phase 1-2 的临时状态，Phase 3 必须补上并设为 PR 检查项 |
| 忽略 `cargo doc` 部署 | 简化部署流程 | 贡献者不知道如何浏览 API 文档 | 有 docs.rs 链接时可接受，但落地页必须引导至此 |
| 同一个 README 维护中文和英文版本 | 双语覆盖 | 维护成本翻倍，容易一个更新另一个不更新 | 目标受众单一语言时不做双语。sqllog2db 以中文用户为主，不做英文版 |
| 落地页跳过 `.nojekyll` 文件（使用 Jekyll 处理） | 少了解一个配置 | Jekyll 自动处理 `_` 开头文件，可能导致静态资源丢失 | 使用默认 Jekyll 没问题；使用其他 SSG 必须添加 `.nojekyll` |

## Integration Gotchas

Common mistakes when connecting to external services.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| **GitHub Actions + pages deploy** | 使用 `JamesIves/github-pages-deploy-action@v4` 但不设置 `branch` 参数，默认推送到 `gh-pages` 但仓库设置不是这个分支 | 先确认仓库 Settings > Pages > Source 设置；推荐用官方 `actions/upload-pages-artifact@v3` + `actions/deploy-pages@v4` 组合 |
| **CNAME 文件持久化** | 只在首次部署时添加 CNAME，后续部署覆盖丢失 | 将 CNAME 文件放在 SSG 的静态资源目录（如 Zola 的 `static/` 或 Jekyll 的根目录），确保每次部署都包含 |
| **Custom domain + HTTPS** | 添加自定义域名后立即测试，HTTPS 证书未签发显示不安全 | GitHub 自动签发 HTTPS 证书（通过 Let's Encrypt），但需要 5-30 分钟；使用 CNAME 记录指向 `<username>.github.io` |
| **lychee link checker** | 配置过于严格，对外部链接的频繁检查导致 CI 经常性失败 | 使用 `--exclude` 排除已知不可达的外部域名（如 `crates.io/api`、`github.com` API 端点）；设置为 `continue-on-error: true` 避免阻塞 PR |
| **crates.io + README 图片** | 使用相对路径引用截图/图表 | 使用 `https://raw.githubusercontent.com/...` 绝对 URL，确保在 crates.io 上也能正确渲染 |
| **`.nojekyll` 缺失** | 非 Jekyll 的静态站点部署后 `_` 开头的文件和目录被忽略 | 在 SSG 输出目录的根目录添加一个空文件 `.nojekyll`；或通过 `upload-pages-artifact` 的配置包含它 |

## Performance Traps

Traps that affect documentation maintenance efficiency, not runtime performance.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| **落地页包含大量外部资源**（Google Fonts、CDN 字体、复杂 JS 库） | 中国用户（项目主要受众）加载极慢或无法加载 | 使用自托管资源或无外部依赖；考虑中国用户的网络可达性 | 立即：某些 CDN 在中国可能被限制或延迟严重 |
| **落地页使用 JS 框架做 SSR** | 每次内容更新需要 npm install、npm audit fix、构建 | 使用 Zola/Jekyll 或纯 HTML——零 JS 运行时 | 第一次 `npm audit --fix` 引入 breaking change 或 Dependabot 持续提醒时 |
| **版本历史放在 Landing Page 上** | 每次发版需要更新落地页 + CHANGELOG | 落地页只显示最新版本信息和"查看更多"链接到 CHANGELOG | 第 2 个版本发布后维护成本翻倍 |
| **README 中的性能表格手动维护** | 基准测试更新后忘记更新 README | 自动化：`cargo bench` 后将结果注入 README（通过生成脚本或 CI include） | 每次上游 crate 升级或平台变化后的第一次跑 bench |
| **多个 feature 的配置示例全在 Landing Page 展示** | 新增 feature 后 Landing Page 配置示例立即过时 | Landing Page 只展示最简配置，完整参考指向 README | 每个新功能发布后 |
| **SSG 构建时间过长** | CI 部署耗时从 1min 增加到 5min+ | 内容型站点（无 JS/图片处理）的构建应在 <10s | 当引入图片压缩、Sass 编译、JS 打包等步骤时 |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| **在落地页配置示例中包含敏感占位符路径**（如 `/var/run/secret`、`password = "secret"`） | 用户可能直接复制配置而忘记替换敏感占位符 | 使用明确的安全占位符：`path = "/path/to/your/logs"`，不在示例中展示任何像实际凭证的占位 |
| **落地页使用 CDN 加载 JS 库**（如 highlight.js 代码高亮） | CDN 被攻陷替换为恶意脚本；中国用户加载失败影响使用 | 使用服务端代码高亮（SSG 内置插件）或自托管。若必须用 CDN，使用 SRI（Subresource Integrity） |
| **落地页暴露内部项目信息**（如 CI 内部 URL、服务器路径、未公开的配置文件） | 信息泄露，社会工程攻击面 | Pages 部署前做一次信息扩散（SII）检查：搜索 landing page 内容中的内网 IP、内部域名、密钥 |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| **落地页全中文但代码注释和报错信息全英文** | 中文用户读注释吃力但不致命 | 保持一致：sqllog2db 现有 init 模板已是中英双语（`# 中文注释` + `# English comment`），是好的模式，继续沿用 |
| **GIF 演示 CLI 操作** | 中国用户可能因 GitHub 间歇不可达看不到 GIF；GIF 体积大、加载慢 | 使用静态截图 + 文字说明替代 GIF。如果坚持用动态演示，使用托管在 GitHub 仓库中的视频文件或链接到 YouTube/Bilibili |
| **落地页首屏堆满功能列表** | 用户无法快速判断"这个工具能解决我的问题吗" | 首屏定位：一句话描述 + screenshot/示意图 + "快速开始"按钮。功能列表放在 second fold |
| **"快速开始"不是真正的快速开始** | README 的"快速开始"包含 init -> validate -> run -> stats -> digest 5 个步骤，新用户被信息淹没 | 真正的快速开始：`cargo install` -> 生成默认配置 -> 直接运行（展示默认行为）。细节步骤放在"高级用法" |
| **落地页的"快速开始"示例未经实测** | 展示的命令在实际环境中执行失败（如路径错误、缺少权限） | 所有在落地页展示的 CLI 命令必须从头到尾在真实环境跑通一遍，贴实际输出而不是手写 |

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **README 更新：** 只更新了新增功能部分，忘记审查和更新旧内容（配置示例、性能数据、FAQ 答案是否仍准确？）
- [ ] **CHANGELOG.md：** 文件存在但版本记录不完整（当前 CHANGELOG 只从 v0.10.7 开始，缺少 v1.x 版本的完整记录）
- [ ] **docs/ 中的截图/图标：** 文件中引用了图片但图片还未生成或上传（placeholder 问题）
- [ ] **落地页的"快速开始"示例：** 贴出的命令未在真实环境中运行验证过（应从头到尾跑一遍，贴实际输出）
- [ ] **内部链接验证：** README 中的所有 `./` 相对链接都手动用 `[ -f ... ]` 检查过可访问性
- [ ] **Cargo.toml documentation 字段：** GitHub Pages 已上线但 Cargo.toml 中的 `documentation` 字段还未更新指向 Pages URL
- [ ] **crates.io 版本：** `documentation` 字段在下一次 `cargo publish` 时才生效——确保在下一次发版时已设置
- [ ] **CI 文档检查：** 添加了 `lychee` 或类似工具但配置了 `--fail`，导致链接检查失败直接阻塞 PR（应根据场景决定是否设为 blocker）
- [ ] **`.nojekyll` 文件：** 如果使用非 Jekyll SSG，确保产物根目录包含此文件
- [ ] **`CNAME` 文件：** 如果使用自定义域名，确保在 SSG 的静态资源目录中包含此文件
- [ ] **落地页的 404 页面：** GitHub Pages 默认有 404 页面，但可能没被自定义

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| README 配置示例过期 | **LOW**（编辑 README 单文件） | 1. 运行 `cargo run -- init` 获取当前默认配置；2. 用最新输出替换 README 中的配置示例块；3. 检查是否有新增/删除的配置字段在 README 其他部分引用 |
| 落地页链接腐烂 | **MEDIUM**（需要逐个修复） | 1. 运行 `lychee . --no-progress --exclude github.com` 检查全站链接；2. 批量替换无效外部链接；3. 修复后运行 `lychee` 验证 |
| landing page 概念过期 | **HIGH**（需要大面积重写） | 1. 在问题发生前防止：用 SSG 的 include/partial 机制减少重复内容；2. 如果已过期，执行"差异审计"：逐段对比 README -> Landing page，标记每个差异点的优先级 |
| 多文档源不一致（三份各不同） | **MEDIUM**（需要梳理差异） | 1. 锁定 README 为 source of truth；2. 合并所有差异到 README；3. 从 Landing page 删除所有与 README 冗余的内容，改为引用；4. 建立"只有 README -> 落地页"的单向同步流程 |
| `cargo publish` 后 crates.io 文档链接错误 | **LOW**（下一个版本修复） | 1. 当前版本无法回退 crates.io 信息；2. 在下一次 `cargo publish` 前修复 Cargo.toml；3. 在 README 中添加显式的 docs 链接作为临时替代（README 文件本身 crates.io 会显示） |
| CI 中 lychee 链接检查频繁失败 | **LOW**（配置调整） | 1. 检查失败的外部域名是否稳定可达；2. 将不稳定域名加入 `--exclude`；3. 考虑设为 `continue-on-error: true` |
| GitHub Pages 部署后 404 | **HIGH**（需要立即修复） | 1. 检查 GitHub Actions 日志确认 `publish_dir` 是否正确；2. 确认 index.html 存在；3. 检查仓库 Settings > Pages Source 是否设为 "GitHub Actions"；4. 检查 `.nojekyll` 和 `CNAME` 文件 |

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| 文档与代码脱节（P1） | Phase 1（README 更新） | 逐段比对：`cargo run -- --help` vs README 命令示例；`cargo run -- init` vs 配置示例 |
| 链接腐烂（P2） | Phase 1（修复/创建文档）+ Phase 3（CI） | Phase 1 手动验证；Phase 3 lychee CI check 自动拦截 |
| 落地页配置示例过时（P3） | Phase 0 + Phase 2 | 落地页配置示例与 `cargo run -- init` 输出一致；或落地页不展示完整配置只做引导 |
| 落地页过度工程化（P4） | Phase 0（工具栈决策） | 构建工具 <= 单个 Rust 二进制（Zola）或零额外依赖（Jekyll/纯 HTML） |
| 多文档源同步负担（P5） | Phase 0（职责界定） | README 是事实源，落地页内容为 README 子集 + 视觉增强，不引入全新内容 |
| Cargo.toml 元数据缺失（P6） | Phase 2（部署后立即） | `cargo metadata` 输出中 `documentation` 字段指向 Pages URL |
| GitHub Pages CI 陷阱（P7） | Phase 2（部署前测试） | 使用官方 actions + 在 PR artifact 中预演；检查 `.nojekyll` 和 `CNAME` |
| 文档维护流程缺失（P8） | Phase 3（CI 与流程制度） | CONTRIBUTING.md 包含"文档更新"PR checklist；CI 中有链接检查（可不设 blocker） |
| crates.io README 兼容性（P9） | Phase 1（链接格式修正） | `cargo publish --dry-run` 检查 README 渲染结果；图片用 raw URL |
| API 文档集成缺失（P10） | Phase 2（落地页链接） | 落地页有指向 docs.rs 或 API 文档的引导链接 |

## Sources

- [sqllog2db README.md](https://github.com/guangl/sqllog2db/blob/main/README.md) — 直接文件读取确认：旧配置格式、断链、v1.3 v1.4 功能缺失
- [sqllog2db CHANGELOG.md](https://github.com/guangl/sqllog2db/blob/main/CHANGELOG.md) — 版本记录不完整，缺少 v1.0-v1.4 的完整记录
- [sqllog2db Cargo.toml](https://github.com/guangl/sqllog2db/blob/main/Cargo.toml) — 缺少 `documentation` 字段；`homepage` 指向 GitHub 仓库
- [sqllog2db .github/workflows/ci.yaml](/.github/workflows/ci.yaml) — 已有 `cargo doc --no-deps -D warnings` 但无 lychee 或链接检查
- [sqllog2db .planning/RETROSPECTIVE.md](/.planning/RETROSPECTIVE.md) — v1.3 retro 记录了"文档债"问题；ROADMAP 进度表陈旧的反复模式
- [peaceiris/actions-gh-pages](https://github.com/peaceiris/actions-gh-pages) — 成熟的 GitHub Pages 部署 action
- [lycheeverse/lychee](https://github.com/lycheeverse/lychee) — Rust 生态链接检查工具
- [Zola SSG](https://www.getzola.org/) — Rust 原生静态站点生成器，单二进制
- [GitHub Pages 官方文档 - 自定义域名和 HTTPS](https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site) — CNAME + HTTPS 配置
- [GitHub Pages 官方文档 - 使用 actions 部署](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site#publishing-with-a-custom-github-actions-workflow) — 官方推荐部署方式
- [actions/upload-pages-artifact](https://github.com/actions/upload-pages-artifact) — 官方 Pages 上传 action
- [actions/deploy-pages](https://github.com/actions/deploy-pages) — 官方 Pages 部署 action
- 个人经验：多个 Rust 开源项目（clap、serde 生态）的文档维护实践

---
*Pitfalls research for: sqllog2db v1.5 文档完善 & GitHub Pages 落地页*
*Researched: 2026-05-18*
