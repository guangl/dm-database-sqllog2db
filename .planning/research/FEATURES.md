# Feature Research — Rust CLI 文档 & GitHub Pages 落地页

**Domain:** 文档体系与项目展示 — Rust CLI 工具的 README、文档站、GitHub Pages 落地页
**Researched:** 2026-05-18
**Milestone:** v1.5 文档完善 & 项目展示
**Confidence:** HIGH (阅读了 ripgrep, fd, bat, hyperfine, zoxide, ruff, bottom, eza, xh, gping 等 Rust CLI 的 README/文档站实践)

---

## 文档特征全景

本文件分析的是 sqllog2db **项目文档本身** 应该具备哪些特征，而非产品功能。

---

## Table Stakes（用户期望这些）

缺少这些 = 项目感觉不成熟。Rust CLI 用户来自开源社区，他们对文档质量有明确预期。

| 特征 | 用户期望 | 复杂度 | 说明 |
|---------|--------------|------------|-------|
| **README 头部标识** — 项目名、一句话描述、状态徽章 | 用户扫一眼就知道"这是做什么的、是否活跃" | LOW | 必备徽章：CI status, crates.io version, license。徽章数量控制在 4-6 个，避免徽章墙 |
| **安装说明** — 支持至少 cargo install + 各平台包管理器 | 用户最关心"怎么装"。单行命令即可安装是 CLI 工具的核心优势 | LOW-Med | ripgrep/fd/bat 均覆盖 15+ 平台。sqllog2db 当前仅需 `cargo install`，可在 GitHub Pages 上补充各平台二进制下载（GitHub Releases） |
| **QuickStart/快速上手** — 3-5 个命令行示例覆盖核心功能 | 用户安装后第一件事是尝试。示例需要可复制粘贴直接运行 | LOW | fd 的 README 每个 flag 都附带具体命令和输出。sqllog2db 需要展示：`run`, `digest`, `stats`, `validate` 四个子命令的典型用法 |
| **完整 CLI 选项参考** — `--help` 输出或以结构化表格列出 | 用户在决定下载前就想了解功能范围。README 内包含 help 输出降低尝试门槛 | LOW | hyperfine 和 gping 直接在 README 里包含 `--help` 原样输出。sqllog2db 的 `--help` 较短，适合内嵌 |
| **CHANGELOG.md** | 用户升级前必看。尤其 0.x/1.x 阶段，breaking changes 需要明确标注 | LOW | Keep a Changelog 格式（[keepachangelog.com](https://keepachangelog.com/)）。每个版本标注 Added/Changed/Deprecated/Removed/Fixed/Security |
| **LICENSE 文件** | 企业用户/包管理器必须确认许可证。缺失 = 被自动过滤 | LOW | MIT 或 Apache-2.0。需要文件存在于仓库根目录 |
| **从源码构建说明** | 贡献者和高级用户需要。Rust 社区习惯 `cargo build --release` | LOW | 一行命令。但需要注明 MSRV（最低 Rust 版本）|
| **错误报告 / 功能请求指引** | 用户遇到问题时需要一个清晰的反馈渠道 | LOW | ISSUE_TEMPLATE 或 CONTRIBUTING.md 中说明 |
| **安全策略（SECURITY.md）** | 企业安全团队在审查依赖时会检查。GitHub 自动检测此文件并显示在仓库页 | LOW | 标准模板，说明漏洞报告联系方式和响应时间预期 |

---

## Differentiators（竞争性优势）

这些特征使项目文档从"可用的"变为"令人信服的"。Rust 生态的优秀 CLI 工具文档在这些方面做得突出。

| 特征 | 价值主张 | 复杂度 | 说明 |
|---------|-------------------|------------|-------|
| **性能基准（Benchmark）展示** | sqllog2db 的核心竞争力之一。在 README 或 Pages 中以可视化方式展示 1.55M records/sec 吞吐量和内存常量特性，是让用户立刻相信的工具的证据 | MED-HIGH | 参考 ripgrep 的做法：含具体测试语料、命令、时间和归一化比率的表格。sqllog2db 需要：1) 合成 CSV 基准 5.2M/s；2) 真实 1.1GB 文件基准 1.55M/s；3) 内存占用曲线（证明流式处理不会随文件增大而增长）|
| **图表库 SVG 输出展示（Gallery）** | sqllog2db 独特卖点：能生成四类 SVG 图表。在文档中直接嵌入真实生成的 SVG 图片，既是特征展示也是实际输出预览 | MED | 在 README 和 Pages 上嵌入实际生成的 SVG 缩略图，链接到全尺寸。四类图表示例：频率柱状图、延迟直方图、趋势折线图、用户饼图 |
| **架构 / 数据流图** | sqllog2db 的流式处理 + Pipeline 过滤器 + 双路输出架构值得用一张图解释。用户看了数据流图就理解"这个工具怎么工作的" | MED | mdBook 或 mermaid.js 生成架构图。展示：日志文件 → Parser → Pipeline (Filters + Template Analysis) → ExporterManager → CSV/SQLite。标注 p95/p99 位置和热路径 |
| **配置模型参考文档（Annotation-Driven）** | sqllog2db 的 TOML 配置支持嵌套子表、serde alias 向后兼容、validate+compile 统一验证。需要一份完整配置参考，每行都有注释说明 | MED | 参考 bat 的 README 配置示例方式：完整的 config.toml 示例，每行都有注释说明作用、默认值、可选/必须。需要覆盖：filter / template / charts / output / replace_parameters 五个配置块 |
| **Real-World Workflow Walkthrough** | 不是单纯列出命令，而是提供一个"端到端真实场景"：从配置初始化 → 解析并过滤 → 模板分析 → 导出 CSV → 生成图表 → 解读结果 | MED-HIGH | 参考 fd README 的问题-解决模式。sqllog2db 的场景："作为 DBA，我想找到上周执行最慢的 10 个 SQL 模板并导出成报告"——展示从 init 到 digest 的完整流程 |
| **Asciicast 交互式 Demo** | 用户可以在终端里观看 sqllog2db 的运行过程（包含彩色输出、进度条）。asciinema.org 录制后嵌入 README | LOW | 录制约 30 秒的 demo，展示：`sqllog2db run` 的实时输出流。asciicast 自动播放按钮让用户无需安装即可感受工具效果 |
| **双通道文档（中文 + 英文）** | sqllog2db 面向达梦数据库用户（中国 DBA 为主），但中文互联网上高质量的 Rust 工具文档较少。中英双语文档覆盖两个受众群体 | MED | 英文版为主要文档，中文版（`README.zh-CN.md` 或独立 Pages）作为第二通道。zh-CN 翻译可以简化部分内容，聚焦中国 DBA 最关心的功能（过滤、模板分析、SVG 图表）|
| **FAQ / Troubleshooting 板块** | 预判用户常见问题：为什么找不到日志文件？如何设置过滤规则？达梦日志格式是什么样的？如何处理超大文件？| LOW-MED | 参考 bat 的 TroubleShooting 板块。提前回答可以减少 GitHub Issues 中的重复提问 |
| **贡献指南（CONTRIBUTING.md）** | 开源项目的"招募帖"。说明代码风格（函数 ≤ 40 行）、测试要求（cargo clippy 零警告）、PR 流程 | LOW | 包含：环境搭建、编码规约、测试策略、PR 模板 |
| **Playground / Web Demo** | 用户无需安装即可在浏览器中体验 sqllog2db。WebAssembly 版可以演示解析速度 | HIGH | 复杂度高（WASM 编译 + 前端交互）。可作为 v1.5+ 后续功能考虑 |
| **Star / Download Count 展示** | 社会化证据。README 头部徽章区域包含 GitHub Stars 和 crates.io 下载量 | LOW | shields.io 动态徽章，展示项目受欢迎程度 |

---

## Anti-Features（常见陷阱，但不推荐做）

| Anti-Feature | 为什么有人想要 | 为什么不推荐 | 更好的做法 |
|--------------|---------------|-----------------|-------------|
| **mdBook 完整文档站** | 看起来很正式，有导航栏、搜索功能 | 对于单工具 CLI 项目，维护 mdBook 成本高。mdBook 需要额外的构建步骤和 GitHub Actions CI，且搜索功能需要 JavaScript，Pages 用户可能关闭 JS | 使用 GitHub Pages + 纯静态 HTML（Jekyll 或手写），或者更好的方案：README + 补充文档（docs/ 目录）+ GitHub Pages 落地页。不需要全套 mdBook |
| **文档中过度使用 Emoji** | 增加视觉趣味性 | 专业 CLI 工具的文档应该干净。过多 emoji 降低可读性，尤其是技术配置文档 | 仅在 README 头部徽章区域和任务列表中使用。避免在命令示例、配置参考、表头中使用 |
| **自动化 API 文档（docs.rs 风格）** | "代码应该自我文档化" | sqllog2db 是 CLI 工具而非库。用户不需要看 `ExporterManager` 的 trait 方法签名 | docs.rs 已经有所有 crate 的 API 文档。README 和 Pages 应该聚焦于如何使用工具，而非内部实现细节。但架构文档（数据流图）是有价值的 |
| **过长的安装说明表格** | 用户总想找到自己的平台 | ripgrep/bat 列出了 20+ 包管理器，但 sqllog2db 目前仅 `cargo install` 一种安装方式。列出跨平台表格显得内容空洞 | 专注写好 `cargo install sqllog2db` 一行。等到有 Homebrew/Pacman 包后再扩展安装表格 |
| **视频教程 / Screencast 视频** | 比文本更直观 | 视频需要更新维护，CI 无法检查 dead link。DBA 用户群体偏好可搜索的文本 | ASCIIcast 比视频更好：纯文本、可搜索、可复制粘贴命令 |
| **全文搜索功能（Pages 上）** | 用户想快速找到特定配置项 | GitHub Pages 静态站点没有原生搜索。集成 lunr.js/lunr.py 增加复杂度 | 组织好目录结构，让用户可以肉眼扫描。内容按 logical 层级组织而非 flat 列表 |
| **独立域名（而非 github.io）** | 看起来更专业 | 需要额外购买域名、配置 DNS/CNAME、维护 HTTPS 证书。对于 Rust CLI 项目，用户更关注功能而非域名 | `<org>.github.io/sqllog2db` 已经足够。如果有域名需求，用 CNAME 指向 Pages 即可 |

---

## 特征依赖关系

```
README（核心入口）
    ├──feeds──> 用户决定是否尝试
    ├──requires──> CHANGELOG.md（README 中提供链接）
    ├──requires──> LICENSE（README 徽章区域显示许可）
    └──feeds──> GitHub Pages（深层内容展示）

GitHub Pages（落地页）
    ├──requires──> README 已定义信息架构（Pages 扩展而非替代）
    ├──requires──> SVG 图表 asset（Gallery 页面）
    ├──requires──> 性能基准数据（Benchmark 页面）
    ├──enhances──> Asciicast 录制内容（可嵌入）
    ├──enhances──> Mermaid.js 架构图
    └──indep of──> CONTRIBUTING.md / SECURITY.md（仓库内独立文件）

docs/ 目录（补充文档）
    ├──requires──> docs/quickstart.md（快速上手）
    ├──requires──> docs/architecture.md（架构说明）
    ├──requires──> docs/config-reference.md（完整配置参考）
    └──links_to──> GitHub Pages（Pages 可以统一展示这些）

CONTRIBUTING.md / SECURITY.md
    └──indep of──> README / Pages 内容

CHANGELOG.md
    └──indep of──> 其他文档（但应在 v1.5 发布前创建）
```

### 依赖说明

- **README → CHANGELOG / LICENSE**：README 头部徽章区域和页脚需要链接到这两个文件。它们应该存在于仓库根目录。
- **GitHub Pages → SVG assets**：Pages 上展示的四类图表需要实际先生成一次 SVG 文件，然后作为静态 asset 托管在 Pages 仓库中。这些 SVG 可以是预生成的样本数据。
- **GitHub Pages → 基准数据**：性能展示需要实际运行基准并记录结果。Pages 是展示这些数据的最佳场所（比 README 更宽敞）。
- **README + Pages 互补**：README 作为"快速入口"，让用户在 GitHub 仓库页面上直接获取核心信息。Pages 作为"深度内容"，用户主动访问时获得更丰富的展示。

---

## v1.5 MVP 定义

### v1.5 必须发布（核心文档，优先级由高到低）

| 优先级 | 特征 | 为什么必须现在做 |
|--------|---------|------------------|
| P1 | **README 全面更新** | 当前 README 落后于 v1.3/v1.4 功能。用户看到的文档与实际功能不匹配，产生不信任 |
| P1 | **CHANGELOG.md** | 整个项目历史没有 changelog。v1.5 截止点应该是创建 changelog 的时机。使用 Keep a Changelog 格式 |
| P1 | **LICENSE 文件** | 仓库根目录缺失 LICENSE 文件。任何公共仓库的必须文件 |
| P1 | **项目徽章（Badges）** | CI status、crates.io version、license 三个核心徽章需要在 README 头部展示 |
| P1 | **QuickStart 示例** | README 中需要 3-5 个可复制粘贴运行的命令行示例，覆盖 `init` + `run` + `digest` + `stats` + `validate` |
| P2 | **GitHub Pages 基础落地页** | 一个美观的 index.html，包含项目介绍、安装命令、快速上手、功能概览。不需要复杂的导航系统 |
| P2 | **性能基准展示** | README 或 Pages 上展示基准测试结果。以表格形式展示合成基准和真实文件基准数据 |
| P2 | **架构 / 数据流图** | README 或 Pages 上用 Mermaid.js 画出数据流图，让用户直观理解工作流程 |
| P2 | **docs/quickstart.md** | 补充文档目录中的快速上手，比 README 更详细但比完整配置参考简洁 |
| P3 | **SVG 图表示例展示** | GitHub Pages 上嵌入实际生成的四类 SVG 图表。这既是特征展示也是输出预览 |
| P3 | **配置参考文档** | docs/config-reference.md 包含完整配置块、每行带注释的 config.toml 示例、serde alias 向后兼容性说明 |
| P3 | **Asciicast Demo** | 录制一个 30 秒 demo 嵌入 README，展示 sqllog2db run 的实时输出 |

### v1.5 可推迟（v1.6+）

| 特征 | 触发条件 |
|---------|-------------------|
| GitHub Pages 完整站点（多页面导航、Gallery、Benchmark 页） | 基础落地页上线后，有明确用户反馈需要更多内容 |
| 中文文档 README.zh-CN.md | 社区有中文用户 PR 贡献翻译，或调研显示中文用户访问量显著 |
| FAQ / Troubleshooting 板块 | 累积到 5 个以上的重复 GitHub Issues |
| CONTRIBUTING.md | 出现外部贡献者的 PR，或明确有意愿招募贡献者 |
| SECURITY.md | 仓库正式公开后 |
| Playground / WASM Web Demo | v1.5+ 长期规划，目前复杂度高、ROI 不确定 |

### 未来考虑（v2+）

| 特征 | 为什么推迟 |
|---------|-------------------|
| 视频教程 | 维护成本高，ROI 不确定 |
| 独立域名 | 项目未达到需要独立域名的阶段 |
| 完整 mdBook 文档站 | 过度工程化，当前文档规模不需要 |
| 交互式配置生成器 | 仅当配置模型变得非常复杂时才有价值 |

---

## 特征优先级矩阵

| 特征 | 用户价值 | 实现成本 | 优先级 |
|---------|------------|-------------|----------|
| README 全面更新（同步 v1.3/v1.4 功能） | HIGH | LOW | P1 |
| CHANGELOG.md | HIGH | LOW | P1 |
| LICENSE 文件 | HIGH | LOW | P1 |
| 项目徽章（Badges） | HIGH | LOW | P1 |
| QuickStart 示例 | HIGH | LOW | P1 |
| GitHub Pages 基础落地页 | MED | MED | P2 |
| 性能基准展示 | HIGH | MED | P2 |
| 架构 / 数据流图 | MED | LOW | P2 |
| docs/quickstart.md | MED | LOW | P2 |
| SVG 图表示例展示 | MED | LOW | P3 |
| 配置参考文档 | MED | MED | P3 |
| Asciicast Demo | MED | LOW | P3 |
| GitHub Pages 完整站点 | MED | HIGH | 推迟 |
| 中文文档 | LOW | MED | 推迟 |
| FAQ / Troubleshooting | MED | MED | 推迟 |
| CONTRIBUTING.md | LOW | LOW | 推迟 |
| SECURITY.md | LOW | LOW | 推迟 |
| Playground / WASM | LOW | HIGH | 推迟 |

---

## 竞争分析：其他 Rust CLI 工具的文档实践

| 特征 | ripgrep | fd | bat | ruff | hyperfine | zoxide | bottom | sqllog2db 计划 |
|---------|-------------|--------|-------|--------|-----------|--------|--------|----------------|
| README 结构完整 | ✓ 基准表格 + 为何用/为何不用 | ✓ 详尽使用分类 + 集成 | ✓ 展示驱动 + 深度定制 | ✓ 基准图 + 证言 + 集成 | ✓ 用法 + benchmark | ✓ 安装为主 + 链接至 Wiki | ✓ 功能 + 安装 + 配置 | ✓ — 问题问题修复当前 README 缺失的 v1.3/v1.4 内容 |
| 安装覆盖 | 20+ 包管理器 | 15+ | 15+ | 4 种优先路径 + 多 PM | 14+ | tab 式多平台 + 推荐方法 | 22+ 包管理器 | cargo install 为主，明确标注"目前仅此一种" |
| 性能基准 | ✓ 多语料 + 多工具比较表格 | ✓ find 对比 | — | ✓ SVG 柱状图（CPython 全栈） | ✓ 自身 benchmark 结果展示 | ✓ 隐式（badge）| — | ✓ README + Pages 展示合成和真实基准 |
| 图表/可视化 | — | ✓ SVG 屏幕录像 | ✓ 多截图对比 | ✓ SVG 柱状图 | ✓ 输出的 Markdown 表格 | ✓ 截图 | ✓ GIF demo | ✓ SVG 图表示例 + asciicast |
| GitHub Pages | — | — | — | docs.astral.sh | — | — | ✓ mkdocs 站点 | ✓ Pages 基础落地页 |
| 配置参考 | ✓ 独立文档 | — | ✓ 注释式示例 | ✓ 默认配置 TOML | — | ✓ CLI flags 表格 + env 表格 | ✓ mkdocs 配置页 | ✓ docs/config-reference.md |
| "为何不用" 板块 | ✓ 明确的缺点说明 | ✓ 诚实局限 | ✓ 兼容性问题表格 | — | — | — | — | ✓ 可选 |
| 集成 3rd party 示例 | — | ✓ fzf/rofi/emacs | ✓ 8 种集成 | ✓ GitHub Actions + VS Code + pre-commit | ✓ chronologer/bencher | ✓ 20+ 第三方集成表格 | — | 推迟，目前无集成需求 |
| 数据流/架构图 | — | — | — | — | ✓ ASCII 执行流程图 | — | — | ✓ Mermaid.js 数据流图 |

---

## sqllog2db 特有的文档亮点

基于 sqllog2db 的技术特点，以下是在文档中应该着重突出的差异化能力：

1. **流式处理 + 常量内存**：这是 sqllog2db 对比脚本式 log 解析工具（如 awk/grep 脚本或 Python parser）的核心优势。建议在 README 和 Pages 上用一个"内存占用 vs 文件大小"的曲线图展示
2. **SVG 图表自包含页面**：sqllog2db 生成的 SVG 图表是完全自包含的（无外部 CSS/JS/字体依赖），双击即可在任何浏览器打开。这是技术亮点，应该在文档中强调
3. **达梦数据库特定优化**：日式/达梦 SQL 日志格式的特殊处理（注释剥离、IN 列表折叠、关键字归一化）是 niche 优势
4. **向后兼容的配置模型**：serde alias + RawFiltersFeature 中间 struct = 旧配置仍然可用。应该明确标注兼容性矩阵

---

## 复杂度注释

| 特征 | 估计复杂度 | 理由 |
|---------|---------------------|--------|
| README 更新 | LOW | 已有项目知识和现有 README 草稿可参考。主要工作是把 v1.3/v1.4 功能补充进去 |
| CHANGELOG.md | LOW | Keep a Changelog 格式。回顾 git log 整理变更 |
| LICENSE 文件 | LOW | 拷贝标准 MIT 或 Apache-2.0 模板 |
| 徽章集成 | LOW | shields.io URL 生成。Markdown image 链接 |
| QuickStart 示例 | LOW | 从现有代码中提取实际运行的命令 |
| 架构图（Mermaid.js） | LOW | Mermaid.js 流程图标记，GitHub 原生支持渲染 |
| 性能基准展示 | MED | 需要运行基准并分析结果数据，决定展示哪些指标 |
| SVG 图表示例 | LOW | 实际运行一次 sqllog2db charts 生成 SVG，选取最佳质量的四个 |
| GitHub Pages 落地页 | MED | 需要设计 HTML/CSS，选择 Pages 主题（Jekyll 或手写） |
| docs/config-reference.md | MED | 需要系统性地遍历所有配置块、默认值、验证规则 |
| Asciicast Demo | LOW | asciinema 录制 + SVG 嵌入（terminalizer 或 agg 渲染）|
| GitHub Pages 完整多页站点 | HIGH | 需要设计导航、多页面模板、Gallery 页、移动端适配 |

---

## Sources

- [rgerup.ripgrep README](https://github.com/BurntSushi/ripgrep) — 文档结构最佳实践：基准表格、"Why use / Why not" 板块、平台安装覆盖（HIGH confidence）
- [sharkdp/fd README](https://github.com/sharkdp/fd) — 渐进式文档 + 问题-解决模式 + 集成示例（HIGH confidence）
- [sharkdp/bat README](https://github.com/sharkdp/bat) — 展示驱动 + 深度定制配置 + 故障排除（HIGH confidence）
- [sharkdp/hyperfine README](https://github.com/sharkdp/hyperfine) — 基准文档 + ASCII 流程图 + 用法场景分类（HIGH confidence）
- [astral-sh/ruff README](https://github.com/astral-sh/ruff) — 可视化基准 + 社会化证言 + 集成生态（HIGH confidence）
- [ajeetdsouza/zoxide README](https://github.com/ajeetdsouza/zoxide) — 安装优先 + 平台 tab + 环境变量文档（HIGH confidence）
- [ClementTsang/bottom docs](https://clementtsang.github.io/bottom/stable/) — MkDocs 多页导航 + 分层文档 + stable/nightly 双通道（HIGH confidence）
- [eza-community/eza README](https://github.com/eza-community/eza) — collapsible 选项参考 + 区分上游项目（MEDIUM confidence）
- [ducaale/xh README](https://github.com/ducaale/xh) — 诚实对比 + 安装表格 + footnoted caveats（MEDIUM confidence）
- [bat: configuration as code](https://github.com/sharkdp/bat) — 注释驱动的配置示例风格（HIGH confidence）
- [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) — CHANGELOG.md 格式规范（HIGH confidence）
- [asciinema](https://asciinema.org/) — 终端录制嵌入工具（MEDIUM confidence）

---

*Feature research for: sqllog2db v1.5 — 文档完善 & 项目展示*
*Researched: 2026-05-18*
