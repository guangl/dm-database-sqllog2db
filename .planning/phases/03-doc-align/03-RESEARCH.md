# Phase 3: 文档与验证对齐 - Research

**Researched:** 2026-06-07
**Domain:** 文档补全（Markdown）+ Rust CLI `after_help` 字符串修改
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**QUAL-01: VALIDATION.md 重建方式**
- D-01: Phase 67/68/69/70 的 VALIDATION.md 从各自的 SUMMARY.md 提取 self-check 条目、requirements-completed 列表和 metrics 数据来构建。格式参考 Phase 01/02 的 VALIDATION.md（status、nyquist_compliant、per-task 验证表）。
- D-02: 这四个阶段的 VALIDATION.md 标记 `status: complete`、`nyquist_compliant: true`，并在"Validation Sign-Off"中标注所有条目已通过（以 SUMMARY.md 的 self-check: PASSED 为依据）。
- D-03: 每个 VALIDATION.md 包含"Test Infrastructure"（测试命令）、"Per-Task Verification Map"（从 SUMMARY 的 tasks_completed 条目转录）、"Validation Sign-Off"三部分。Wave 0 Requirements 和 Manual-Only 部分可省略（阶段已完成，无待机测试）。
- D-04: Phase 67 有 3 个 SUMMARY（01/02/03），Phase 68 有 2 个，Phase 69 有 4 个，Phase 70 有 3 个。每个阶段写一份汇总 VALIDATION.md，不按 plan 拆分。

**DOC-04: README 新增内容结构**
- D-05: "功能特性 → 配置与性能"区域的 CLI 条目从"四个命令（init/validate/run/stats）"更新为"五个命令（init/validate/run/stats/watch）"。
- D-06: 在 README 的"功能特性"区域新增一个"持续监听"条目，描述 watch 子命令行为（目录监听、500ms 防抖、增量处理、Ctrl+C 摘要）。
- D-07: 在 README 适当位置新增三个说明段落：
  - `watch` 用法：`sqllog2db watch -c config.toml`，说明启动/停止、CSV/SQLite 配置来自 config.toml
  - `init --interactive`：`sqllog2db init --interactive`，说明交互式向导生成配置
  - `--quiet`/`--verbose`：全局选项说明，`-q` 抑制非错误输出，`-v` 显示每文件详情

**DOC-05: --help 示例补充**
- D-08: `watch --help` 补充第 2 个示例：
  ```
      Watch in quiet mode (suitable for cron/background):
          sqllog2db watch -c config.toml --quiet
  ```
- D-09: `validate --help` 补充第 2 个示例：
  ```
      Validate and show detailed field information:
          sqllog2db validate -c config.toml --verbose
  ```
- D-10: `stats --help` 已有 3 个示例，**不需要修改**。
- D-11: 修改点在 `src/cli/opts.rs` 的 `Watch` 和 `Validate` variant 的 `after_help` 字符串。

### Claude's Discretion

- README 的 watch/init --interactive 示例段落放在文档哪个具体位置（"功能特性"末尾 vs 独立章节），由 planner 根据现有 README 结构决定，要求与整体风格一致。
- VALIDATION.md 中"Per-Task Verification Map"的任务粒度（按 plan 还是按 requirement），由 planner 参考 Phase 01/02 VALIDATION.md 格式决定。

### Deferred Ideas (OUT OF SCOPE)

- Phase 67/68/69/70 的 `run --help` 和 `init --help` 示例进一步丰富
- README 多语言版本（英文 README）
- CHANGELOG.md 补充 v1.19 条目——留 planner 判断（REQUIREMENTS.md 明确排除 CHANGELOG 为 Out of Scope）
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| QUAL-01 | Phase 67/68/69 VALIDATION.md 草稿补全为正式文件，Phase 70 VALIDATION.md 新建 | SUMMARY.md 全部读取完成；Phase 67（3 plan）、68（2 plan）、69（4 plan）、70（3 plan）self-check 全部 PASSED，数据足以重建 |
| DOC-04 | README 补充 watch 用法、init --interactive 说明、进度选项（续 v1.16 DOC-01/02/03） | README 当前状态已确认：watch 完全缺失，init --interactive 缺失，--quiet/--verbose 全局选项缺失；插入点已定位（第 41 行"简洁的 CLI"条目，第 105 行"快速入门"章节） |
| DOC-05 | watch / validate / stats 子命令 --help 补充示例和选项说明 | opts.rs 完整读取：Watch 当前 1 个示例（行 172-175），Validate 当前 1 个示例（行 107-110），Stats 已有 3 个示例（行 126-135）。D-08/D-09 示例文本已由决策层确定 |
</phase_requirements>

---

## Summary

Phase 3 是纯文档/文字类工作，不涉及任何功能代码逻辑变更。三个任务分别对应三个需求：

**QUAL-01** 要求为 Phase 67/68/69/70 各建立一份汇总 VALIDATION.md。所有 SUMMARY.md 已确认存在且 self-check: PASSED，数据来源充足——直接从 SUMMARY 提取 tasks_completed、requirements-completed、metrics 转录即可，无需重跑任何命令。

**DOC-04** 要求更新 README.md，增加 watch 子命令用法、init --interactive 说明和 --quiet/--verbose 全局选项说明。当前 README 第 41 行"简洁的 CLI"条目列出四个命令（无 watch），"快速入门"章节（第 105 行起）仅含 init/validate/run/stats 示例，三项内容均完全缺失。

**DOC-05** 要求补充 watch 和 validate 的 --help 示例。opts.rs 中 Watch 当前 1 个示例、Validate 当前 1 个示例，各追加 1 个即可达 ≥2 要求；Stats 已有 3 个示例，不需修改。

**Primary recommendation:** 三个需求互相独立，可以分三个 plan 按顺序执行；每个 plan 提交后运行 `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` 确认无回归（DOC-05 修改 Rust 源文件，需要编译验证；DOC-04 和 QUAL-01 纯 Markdown，仅需 fmt 检查）。

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| VALIDATION.md 重建 | 文档层（.planning/phases/） | — | 纯文档补全，无代码变更 |
| README 更新（DOC-04） | 文档层（README.md） | — | Markdown 文档，无代码变更 |
| --help 示例（DOC-05） | CLI 层（src/cli/opts.rs） | — | Rust `after_help` 字符串，需编译验证 |

---

## Current File State（已确认）[VERIFIED: 直接读取文件]

### opts.rs 现有 after_help 状态

**Watch variant（行 172-175）：**
```
EXAMPLES:
    Watch and process new log files automatically:
        sqllog2db watch -c config.toml
```
当前 1 个示例 → 需追加 1 个。

**Validate variant（行 107-110）：**
```
EXAMPLES:
    Validate a configuration file:
        sqllog2db validate -c config.toml
```
当前 1 个示例 → 需追加 1 个。

**Stats variant（行 126-135）：**
已有 3 个示例（default top-20、--top 5、时间范围过滤）→ **不需要修改**。

### README.md 当前缺失内容

- 第 41 行：`**简洁的 CLI**：\`init\`（生成配置）、\`validate\`（校验）、\`run\`（执行导出）、\`stats\`（统计分析）四个命令。` → 需更新为五个命令并加入 watch。
- 第 105 行以下"快速入门"章节：仅含 init/validate/run/stats 示例，无 watch / --interactive / --quiet/--verbose 内容。
- 功能特性区域无 watch 条目。

### VALIDATION.md 现有状态

- Phase 67：无 VALIDATION.md（目录中仅有 3 个 SUMMARY.md）[VERIFIED: 目录扫描]
- Phase 68：无 VALIDATION.md（目录中仅有 2 个 SUMMARY.md）[VERIFIED: 目录扫描]
- Phase 69：无 VALIDATION.md（目录中仅有 4 个 SUMMARY.md）[VERIFIED: 目录扫描]
- Phase 70：无 VALIDATION.md（目录中仅有 3 个 SUMMARY.md）[VERIFIED: 目录扫描]
- Phase 01：有 VALIDATION.md，`status: draft`，格式参考样本（含 Test Infrastructure / Per-Task Verification Map / Wave 0 Requirements / Manual-Only / Sign-Off）[VERIFIED: 读取文件]
- Phase 02：有 VALIDATION.md，`status: draft`，格式参考样本（per-task 验证表含 Task ID/Plan/Wave/Requirement/Threat Ref/Secure Behavior/Test Type/Automated Command/File Exists/Status）[VERIFIED: 读取文件]

---

## Architecture Patterns

### VALIDATION.md 目标格式（基于 Phase 01/02 样本）[VERIFIED: 读取文件]

```yaml
---
phase: {N}
slug: {slug}
status: complete          # 注意：完成态改为 complete（非 draft）
nyquist_compliant: true   # 注意：已完成阶段为 true
wave_0_complete: true     # 注意：已完成阶段为 true
created: {date}
updated: {date}
---
```

正文结构：
1. `## Test Infrastructure` — 框架、配置文件、Quick run command、Full suite command
2. `## Sampling Rate` — 每任务提交后/每 wave 合并后/verify-work 前的命令
3. `## Per-Task Verification Map` — 表格（Task ID / Plan / Wave / Requirement / Threat Ref / Secure Behavior / Test Type / Automated Command / File Exists / Status）
4. `## Validation Sign-Off` — 所有条目标为已通过（SUMMARY self-check: PASSED 为依据）

已完成阶段省略 Wave 0 Requirements 和 Manual-Only Verifications 两节（D-03 决策）。

### opts.rs `after_help` 格式规范（已验证）[VERIFIED: 读取文件]

```
after_help = "\
EXAMPLES:
    {描述行 4 空格缩进}:
        {命令行 8 空格缩进}

    {描述行 4 空格缩进}:
        {命令行 8 空格缩进}"
```

示例之间空行分隔，末尾无多余空行，字符串以 `"` 结束。

### README 条目格式（已验证）[VERIFIED: 读取文件]

功能特性列表项格式：`- **{粗体标题}**：{描述文字}`
快速入门代码块：独立 bash fenced code block，每个命令一行。

---

## SUMMARY.md 数据摘要（VALIDATION.md 重建依据）

### Phase 67（prog-diag）[VERIFIED: 读取所有 3 个 SUMMARY]

| Plan | 需求 | 关键任务 | Self-Check |
|------|------|----------|-----------|
| 67-01 | PROG-01, PROG-02 | make_progress_bar 新签名、tick_progress records/sec；2 个单元测试 | PASSED |
| 67-02 | DIAG-01, DIAG-02 | ErrorKind/ParseErrorRecord/classify_error_kind/truncate_to_120_chars；4 个单元测试 | PASSED |
| 67-03 | PROG-03, DIAG-03 | print_run_summary 扩展、write_error_log、filtered_out 递增；3 个集成/单元测试 | PASSED |

全套命令：`cargo test --lib` 344 passed，clippy clean，commits：db845cc / bc81d53 / 67feea0

### Phase 68（init-wizard）[VERIFIED: 读取所有 2 个 SUMMARY]

| Plan | 需求 | 关键任务 | Self-Check |
|------|------|----------|-----------|
| 68-01 | INIT-01, INIT-02, INIT-03 | run_wizard/WizardAnswers/ExporterChoice/handle_init_interactive；12 个单元测试 | PASSED |
| 68-02 | INIT-01, INIT-02, INIT-03 | 6 个 e2e assert_cmd 集成测试 | PASSED |

全套命令：`cargo test` 全部通过，clippy clean，commits：d784e67 / 263d536 / 8fe0f1f / 862e6f1

### Phase 69（watch）[VERIFIED: 读取所有 4 个 SUMMARY]

| Plan | 需求 | 关键任务 | Self-Check |
|------|------|----------|-----------|
| 69-01 | WATCH-01 | notify="6" 依赖、ErrorStats.records_exported、Commands::Watch 骨架 | PASSED |
| 69-02 | WATCH-01, WATCH-05, WATCH-06（部分） | handle_watch 完整实现、collect_watch_dirs、format_elapsed_hms、main.rs Watch arm | PASSED |
| 69-03 | WATCH-01, WATCH-02, WATCH-05, WATCH-06 | 4 个 watch e2e 集成测试；canonicalize + Modify(Data(Content)) 修复 | PASSED |
| 69-04 | WATCH-02, WATCH-05 | HumanDuration 动态 last 字段、500ms 防抖窗口 WatchLoopState；4 个单元测试 | PASSED |

全套命令：`cargo test` 852 passed, 2 ignored（`test_watch_triggers_on_new_log_file` macOS stdin-pipe 问题，Phase 70 修复），clippy clean，commits：0a4c6c5 / 7192009 / b129dda / 72ee41d / d2545c3 / 409a053 / 97f898b

### Phase 70（watch）[VERIFIED: 读取所有 3 个 SUMMARY]

| Plan | 需求 | 关键任务 | Self-Check |
|------|------|----------|-----------|
| 70-01 | WATCH-04（基础设施） | tempfile 提升、watch.rs→watch/mod.rs 迁移、offsets.rs（ensure/load/save）；5 个单元测试 | PASSED |
| 70-02 | WATCH-03, WATCH-04 | WatchLoopState 扩展 file_offsets、trigger_full_file、trigger_incremental；4 个单元测试 | PASSED |
| 70-03 | WATCH-03, WATCH-04 | tests/watch_incremental.rs 4 个集成测试（WATCH-03 追加不重复、WATCH-04 重启恢复） | PASSED |

全套命令：`cargo test --test watch_incremental` 4 passed，全套 0 failed，clippy clean，commits：5e7630f / c6a35dd / cf27bff / 5c756a0 / 21cbe9b / 1500c07

---

## Standard Stack

此 phase 无新增依赖。所有修改均在已有文件内完成。

| 修改文件 | 类型 | 改动 |
|----------|------|------|
| `src/cli/opts.rs` | Rust 源文件 | Watch + Validate `after_help` 字符串追加 |
| `README.md` | Markdown | 第 41 行更新 + 功能特性新增条目 + 快速入门新增段落 |
| `.planning/phases/67-prog-diag/67-VALIDATION.md` | Markdown | 新建（从 SUMMARY 重建） |
| `.planning/phases/68-init-wizard/68-VALIDATION.md` | Markdown | 新建（从 SUMMARY 重建） |
| `.planning/phases/69-watch/69-VALIDATION.md` | Markdown | 新建（从 SUMMARY 重建） |
| `.planning/phases/70-watch/70-VALIDATION.md` | Markdown | 新建（从 SUMMARY 重建） |

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| VALIDATION.md 格式 | 自行设计新格式 | 参考 Phase 01/02 VALIDATION.md 既有格式 | 一致性；planner 和 gsd 工具链依赖固定 frontmatter 字段 |
| after_help 格式 | 发明新示例风格 | 沿用 opts.rs 现有 `4空格描述 + 8空格命令` 格式 | 与其他子命令保持一致；clap 自动渲染 |

---

## Common Pitfalls

### Pitfall 1: VALIDATION.md frontmatter 字段遗漏
**What goes wrong:** 忘记将 `status: draft` 改为 `status: complete`，或 `nyquist_compliant: false` 漏改为 `true`。
**Why it happens:** 直接拷贝 Phase 01/02 VALIDATION.md 模板但未修改状态字段。
**How to avoid:** 写完每个文件后逐行核对 frontmatter 5 个字段（phase、slug、status、nyquist_compliant、wave_0_complete）。
**Warning signs:** gsd 工具链报"phase not complete"。

### Pitfall 2: after_help 字符串末尾多余换行或引号位置错误
**What goes wrong:** 追加示例后 `cargo clippy` 报告 `unexpected character` 或 clap 渲染时出现空行。
**Why it happens:** 多行字符串拼接时 `"` 位置错误，或示例段尾部多了 `\n`。
**How to avoid:** 追加后运行 `cargo clippy --all-targets -- -D warnings`，验证通过后提交。
**Warning signs:** `cargo build` 编译报错。

### Pitfall 3: README 中文引号或格式不一致
**What goes wrong:** 新增段落使用英文标点或格式风格与现有条目不一致（如用 `#` 小标题而非无标题段落）。
**Why it happens:** 写文档时未参考现有段落格式。
**How to avoid:** 新增内容前仔细阅读邻近段落格式，保持粗体标题 + em dash + 描述句的既有模式（功能特性条目），以及代码块独立分段的模式（快速入门）。

### Pitfall 4: VALIDATION.md 的 Per-Task Verification Map 与 SUMMARY 数据不匹配
**What goes wrong:** 任务 ID 编号、需求 ID、测试命令与 SUMMARY.md 记载的实际情况不符。
**Why it happens:** 多阶段（最多 4 个 plan）数据手动汇总时疏漏。
**How to avoid:** 直接从 SUMMARY.md 的 `tasks_completed` 和 `requirements-completed` 字段转录，不要重新发明。

---

## Code Examples

### after_help 追加示例的正确格式

Watch variant 追加前（行 172-175）：
```rust
after_help = "\
EXAMPLES:
    Watch and process new log files automatically:
        sqllog2db watch -c config.toml"
```

追加后目标格式：
```rust
after_help = "\
EXAMPLES:
    Watch and process new log files automatically:
        sqllog2db watch -c config.toml

    Watch in quiet mode (suitable for cron/background):
        sqllog2db watch -c config.toml --quiet"
```

Validate variant 追加后目标格式：
```rust
after_help = "\
EXAMPLES:
    Validate a configuration file:
        sqllog2db validate -c config.toml

    Validate and show detailed field information:
        sqllog2db validate -c config.toml --verbose"
```

### VALIDATION.md frontmatter 模板（完成态）

```yaml
---
phase: {N}
slug: {slug}
status: complete
nyquist_compliant: true
wave_0_complete: true
created: {phase_start_date}
updated: 2026-06-07
---
```

### README 功能特性新条目格式（参考现有"简洁的 CLI"条目风格）

```markdown
- **持续监听**：`watch` 子命令监听配置目录下的新 `.log` 文件，500ms 防抖后自动触发增量处理，Ctrl+C 退出并打印本次运行摘要（处理次数、总行数、运行时长）。
```

---

## Environment Availability

Step 2.6: SKIPPED（此 phase 为纯文档/字符串修改，无外部依赖）。

质量门禁工具已确认可用：[VERIFIED: 直接运行]

| 工具 | 可用 | 版本/结果 |
|------|------|----------|
| `cargo test` | 是 | 全套 911 passed, 2 ignored, 0 failed（当前基线） |
| `cargo clippy --all-targets -- -D warnings` | 是 | 0 warnings（当前基线） |
| `cargo fmt --check` | 是 | 0 diff（当前基线） |

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `cargo test` |
| Config file | Cargo.toml（无独立 test config）|
| Quick run command | `cargo test` |
| Full suite command | `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | Notes |
|--------|----------|-----------|-------------------|-------|
| QUAL-01 | VALIDATION.md 文件存在且 frontmatter 字段正确 | manual | `ls .planning/phases/6*/` | 纯文档，无自动化测试 |
| DOC-04 | README 包含 watch/--interactive/--quiet/--verbose 内容 | manual | `grep -c "watch\|interactive\|quiet\|verbose" README.md` | 纯文档，无自动化测试 |
| DOC-05 | opts.rs 修改通过编译/clippy/fmt | automated | `cargo clippy --all-targets -- -D warnings && cargo fmt --check` | 修改 Rust 源文件必须验证 |

### Sampling Rate

- **每任务提交后：** `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`（DOC-05 的 opts.rs 修改需要编译验证；DOC-04 和 QUAL-01 纯 Markdown 也跑一遍确认无副作用）
- **Phase gate：** 同上，确保全套绿灯

### Wave 0 Gaps

None — 此 phase 无新测试文件需要提前创建。opts.rs 修改通过现有 clippy 门禁验证；VALIDATION.md 和 README 是纯文档，无专用测试。

---

## Project Constraints (from CLAUDE.md)

[VERIFIED: 读取 ./CLAUDE.md]

| 指令 | 相关性 | 影响 |
|------|--------|------|
| `cargo clippy --all-targets -- -D warnings` 必须通过 | 高 | DOC-05 修改 opts.rs 后必须运行 |
| `cargo fmt` 必须通过 | 高 | opts.rs 修改后必须运行 |
| 函数不超过 40 行 | 低 | 此 phase 不新增函数，仅追加字符串 |
| 使用 descriptive variable names | 不适用 | 纯文档修改 |
| 提交使用 conventional commit 格式 | 高 | 每个任务提交需使用 `docs(03-N):`、`fix(03-N):` 等格式 |

---

## State of the Art

| 当前状态 | 目标状态 | 变更类型 |
|----------|----------|----------|
| Phase 67/68/69/70 无 VALIDATION.md | 各有 1 份完成态 VALIDATION.md | 新建文档 |
| README 无 watch 条目，功能特性列 4 个命令 | README 含 watch 条目，功能特性列 5 个命令 | 文档更新 |
| Watch/Validate --help 各 1 个示例 | Watch/Validate --help 各 2 个示例 | Rust 字符串追加 |

---

## Assumptions Log

此 research 中无 `[ASSUMED]` 标记的声明——所有关键事实均通过读取实际文件验证。

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| — | — | — | — |

**If this table is empty:** 所有声明均经过文件读取验证，无需用户额外确认。

---

## Open Questions

无阻塞性开放问题。唯一的 discretion 决策点（README 新段落具体位置）已由 CONTEXT.md 授权 planner 自行决定，不需要外部信息。

---

## Sources

### Primary (HIGH confidence)
- `/Users/guang/Projects/sqllog2db/src/cli/opts.rs` — 所有子命令 after_help 现状直接读取
- `/Users/guang/Projects/sqllog2db/README.md` — 全文读取，确认缺失内容和插入点
- `.planning/phases/01-watch/01-VALIDATION.md` — 格式参考样本（draft 状态）
- `.planning/phases/02-fsevents/02-VALIDATION.md` — 格式参考样本（per-task 表结构）
- `.planning/phases/67-prog-diag/67-{01,02,03}-SUMMARY.md` — Phase 67 重建数据来源
- `.planning/phases/68-init-wizard/68-{01,02}-SUMMARY.md` — Phase 68 重建数据来源
- `.planning/phases/69-watch/69-{01,02,03,04}-SUMMARY.md` — Phase 69 重建数据来源
- `.planning/phases/70-watch/70-{01,02,03}-SUMMARY.md` — Phase 70 重建数据来源
- `.planning/phases/03-doc-align/03-CONTEXT.md` — 用户决策（D-01 至 D-11）

---

## Metadata

**Confidence breakdown:**
- 修改内容（after_help/README）: HIGH — 直接读取了源文件，确认了当前状态和插入点
- VALIDATION.md 数据来源: HIGH — 12 个 SUMMARY.md 全部读取，self-check 均为 PASSED
- 格式规范: HIGH — 参考了 Phase 01/02 的实际 VALIDATION.md 文件

**Research date:** 2026-06-07
**Valid until:** 此类文档对齐工作不依赖外部服务，研究结果长期有效（代码仓库不变则有效）
