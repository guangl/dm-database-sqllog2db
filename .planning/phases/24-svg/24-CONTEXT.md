# Phase 24: 文档中文化 & 去 SVG 化 - Context

**Gathered:** 2026-05-19
**Status:** Ready for planning

## Phase Boundary

将所有用户面文档翻译为中文（README.md、GitHub Pages 落地页、docs/quickstart.md、docs/config-reference.md），并从文档中移除 SVG 图表引用（代码保留）。

**In scope:** I18N-01~04, DESVG-01~02
**Out of scope:** 代码中 SVG 图表功能移除（仅清理文档引用，src/charts/ 及 plotters 依赖保留）；CLAUDE.md 中文化（保持中英混排现状）

## Implementation Decisions

### 翻译策略
- **D-01:** 机翻+人工校对 — 先用翻译工具生成初稿，再人工校对修正，确保术语准确
- **D-02:** CLAUDE.md 保持现状，不在 Phase 24 翻译范围

### SVG 替代方案
- **D-03:** README 和 GitHub Pages 中的 SVG 图表引用替换为纯文字描述（图表类型 + 含义），不保留视觉元素
- **D-04:** 与 v1.5 决策一致：不引入新的图表渲染依赖

### 站点配置
- **D-05:** site/book.toml 的 `language = "en"` 在 Phase 24 改为 `"zh"`，Phase 26 不再重复修改

### Claude's Discretion
- 翻译工具选择
- 具体措辞的风格统一（术语表）
- 机翻初稿与人工校对的工作量分配

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求文档
- `.planning/REQUIREMENTS.md` — I18N-01~04（文档中文化）和 DESVG-01~02（去 SVG 化）的权威描述
- `.planning/ROADMAP.md` — Phase 24 的范围和依赖关系

### 项目规范
- `.planning/PROJECT.md` — v1.6 里程碑目标、Context 和 Key Decisions（含 I18N+DESVG 合并决策、不保留双语版本决策）
- `site/book.toml` — mdBook 配置，`language` 字段需从 `"en"` 改为 `"zh"`

### 待修改文件
- `README.md` — 项目主文档
- `site/src/index.md` — GitHub Pages 落地页
- `docs/quickstart.md` — 快速入门
- `docs/config-reference.md` — 配置参考

## Existing Code Insights

### Reusable Assets
- 现有文档结构已成熟，只需翻译和去 SVG，无需新增目录或文件

### Established Patterns
- v1.5 文档英文风格：简洁、直接、面向 DBA
- v1.5 决策 "ASCII art 替代 Mermaid.js" — 同类的工具无关化替代思路

### Integration Points
- Phase 24 的输出（中文文档）是 Phase 26（GitHub Pages 多页文档站）的输入
- site/book.toml 的 language 改动需与 Phase 26 的 mdBook 多页结构兼容

## Specific Ideas

- 中文化后可能需要调整行宽和排版（中文每行字数比英文少）
- 术语一致性：建议建立小术语表（如 "pipeline" → "处理管道", "exporter" → "导出器"）

## Deferred Ideas

None — discussion stayed within phase scope

---

*Phase: 24-文档中文化 & 去 SVG 化*
*Context gathered: 2026-05-19*
