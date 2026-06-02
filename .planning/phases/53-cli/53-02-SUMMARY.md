---
phase: 53-cli
plan: "02"
subsystem: cli
tags: [stats, cli, time-range, priority-merge, clap]
dependency_graph:
  requires:
    - 53-01 (StatsConfig 结构体 + Config.stats 字段)
  provides:
    - CLI --from / --to / Option<u32> top 参数（src/cli/opts.rs）
    - handle_stats 优先级合并逻辑（src/cli/stats/mod.rs）
    - main.rs Stats 分发分支接入新签名
  affects:
    - src/cli/opts.rs（Stats 变体字段扩展）
    - src/cli/stats/mod.rs（handle_stats 签名重写 + 8 个单元测试）
    - src/main.rs（Stats 分发分支解构 from/to）
tech_stack:
  added: []
  patterns:
    - Option<T>.or(other).unwrap_or(default) 优先级链式合并
    - 私有 merge_stats_options 函数将合并算法与 I/O 逻辑解耦，便于单元测试
    - clap value_parser range(1..) 保持 --top 范围校验同时允许 None 默认
key_files:
  created: []
  modified:
    - src/cli/opts.rs
    - src/cli/stats/mod.rs
    - src/main.rs
decisions:
  - 三个任务合并为单次提交以通过 pre-commit hook（clippy + tests 全套检查）
  - merge_stats_options 抽取为私有函数，handle_stats 主体保持在 40 行以内
  - --top 改为 Option<u32> 后 clap range(1..) 仍保留，--top 0 仍被拦截
metrics:
  duration: "约 20 分钟"
  completed_date: "2026-06-01"
  tasks_completed: 3
  files_changed: 3
---

# Phase 53 Plan 02: Stats CLI 时间段参数与优先级合并 Summary

## One-liner

`stats` 子命令新增 `--from`/`--to` 时间段 CLI 参数，`--top` 改为 `Option<u32>`，`handle_stats` 提取 `merge_stats_options` 实现 CLI > config > 默认的三级优先级合并，8 个单元测试 + 11 个集成测试全部通过。

## What Was Built

### Task 1: src/cli/opts.rs（修改）

扩展 `Commands::Stats` 变体：

- `top: u32` 改为 `top: Option<u32>`，移除 `default_value = "20"`（保留 `range(1..)` 拦截 `--top 0`）
- 新增 `from: Option<String>`，`#[arg(long = "from", value_name = "DATETIME", help = "...YYYY-MM-DD...")]`
- 新增 `to: Option<String>`，同 from 的属性模式
- `after_help` 追加时间段过滤示例行

### Task 2: src/cli/stats/mod.rs（重写）

- 提取私有 `merge_stats_options(cfg, cli_top, cli_from, cli_to) -> (u32, Option<String>, Option<String>)`：
  - `effective_top = cli_top.or(cfg.stats.top).unwrap_or(20)`
  - `effective_from = cli_from.or_else(|| cfg.stats.from.clone())`
  - `effective_to = cli_to.or_else(|| cfg.stats.to.clone())`
- `handle_stats` 新签名 `(cfg, top: Option<u32>, from: Option<String>, to: Option<String>)`
- 合并结果写入 `merged_cfg`（`cfg.clone()` 副本），传给 `run_stats`
- `log::info!` 输出合并后的 `top/from/to`（满足集成测试 top=20/top=5 日志断言）
- 8 个单元测试：2 个迁移（原有）+ 3 个行为测试（handle_stats 层）+ 3 个纯逻辑测试（merge_stats_options 层）

### Task 3: src/main.rs（修改）

- Stats match 分支解构：`Commands::Stats { config, top, from, to }`
- 调用改为：`cli::stats::handle_stats(&cfg, *top, from.clone(), to.clone())`

## Deviations from Plan

### Commit 策略（同 Plan 01）

三个任务合并为一次提交 `ad95b1f`。原因：pre-commit hook 运行 `cargo clippy --all-targets -- -D warnings` 和 `cargo test`。Task 1 修改 opts.rs 类型后，main.rs 的 `*top: &u32` 和 `handle_stats(&cfg, *top)` 立即产生编译错误，Task 2 修改 handle_stats 签名后 main.rs 调用也不匹配。三个文件存在直接类型依赖，无法独立通过 hook 检查。

提交消息中已明确区分 Task 1/2/3 的贡献。

## Verification Results

```
cargo build --release               -- 通过
cargo test --lib cli::stats::       -- 8/8 通过（新增 3 个合并矩阵测试 + 迁移 2 个）
cargo test --test integration -- stats  -- 11/11 通过（S1-S6 全部通过 + 5 个功能测试）
cargo test (全量)                   -- 272/272 全部通过（零回归）
cargo clippy --all-targets -- -D warnings  -- 通过（零警告）
cargo fmt --check                   -- 通过
sqllog2db stats --help              -- 含 --from / --to / YYYY-MM-DD 三个关键子串
```

## Key Decisions

1. `merge_stats_options` 作为私有函数独立于 `handle_stats`：使三个纯逻辑断言测试（返回元组等于具体值）可以直接覆盖合并算法，不依赖 I/O（tempfile/run_stats）
2. `--top` 保留 `range(1..)`：移除 `default_value` 后 clap 的 `value_parser` 仍然在用户显式传入 `--top 0` 时拦截，行为与 v1.13 一致（S5 集成测试继续通过）
3. `handle_stats` 使用 `cfg.clone()` 构造 `merged_cfg` 传给 `run_stats`，而非修改调用方传入的 `&Config`：符合 D-06 "run_stats 只读 cfg" 设计原则

## Threat Flags

无新增网络端点、认证路径或文件访问模式。仅扩展了 CLI 参数解析和内存中的配置合并逻辑。

## Self-Check: PASSED

| Check | Result |
|-------|--------|
| src/cli/opts.rs 含 from: Option<String> | FOUND（第 157 行） |
| src/cli/opts.rs 含 to: Option<String> | FOUND（第 164 行） |
| src/cli/opts.rs 含 top: Option<u32> | FOUND（第 150 行） |
| src/cli/opts.rs Stats 变体内无 default_value = "20" | CONFIRMED |
| src/cli/stats/mod.rs 含 pub fn handle_stats | FOUND（第 21 行） |
| src/cli/stats/mod.rs 含 unwrap_or(20) | FOUND（第 10 行） |
| src/cli/stats/mod.rs 含 from.or_else | FOUND（第 11 行） |
| src/cli/stats/mod.rs 含 to.or_else | FOUND（第 12 行） |
| src/main.rs Stats 分支含 from, to 解构 | FOUND（第 178-186 行） |
| src/main.rs 含 handle_stats(&cfg, *top, from.clone(), to.clone()) | FOUND（第 188 行） |
| commit ad95b1f 存在 | FOUND |
| SUMMARY.md 存在 | FOUND |
