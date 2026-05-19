# Phase 23: 补充文档 + CI 质量门禁 - Context

**Gathered:** 2026-05-18
**Status:** Ready for planning

## Phase Boundary

创建详细的快速入门指南（docs/quickstart.md）和完整配置参考文档（docs/config-reference.md）。录制 Asciicast 终端演示并嵌入 README 和 Pages。CI 工作流集成 lychee 链接检查，防止文档链接腐化。

## Implementation Decisions

### docs/quickstart.md
- **D-01:** 按使用场景分教程：场景1-导出CSV、场景2-导出SQLite、场景3-统计分析、场景4-模板聚合。每个场景含完整命令链和预期输出。
- **D-02:** 比 README QuickStart 更详细，包含环境准备（Rust 安装）、完整终端输出示例、常见错误及解决方法。

### docs/config-reference.md
- **D-03:** 每个配置块独立章节（[sqllog]、[logging]、[filter.include]/[filter.exclude]、[template]、[charts]、[exporter.csv]/[exporter.sqlite]、[features.replace_parameters]）。
- **D-04:** 每节包含：带注释的 TOML 示例、字段表格（名称/类型/默认值/说明）、注意事项。

### docs/ 目录结构
- **D-05:** 所有文档平铺在 `docs/` 根目录（quickstart.md、config-reference.md），不建子目录。v1.6+ 添加 architecture.md 等同样平铺。

### Asciicast 终端演示
- **D-06:** 录制完整 3 步流程（init → validate → run），展示从生成配置到导出成功的全过程，约 30-45 秒。
- **D-07:** README 放静态 SVG 预览+链接到 asciinema.org，Pages 放交互式 asciinema-player 嵌入。

### lychee 链接检查
- **D-08:** 扫描所有 Markdown 文件（README.md、CHANGELOG.md、docs/*.md、site/*.md），不包括 .planning/。
- **D-09:** 外部链接（crates.io、github.com、docs.rs）配置 `--max-retries 3 --timeout 30` 允许重试，失败阻塞 CI。
- **D-10:** 仅 Markdown 文件变更时触发（GitHub Actions `paths` 过滤），减少 CI 时间。
- **D-11:** 内部链接（相对路径）失败直接阻塞 CI。

### Claude's Discretion
- docs/quickstart.md 各场景的具体命令和示例输出内容
- docs/config-reference.md 的具体字段表格和示例
- Asciicast 录制的具体终端内容（需实际运行命令录制）
- lychee GitHub Actions workflow 的完整实现
- lychee 忽略的 URL 模式（如 crates.io 可能存在速率限制）

## Canonical References

### 项目文档
- `.planning/REQUIREMENTS.md` — v1.5 全部需求（SUPP-02 至 SUPP-05 分配给 Phase 23）
- `.planning/ROADMAP.md` — Phase 23 定义、依赖关系（依赖 Phase 22）、成功标准
- `.planning/PROJECT.md` — 项目上下文

### 上游产物
- `.planning/phases/21-readme/21-CONTEXT.md` — Phase 21 决策（README 精简影响 quickstart 内容范围）
- `.planning/phases/22-github-pages/22-CONTEXT.md` — Phase 22 决策（Pages 内容影响文档链接布局）
- `README.md` — Phase 21 产物，Asciicast 嵌入目标
- `site/` — Phase 22 产物，lychee 检查目标

### 外部参考
- https://lychee.cli.rs/ — lychee CLI 使用和配置
- https://asciinema.org/ — Asciicast 录制和嵌入
- `config.toml` — 当前配置模板（config-reference.md 基础）

## Existing Code Insights

### Reusable Assets
- `sqllog2db init -o config.toml --force` — 生成默认配置（config-reference.md 基于此）
- CLI 所有子命令 — quickstart.md 各场景的命令来源
- `sqllog2db run` 实时输出 — Asciicast 录制内容

### Established Patterns
- 项目配置 TOML 风格 — config-reference.md 需保持一致
- GitHub Actions workflow YAML 结构 — 新增 lychee workflow 遵循 ci.yaml 风格

### Integration Points
- README.md（Phase 21）— 链接到 docs/quickstart.md 和 docs/config-reference.md
- GitHub Pages（Phase 22）— 嵌入 Asciicast 交互式播放器
- CI workflow（Phase 22）— lychee 作为独立 workflow 文件加入

## Specific Ideas

- docs/quickstart.md 目标长度：每个场景 20-40 行，总计 150-250 行
- docs/config-reference.md 目标长度：7-8 个配置块章节，总计 300-400 行
- Asciicast 终端尺寸：120x30，展示完整输出不折行
- lychee 可考虑缓存以加速重复运行（`--cache` flag）

## Deferred Ideas

- docs/architecture.md 详细架构文档 — v1.6+（DOC-F03）
- FAQ / Troubleshooting 板块 — v1.6+（DOC-F05）
- 自动化文档生成（从代码注释）— 不在 v1.5 范围

---
*Phase: 23-补充文档 + CI 质量门禁*
*Context gathered: 2026-05-18*
