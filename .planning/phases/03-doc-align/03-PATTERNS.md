# Phase 3: 文档与验证对齐 - Pattern Map

**Mapped:** 2026-06-07
**Files analyzed:** 6 (2 modified + 4 new)
**Analogs found:** 6 / 6

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/cli/opts.rs` | config/CLI | request-response | `src/cli/opts.rs` (existing file, append only) | exact |
| `README.md` | doc | — | `README.md` (existing file, extend) | exact |
| `.planning/phases/67-prog-diag/67-VALIDATION.md` | doc/validation | — | `.planning/phases/02-fsevents/02-VALIDATION.md` | exact |
| `.planning/phases/68-init-wizard/68-VALIDATION.md` | doc/validation | — | `.planning/phases/02-fsevents/02-VALIDATION.md` | exact |
| `.planning/phases/69-watch/69-VALIDATION.md` | doc/validation | — | `.planning/phases/02-fsevents/02-VALIDATION.md` | exact |
| `.planning/phases/70-watch/70-VALIDATION.md` | doc/validation | — | `.planning/phases/02-fsevents/02-VALIDATION.md` | exact |

---

## Pattern Assignments

### `src/cli/opts.rs` — Watch/Validate `after_help` 追加（DOC-05）

**Analog:** 同文件现有 `Stats` variant 的多示例格式（lines 126-135）

**现有单示例格式** (Watch variant, lines 172-175):
```rust
after_help = "\
EXAMPLES:
    Watch and process new log files automatically:
        sqllog2db watch -c config.toml"
```

**目标双示例格式**（追加第 2 个示例，示例之间空行分隔，末尾引号紧跟命令行）:
```rust
after_help = "\
EXAMPLES:
    Watch and process new log files automatically:
        sqllog2db watch -c config.toml

    Watch in quiet mode (suitable for cron/background):
        sqllog2db watch -c config.toml --quiet"
```

**同样模式适用于 Validate variant** (lines 107-110):
```rust
// 追加前（单示例）
after_help = "\
EXAMPLES:
    Validate a configuration file:
        sqllog2db validate -c config.toml"

// 追加后（双示例）
after_help = "\
EXAMPLES:
    Validate a configuration file:
        sqllog2db validate -c config.toml

    Validate and show detailed field information:
        sqllog2db validate -c config.toml --verbose"
```

**格式规范**（来自 Stats variant lines 126-135 的既有模式）:
- 描述行：4 空格缩进，以 `:` 结尾
- 命令行：8 空格缩进
- 示例之间：1 个空行
- 末尾：字符串直接以 `"` 结束，无多余换行

**验证命令:** `cargo clippy --all-targets -- -D warnings && cargo fmt --check`

---

### `README.md` — watch/init --interactive/--quiet/--verbose 内容（DOC-04）

**Analog:** 同文件现有功能特性条目格式（README.md line 41）

**D-05 修改点** — 第 41 行"简洁的 CLI"条目，从 4 个命令更新为 5 个：
```markdown
<!-- 修改前 -->
- **简洁的 CLI**：`init`（生成配置）、`validate`（校验）、`run`（执行导出）、`stats`（统计分析）四个命令。

<!-- 修改后 -->
- **简洁的 CLI**：`init`（生成配置）、`validate`（校验）、`run`（执行导出）、`stats`（统计分析）、`watch`（持续监听）五个命令。
```

**D-06 新增功能特性条目**（功能特性区域"配置与性能"节末尾追加，使用相同的粗体标题 + em dash 格式）:
```markdown
- **持续监听**：`watch` 子命令监听配置目录下的新 `.log` 文件，500ms 防抖后自动触发增量处理，Ctrl+C 退出并打印本次运行摘要（处理次数、总行数、运行时长）。
```

**D-07 快速入门段落格式**（参考 README.md lines 105-129 的既有代码块风格）:

插入位置：`快速入门`章节（README.md line 105 区域），在现有 `stats` 示例块之后，`详细用法参见` 链接之前。

新增三段，每段使用独立 bash fenced code block：
```markdown
持续监听新 `.log` 文件（按 Ctrl+C 停止并打印摘要）：

```bash
sqllog2db watch -c config.toml
```

交互式向导生成配置文件：

```bash
sqllog2db init --interactive
```

进度输出控制：`-q`/`--quiet` 抑制非错误输出（适合后台运行），`-v`/`--verbose` 显示每文件详情：

```bash
sqllog2db run -c config.toml --quiet
sqllog2db run -c config.toml --verbose
```
```

---

### `.planning/phases/67-prog-diag/67-VALIDATION.md`（QUAL-01）

**Analog:** `.planning/phases/02-fsevents/02-VALIDATION.md`

**Frontmatter 模板**（完成态，参考 Phase 02 结构，status 改为 complete）:
```yaml
---
phase: 67
slug: prog-diag
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-05
updated: 2026-06-07
---
```

**正文结构**（参考 Phase 01/02 VALIDATION.md 结构，已完成阶段省略 Wave 0 Requirements 和 Manual-Only 两节）:

1. `## Test Infrastructure` — Framework、Config file、Quick run command、Full suite command
2. `## Sampling Rate` — 三个层级的采样命令
3. `## Per-Task Verification Map` — 表格（Task ID / Plan / Wave / Requirement / Threat Ref / Secure Behavior / Test Type / Automated Command / File Exists / Status）
4. `## Validation Sign-Off` — 所有条目已通过

**Per-Task Verification Map 数据来源**（从 SUMMARY.md self-check 转录）:

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | Status |
|---------|------|------|-------------|-----------|-------------------|--------|
| 67-01-01 | 01 | 1 | PROG-01, PROG-02 | unit | `cargo test --lib cli::run::tests` | ✅ complete |
| 67-02-01 | 02 | 2 | DIAG-01, DIAG-02 | unit | `cargo test --lib` | ✅ complete |
| 67-03-01 | 03 | 3 | PROG-03, DIAG-03 | unit+integration | `cargo test --lib` | ✅ complete |

**Validation Sign-Off 格式**（参考 Phase 02 Sign-Off 风格，但全部标 ✅）:
```markdown
## Validation Sign-Off

- [x] All tasks self-check: PASSED（依据：67-01/02/03-SUMMARY.md）
- [x] `cargo test --lib` 344 passed（67-03-SUMMARY.md 记录）
- [x] `cargo clippy --all-targets -- -D warnings` clean
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** complete（self-check: PASSED 已记录于所有 SUMMARY.md）
```

---

### `.planning/phases/68-init-wizard/68-VALIDATION.md`（QUAL-01）

**Analog:** `.planning/phases/02-fsevents/02-VALIDATION.md`（同上）

**Frontmatter:**
```yaml
---
phase: 68
slug: init-wizard
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-05
updated: 2026-06-07
---
```

**Per-Task Verification Map 数据来源**（从 68-01/02-SUMMARY.md 转录）:

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | Status |
|---------|------|------|-------------|-----------|-------------------|--------|
| 68-01-01 | 01 | 1 | INIT-01, INIT-02, INIT-03 | unit | `cargo test --lib` (12 unit tests) | ✅ complete |
| 68-02-01 | 02 | 2 | INIT-01, INIT-02, INIT-03 | integration | `cargo test` (6 e2e assert_cmd tests) | ✅ complete |

---

### `.planning/phases/69-watch/69-VALIDATION.md`（QUAL-01）

**Analog:** `.planning/phases/02-fsevents/02-VALIDATION.md`（同上）

**Frontmatter:**
```yaml
---
phase: 69
slug: watch
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-05
updated: 2026-06-07
---
```

**Per-Task Verification Map 数据来源**（从 69-01/02/03/04-SUMMARY.md 转录）:

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | Status |
|---------|------|------|-------------|-----------|-------------------|--------|
| 69-01-01 | 01 | 1 | WATCH-01 | unit | `cargo test --lib` | ✅ complete |
| 69-02-01 | 02 | 2 | WATCH-01, WATCH-05, WATCH-06 | unit+integration | `cargo test` | ✅ complete |
| 69-03-01 | 03 | 3 | WATCH-01, WATCH-02, WATCH-05, WATCH-06 | integration | `cargo test` (4 e2e tests) | ✅ complete |
| 69-04-01 | 04 | 4 | WATCH-02, WATCH-05 | unit | `cargo test --lib` (4 unit tests) | ✅ complete |

**注：** 69-03-SUMMARY.md 记录 852 passed, 2 ignored；2 个 ignore 是 macOS stdin-pipe 问题，已在 Phase 70 修复。

---

### `.planning/phases/70-watch/70-VALIDATION.md`（QUAL-01）

**Analog:** `.planning/phases/02-fsevents/02-VALIDATION.md`（同上）

**Frontmatter:**
```yaml
---
phase: 70
slug: watch
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-06
updated: 2026-06-07
---
```

**Per-Task Verification Map 数据来源**（从 70-01/02/03-SUMMARY.md 转录）:

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | Status |
|---------|------|------|-------------|-----------|-------------------|--------|
| 70-01-01 | 01 | 1 | WATCH-04（基础设施） | unit | `cargo test --lib` (5 unit tests) | ✅ complete |
| 70-02-01 | 02 | 2 | WATCH-03, WATCH-04 | unit | `cargo test --lib` (4 unit tests) | ✅ complete |
| 70-03-01 | 03 | 3 | WATCH-03, WATCH-04 | integration | `cargo test --test watch_incremental` (4 tests) | ✅ complete |

---

## Shared Patterns

### VALIDATION.md frontmatter 字段一致性
**Source:** `.planning/phases/02-fsevents/02-VALIDATION.md` lines 1-9
**Apply to:** 所有 4 个新建 VALIDATION.md
```yaml
---
phase: {N}          # 整数
slug: {slug}         # 连字符格式
status: complete     # 已完成阶段固定为 complete（非 draft）
nyquist_compliant: true   # 已完成阶段固定为 true
wave_0_complete: true     # 已完成阶段固定为 true
created: {phase_start_date}
updated: 2026-06-07
---
```
**关键陷阱：** 不要直接拷贝 Phase 01/02 的 `status: draft` 和 `nyquist_compliant: false`，这两个字段必须改为完成态值。

### Per-Task Verification Map 表格结构
**Source:** `.planning/phases/02-fsevents/02-VALIDATION.md` lines 40-51
**Apply to:** 所有 4 个新建 VALIDATION.md
```markdown
| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| {phase}-{plan}-01 | {plan} | {wave} | {req_id} | — | N/A | unit/integration | `cargo test ...` | ✅ | ✅ complete |
```
已完成阶段的 File Exists 和 Status 均标 ✅。

### `after_help` 多示例字符串格式
**Source:** `src/cli/opts.rs` lines 126-135（Stats variant，已有 3 个示例）
**Apply to:** Watch variant (line 172)、Validate variant (line 107)

格式规则：
- 首行：`after_help = "\`（反斜杠表示字符串延续）
- `EXAMPLES:` 行无缩进
- 描述行：4 空格缩进，行尾冒号
- 命令行：8 空格缩进
- 示例间分隔：1 个空行
- 末尾：最后一个命令行后直接 `"`（无多余换行）

### README 功能特性条目格式
**Source:** `README.md` line 41（"简洁的 CLI"条目）
**Apply to:** D-06 新增"持续监听"条目
```markdown
- **{粗体标题}**：{描述句，句末句号}
```

---

## No Analog Found

无——所有文件都有精确的既有模式可参考。

---

## Metadata

**Analog search scope:** `src/cli/opts.rs`、`README.md`、`.planning/phases/01-watch/`、`.planning/phases/02-fsevents/`、`.planning/phases/67-prog-diag/`（SUMMARY.md）
**Files scanned:** 8（opts.rs、README.md、01-VALIDATION.md、02-VALIDATION.md、67-01/02/03-SUMMARY.md 各 1）
**Pattern extraction date:** 2026-06-07
