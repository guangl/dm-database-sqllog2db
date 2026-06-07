# Phase 3: 文档与验证对齐 - Context

**Gathered:** 2026-06-07
**Status:** Ready for planning

<domain>
## Phase Boundary

三项文档对齐工作：
1. **QUAL-01** — Phase 67/68/69/70 的 VALIDATION.md 从草稿/缺失状态补全为正式验证记录（包含实际验证结果）
2. **DOC-04** — README.md 补充 `watch` 子命令用法示例、`init --interactive` 操作说明、`--quiet`/`--verbose` 进度选项说明
3. **DOC-05** — `watch --help`、`validate --help` 各补充至少 1 个使用示例（达到 ≥2 个）；`stats --help` 已有 3 个示例，不需要改动

</domain>

<decisions>
## Implementation Decisions

### QUAL-01: VALIDATION.md 重建方式

[auto] Q: "为已完成的 Phase 67/68/69/70 构建 VALIDATION.md 时采用哪种方式？" → Selected: "从 SUMMARY.md self-check/requirements-completed 重建" (历史记录完整，不需要重跑命令，直接标记为 complete 状态)

- **D-01:** Phase 67/68/69/70 的 VALIDATION.md 从各自的 SUMMARY.md 提取 self-check 条目、requirements-completed 列表和 metrics 数据来构建。格式参考 Phase 01/02 的 VALIDATION.md（status、nyquist_compliant、per-task 验证表）。
- **D-02:** 这四个阶段的 VALIDATION.md 标记 `status: complete`、`nyquist_compliant: true`，并在"Validation Sign-Off"中标注所有条目已通过（以 SUMMARY.md 的 self-check: PASSED 为依据）。
- **D-03:** 每个 VALIDATION.md 包含"Test Infrastructure"（测试命令）、"Per-Task Verification Map"（从 SUMMARY 的 tasks_completed 条目转录）、"Validation Sign-Off"三部分。Wave 0 Requirements 和 Manual-Only 部分可省略（阶段已完成，无待机测试）。
- **D-04:** Phase 67 有 3 个 SUMMARY（01/02/03），Phase 68 有 2 个，Phase 69 有 4 个，Phase 70 有 3 个。每个阶段写一份汇总 VALIDATION.md，不按 plan 拆分。

### DOC-04: README 新增内容结构

[auto] Q: "watch/init --interactive/进度选项说明放在 README 哪个位置？" → Selected: "在功能特性 CLI 条目更新（加入 watch），然后在 README 适当位置新增三个说明段落" (与现有 README 结构对齐，minimal diff)

- **D-05:** "功能特性 → 配置与性能"区域的 CLI 条目从"四个命令（init/validate/run/stats）"更新为"五个命令（init/validate/run/stats/watch）"。
- **D-06:** 在 README 的"功能特性"区域（过滤与字段控制或配置与性能章节附近）新增一个"持续监听"条目，描述 watch 子命令行为（目录监听、500ms 防抖、增量处理、Ctrl+C 摘要）。
- **D-07:** 在 README 适当位置（如"快速上手"或现有使用示例区域）新增三个说明段落，每段 1-3 行：
  - `watch` 用法：`sqllog2db watch -c config.toml`，说明启动/停止、CSV/SQLite 配置来自 config.toml
  - `init --interactive`：`sqllog2db init --interactive`，说明交互式向导生成配置
  - `--quiet`/`--verbose`：全局选项说明，`-q` 抑制非错误输出，`-v` 显示每文件详情

### DOC-05: --help 示例补充

[auto] Q: "watch/validate --help 各需要补充哪些示例？" → Selected: "watch 补充 quiet 模式示例; validate 补充 verbose 输出示例" (watch 当前 1 个，validate 当前 1 个，各补充 1 个即达 ≥2 要求)

- **D-08:** `watch --help` 补充第 2 个示例：静默运行（适合定时任务/后台进程）：
  ```
      Watch in quiet mode (suitable for cron/background):
          sqllog2db watch -c config.toml --quiet
  ```
- **D-09:** `validate --help` 补充第 2 个示例：显示详细信息（含字段检查输出）：
  ```
      Validate and show detailed field information:
          sqllog2db validate -c config.toml --verbose
  ```
- **D-10:** `stats --help` 已在 `src/cli/opts.rs` 中包含 3 个示例（default top-20、--top 5、时间范围过滤），**不需要修改**。DOC-05 中 stats 部分已满足。
- **D-11:** 修改点在 `src/cli/opts.rs` 的 `Watch` 和 `Validate` variant 的 `after_help` 字符串。

### Claude's Discretion

- README 的 watch/init --interactive 示例段落放在文档哪个具体位置（"功能特性"末尾 vs 独立章节），由 planner 根据现有 README 结构决定，要求与整体风格一致。
- VALIDATION.md 中"Per-Task Verification Map"的任务粒度（按 plan 还是按 requirement），由 planner 参考 Phase 01/02 VALIDATION.md 格式决定。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 3: 文档与验证对齐" — Goal、Success Criteria（SC1–SC4）
- `.planning/REQUIREMENTS.md` §QUAL-01、DOC-04、DOC-05

### VALIDATION.md 参考格式
- `.planning/phases/01-watch/01-VALIDATION.md` — Phase 01 完成态 VALIDATION.md 格式样本（status: draft，but self-check 内容完整）
- `.planning/phases/02-fsevents/02-VALIDATION.md` — Phase 02 VALIDATION.md 格式样本（per-task 验证表结构）
- `.planning/phases/67-prog-diag/67-01-SUMMARY.md` — Phase 67 Plan 01 self-check 内容
- `.planning/phases/67-prog-diag/67-02-SUMMARY.md` — Phase 67 Plan 02 内容
- `.planning/phases/67-prog-diag/67-03-SUMMARY.md` — Phase 67 Plan 03 内容
- `.planning/phases/68-init-wizard/68-01-SUMMARY.md` — Phase 68 Plan 01 内容
- `.planning/phases/68-init-wizard/68-02-SUMMARY.md` — Phase 68 Plan 02 内容
- `.planning/phases/69-watch/69-01-SUMMARY.md` — Phase 69 Plan 01 内容
- `.planning/phases/69-watch/69-02-SUMMARY.md` — Phase 69 Plan 02 内容
- `.planning/phases/69-watch/69-03-SUMMARY.md` — Phase 69 Plan 03 内容
- `.planning/phases/69-watch/69-04-SUMMARY.md` — Phase 69 Plan 04 内容
- `.planning/phases/70-watch/70-01-SUMMARY.md` — Phase 70 Plan 01 内容
- `.planning/phases/70-watch/70-02-SUMMARY.md` — Phase 70 Plan 02 内容
- `.planning/phases/70-watch/70-03-SUMMARY.md` — Phase 70 Plan 03 内容

### 核心实现文件
- `src/cli/opts.rs` — 所有子命令的 `after_help` 示例字符串（DOC-05 修改点：Watch/Validate variant）
- `README.md` — 主文档（DOC-04 修改点）

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/cli/opts.rs`：`after_help = "\EXAMPLES:\n..."` 模式已在所有子命令中使用；补充示例只需在字符串末尾追加，格式与现有示例对齐（4空格缩进描述 + 8空格缩进命令）
- Phase 01/02 VALIDATION.md：已建立的 VALIDATION.md 格式模板（frontmatter + Test Infrastructure + Per-Task Verification Map + Manual-Only Verifications + Sign-Off）

### Established Patterns
- --help 示例格式（`opts.rs`）：描述行 4 空格缩进，命令行 8 空格缩进，示例之间空行分隔
- VALIDATION.md 格式：YAML frontmatter（phase/slug/status/nyquist_compliant/wave_0_complete/created/updated）+ markdown 正文
- README 功能特性条目格式：粗体标题 + em dash + 描述（参考"流式解析器"、"灵活的输入模式"等条目）

### Integration Points
- `Watch` variant `after_help`（`src/cli/opts.rs:172-175`）：在现有单行示例后追加第 2 个示例
- `Validate` variant `after_help`（`src/cli/opts.rs:107-110`）：在现有单行示例后追加第 2 个示例
- README 功能特性区域"简洁的 CLI"条目（需更新命令数量并加入 watch）

</code_context>

<specifics>
## Specific Ideas

- DOC-04 明确三个子项：watch 用法示例、init --interactive 说明、--quiet/--verbose 选项说明
- DOC-05 明确三个子命令：watch/validate/stats ≥2 示例，stats 已满足（3 个），只需补 watch 和 validate
- QUAL-01 Phase 67/68/69/70 对应的 git 历史确认这些阶段已完成（SUMMARY.md 存在且 self-check: PASSED）

</specifics>

<deferred>
## Deferred Ideas

- Phase 67/68/69/70 的 `run --help` 和 `init --help` 示例进一步丰富——当前 DOC-05 只要求 watch/validate/stats，run 和 init 已达标，额外示例留后续 milestone
- README 多语言版本（英文 README）——超出本 milestone 范围
- CHANGELOG.md 补充 v1.19 条目——本阶段 README 更新会自然带到 CHANGELOG，但 CHANGELOG 不是 DOC-04 的显式要求，留 planner 判断

</deferred>

---

*Phase: 3-doc-align*
*Context gathered: 2026-06-07*
