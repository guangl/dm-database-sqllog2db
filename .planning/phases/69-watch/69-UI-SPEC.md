---
phase: 69
slug: watch-core-framework
status: draft
shadcn_initialized: false
preset: none
created: 2026-06-06
tool_type: rust-cli-terminal
---

# Phase 69 — Terminal UI Design Contract

> 本 Phase 是 Rust CLI 工具，不含 Web 前端。
> UI 合约约束的是终端输出行为：indicatif spinner 状态行 + Ctrl+C 退出摘要。
> 所有字段均由上游 CONTEXT.md D-04/D-05/D-06/D-12 锁定，executor 不得偏离。

---

## Design System

| Property | Value |
|----------|-------|
| Tool | none (Rust CLI — no web design system) |
| Preset | not applicable |
| Component library | indicatif 0.18 (已在 Cargo.toml:46) |
| Icon library | Unicode spinner frames via indicatif tick_chars |
| Font | Terminal default (not controlled by application) |

---

## Spacing Scale

终端 UI 不使用像素间距系统。以下约束替代标准间距合约：

| Element | Layout Rule |
|---------|-------------|
| 状态行字段分隔符 | ` \| ` (空格 + 竖线 + 空格) |
| 摘要行 | 单行，各字段用逗号 + 空格分隔 |
| spinner 与 wide_msg 之间 | 单个空格（由 indicatif template 控制） |

Exceptions: none

---

## Typography

终端 UI 字体由操作系统终端控制，应用不干预。以下约束替代标准排版合约：

| Role | Rule | Source |
|------|------|--------|
| 状态行 | 纯文本，无 ANSI 富文本修饰（spinner 颜色除外） | CONTEXT D-04 |
| spinner 颜色 | `.cyan`（indicatif 颜色修饰符） | CONTEXT D-04 |
| wide_msg | 默认终端前景色（无颜色修饰） | CONTEXT D-04 |
| 最终摘要 | 纯文本，无颜色修饰 | CONTEXT D-12 |
| 非 TTY 模式 | indicatif 自动禁用 spinner（无需代码判断） | indicatif 默认行为 |

---

## Color

| Role | Value | Usage |
|------|-------|-------|
| Spinner accent | `.cyan` (indicatif modifier) | spinner 字符本身，仅此一处 |
| Status text | terminal default | wide_msg 全部内容 |
| Summary text | terminal default | Ctrl+C 后打印的最终摘要 |
| Destructive | not applicable | Phase 69 无破坏性操作 |

Accent reserved for: spinner 旋转字符（`{spinner:.cyan}`）——仅此一处，wide_msg 内容不着色。

---

## Terminal Output Streams

| Output | Stream | Rationale |
|--------|--------|-----------|
| ProgressBar (状态行) | stderr — `ProgressDrawTarget::stderr()` | CONTEXT D-05：不干扰 stdout 导出数据 |
| handle_run 内部进度条 | stderr (现有行为不变) | 现有模式维持不变 |
| 最终摘要 (Ctrl+C) | stderr — `eprintln!` | CONTEXT D-12 |
| 导出数据 (CSV stdout 路径) | stdout | 与现有 run 命令行为一致 |
| 错误日志 | 配置的 error log 文件 | 与现有 run 命令行为一致 |

---

## Spinner Component Contract

来源：CONTEXT D-04（锁定，不可修改）

```
ProgressBar::new_spinner()
  .with_style(
      ProgressStyle::with_template("{spinner:.cyan} {wide_msg}")
          .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")   // 与现有 run 命令一致
  )
  .with_draw_target(ProgressDrawTarget::stderr())

pb.enable_steady_tick(Duration::from_millis(80))   // 与现有 run 命令一致
```

**注意：** watch 使用 `new_spinner()`（不设 len），而非 `new(total_files)`，因为触发次数不可预知。

---

## Status Line States

来源：CONTEXT D-04、D-06（锁定）

### State 1 — 启动等待状态

触发时机：`handle_watch` 启动，`notify::RecommendedWatcher` 初始化完成后立即显示。

```
{spinner:.cyan} watching {paths} | waiting for new .log files...
```

- `{paths}`：`cfg.sqllog.inputs` 中所有路径，以 `, ` 连接；多于 3 个路径时截断为 `{n} directories`
- 此状态通过 `pb.set_message(...)` 设置

### State 2 — 活跃监听状态

触发时机：每次成功处理一个触发事件后更新。

```
{spinner:.cyan} watching {dir} | triggers: {n} | processed: {rows} rows | last: {elapsed_since_last}
```

字段定义：

| 字段 | 类型 | 说明 |
|------|------|------|
| `{dir}` | String | 触发事件的文件所在目录（`path.parent()` 显示名） |
| `{n}` | u64 | 累计触发次数（每次新文件触发 +1） |
| `{rows}` | u64 | 累计已导出行数（`total_stats.records_exported` 累加） |
| `{elapsed_since_last}` | String | 距上次触发的经过时间，使用 `indicatif::HumanDuration` 或 `{elapsed}` 内置格式 |

- 此状态通过 `pb.set_message(...)` 更新，每次触发处理完成后调用一次

### State 3 — 无状态（非 TTY）

触发时机：`stderr` 不是终端时（如管道、脚本）。

indicatif 的 `ProgressDrawTarget::stderr()` 自动检测 TTY 状态：非 TTY 时 spinner 不渲染，`set_message` 调用无输出。无需应用层额外判断。

---

## Final Summary Contract

来源：CONTEXT D-12、SPECIFICS 章节（锁定）

触发时机：`interrupted.load(Relaxed) == true`（Ctrl+C 信号），跳出 watch loop 后执行。

执行顺序（严格按此顺序）：
1. `pb.finish_and_clear()` — 清除状态行
2. `eprintln!("{summary}")` — 打印最终摘要到 stderr

摘要格式（精确字符串）：

```
Watch stopped. Triggers: {n}, total processed: {rows} rows, elapsed: {hh:mm:ss}
```

字段定义：

| 字段 | 类型 | 计算方式 |
|------|------|---------|
| `{n}` | u64 | 累计触发次数 |
| `{rows}` | u64 | `total_stats.records_exported` 累计值 |
| `{hh:mm:ss}` | String | `start.elapsed()` 格式化为 `HH:MM:SS`，使用整数除法（不引入 chrono）|

退出码：0（`return Ok(None)`）

---

## Copywriting Contract

| Element | Copy | Source |
|---------|------|--------|
| 启动等待消息 | `watching {paths} \| waiting for new .log files...` | CONTEXT D-06 |
| 活跃状态消息 | `watching {dir} \| triggers: {n} \| processed: {rows} rows \| last: {elapsed}` | CONTEXT D-04/SPECIFICS |
| 最终摘要 | `Watch stopped. Triggers: {n}, total processed: {rows} rows, elapsed: {hh:mm:ss}` | CONTEXT D-12/SPECIFICS |
| 空状态（零触发退出） | 与最终摘要格式一致，`Triggers: 0, total processed: 0 rows` | 默认行为 |
| 错误（notify 初始化失败） | 复用现有 `Error` 体系，`eprintln!` 输出后 `return Err(...)` | 现有错误处理模式 |
| Ctrl+C handler 冲突 | 不适用（ctrlc handler 复用 run 命令相同模式，不新增 UI 文案） | CONTEXT D-10 |

Destructive actions: none（Phase 69 无破坏性操作，无需确认 UI）

---

## Interaction Contract

| User Action | System Response |
|-------------|----------------|
| 启动 `sqllog2db watch -c config.toml` | 立即显示 State 1 状态行 |
| 向监听目录新增 `.log` 文件 | 2 秒内触发处理，完成后切换到 State 2 |
| 再次新增 `.log` 文件 | 累计计数器递增，状态行更新 |
| 按 Ctrl+C | `interrupted` 置 true，loop 退出，State 3 流程（clear + summary） |
| 非 TTY 运行（如脚本） | spinner 不显示，摘要仍输出到 stderr |

---

## Component Inventory

| Component | Crate | Already in Cargo.toml | Notes |
|-----------|-------|----------------------|-------|
| `ProgressBar::new_spinner()` | indicatif 0.18 | Yes (line 46) | watch 专用，独立于 run 的 `new(len)` bar |
| `ProgressDrawTarget::stderr()` | indicatif 0.18 | Yes | 状态行写入 stderr |
| `ProgressStyle::with_template` | indicatif 0.18 | Yes | template: `"{spinner:.cyan} {wide_msg}"` |
| `Arc<AtomicBool>` + `ctrlc::set_handler` | std + ctrlc 3 | Yes (line 43) | 直接复用 src/main.rs:166-169 模式 |
| `RecommendedWatcher` + `mpsc::channel` | notify 6 | No — add to Cargo.toml | CONTEXT D-01 |

---

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| crates.io (notify = "6") | RecommendedWatcher | not applicable — crates.io Rust crate, not shadcn registry |
| shadcn | none | not applicable — Rust CLI project |

---

## Checker Sign-Off

- [ ] Dimension 1 Copywriting: PASS
- [ ] Dimension 2 Visuals: PASS
- [ ] Dimension 3 Color: PASS
- [ ] Dimension 4 Typography: PASS
- [ ] Dimension 5 Spacing: PASS
- [ ] Dimension 6 Registry Safety: PASS

**Approval:** pending
