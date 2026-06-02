---
phase: 56-stats-benchmark
plan: "02"
subsystem: scanner / run / benchmark-docs
tags: [refactor, scanner, build-parser, benchmarks, docs, d-03, d-04, bench-01]
dependency_graph:
  requires: [56-01]
  provides: [scanner-build-parser, benchmarks-ci-artifact-docs]
  affects: [src/scanner.rs, src/cli/run/processor.rs, benches/BENCHMARKS.md]
tech_stack:
  added: []
  patterns: [pub(crate)-helper-fn, parser-factory-extraction, DRY-reuse]
key_files:
  created: []
  modified:
    - src/scanner.rs
    - src/cli/run/processor.rs
    - benches/BENCHMARKS.md
decisions:
  - "D-03 采用辅助函数小范围提取方案：新增 pub(crate) fn build_parser，而非完整回调改造（scanner::scan_files 回调无返回值，无法实现 'outer break 的提前终止语义）"
  - "scan_files 内部重构为调用 build_parser，保持 DRY"
  - "processor.rs 主迭代循环（'outer / 配额 / interrupted / fatal export）保留在原处，行为完全不变"
metrics:
  duration: "约 30 分钟"
  completed_date: "2026-06-02"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 3
---

# Phase 56 Plan 02: processor.rs D-03 重构与 CI Benchmark 文档 Summary

在 `scanner.rs` 新增 `pub(crate) fn build_parser` 辅助函数，完成 D-03 的 parser 创建提取；同时将 BENCHMARKS.md 末尾追加 CI Artifact 使用说明章节（D-04），并通过 stat/grep 静态断言闭环 BENCH-01 验证条件。

## Tasks Completed

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 1 | processor.rs parser 创建改用 scanner::build_parser（D-03） | 5aa43ce | src/scanner.rs, src/cli/run/processor.rs |
| 2 | BENCHMARKS.md 追加 CI Artifact 章节 + BENCH-01 静态验证（D-04） | 877c9bf | benches/BENCHMARKS.md |

## What Was Built

**src/scanner.rs（新增 build_parser）**

新增 `pub(crate) fn build_parser(file_path: &std::path::Path) -> Result<LogParser>` 辅助函数：
- 处理 non-UTF8 路径 → 返回 `Err(ParserError::InvalidPath)`
- 文件不存在/打开失败 → 返回 `Err(ParserError::InvalidPath)`
- 将 `LogParserBuilder::new` 构造与错误映射封装于此
- `scan_files` 内部重构为调用 `build_parser`（DRY，消除重复 InvalidPath 错误构造逻辑）

**src/cli/run/processor.rs（D-03 小范围提取）**

- 删除直接 `use dm_database_parser_sqllog::LogParserBuilder`
- 将第 52-58 行 `LogParserBuilder::new(file_path).build().map_err(...)` 替换为 `crate::scanner::build_parser(std::path::Path::new(file_path))?`（单行）
- `process_log_file` 函数签名 14 个参数完全不变
- 返回类型 `Result<(usize, ErrorStats)>` 完全不变
- `'outer` 循环、配额检查、interrupted 信号、fatal export 中断路径全部保留在原处
- `errors_in_file` 计数与汇总日志（第 144-146 行）完全保留

**benches/BENCHMARKS.md（D-04 新增章节）**

在 Phase 44 章节之后追加 `## CI Benchmark Artifact 使用说明`，含四个子小节：
1. **Artifact 命名规则**：CI 40 位 SHA 命名、脚本 8 位短 SHA 文件名、60 天保留期
2. **下载方式**：GitHub UI 步骤 + gh CLI 命令示例
3. **JSON 结构**：带 fenced 代码块的完整结构说明（timestamp/commit_sha/benchmarks/mean_ns/stddev_ns）
4. **手动历史对比方法**：jq 提取 + Python/bc 计算相对变化

## Verification Results

- `cargo build --release`: 成功，无警告
- `cargo test`: 全套通过（261 lib + 292 bin + 64 integration + 1 jemalloc）
- `cargo clippy --all-targets -- -D warnings`: 零告警
- `cargo fmt --check`: 零差异

**BENCH-01 静态检查：**
- `test -x scripts/collect_bench_results.sh`: 退出码 0（脚本存在且可执行）
- `grep -c "continue-on-error: true" .github/workflows/bench.yml`: 1（informational 模式已配置）

**D-03 验收条件：**
- `grep -c "pub(super) fn process_log_file" src/cli/run/processor.rs` = 1
- 签名包含 `file_path: &str`、`exporter_manager: &mut ExporterManager`、`pb: Option<&ProgressBar>` ✓
- `grep -c "Result<(usize, ErrorStats)>" src/cli/run/processor.rs` = 1
- `grep -c "crate::scanner" src/cli/run/processor.rs` = 1
- `grep -c "errors_in_file" src/cli/run/processor.rs` = 7

**D-04 验收条件：**
- `grep -cE "^## CI .*Artifact" benches/BENCHMARKS.md` = 1
- `grep -cE "bench-results-" benches/BENCHMARKS.md` = 8（>= 2）
- `grep -c "mean_ns" benches/BENCHMARKS.md` = 4（>= 1）
- `grep -c "stddev_ns" benches/BENCHMARKS.md` = 2（>= 1）
- `grep -c "gh run" benches/BENCHMARKS.md` = 2（>= 1）
- `grep -c "Phase 56" benches/BENCHMARKS.md` = 1

## Deviations from Plan

### D-03 方案选择说明

**计划原文提供两种方案：**

1. **回调闭包改造（AtomicBool/Cell）**：将 `'outer` 循环中的 `Ok(record)` 分支包装为闭包，调用 `scanner::scan_files`
2. **辅助函数小范围提取**：仅提取 parser 创建，主循环保留在 processor.rs

**评估结论：选择方案 B（辅助函数小范围提取）**

理由：`scanner::scan_files` 的回调签名为 `FnMut(&Sqllog)`（无返回值），无法从闭包内部中断外层 `scan_files` 的迭代——即使通过 `Cell<bool>` 设置 should_break 标志，`scan_files` 仍会继续遍历文件中所有记录，直到 EOF。这会改变以下行为的语义：

- **配额截止**（`if records_in_file >= remaining { break 'outer; }`）：会导致超出配额继续消费记录
- **中断信号**（`interrupted.load(Ordering::Relaxed) { break 'outer; }`）：响应延迟到 scan_files 返回
- **fatal export 中断**（`file_stats.set_fatal(...); break 'outer;`）：致命 export 错误后继续解析文件

上述语义变化违反"run 命令行为不变"的核心约束。方案 B 仅改变第 52-58 行的 parser 创建位置，改动量 8 行，行为完全不变，符合计划中的判断标准（"若评估两个方案都需要 >50 行改动或破坏现有测试"排除标准）。

**已在 processor.rs 完成的 D-03 提取：** parser 创建（InvalidPath 错误映射）已移入 `scanner::build_parser`，`crate::scanner` 现已出现于 processor.rs。

**stats 侧（D-01/D-02）：** 已在 Plan 01 完成（`scan_files_into_accumulator` 调用 `scanner::scan_files`）。

### Auto-fixed Issues

无自动修复问题。

### clippy doc_markdown 警告修复（Rule 3 — 阻塞性格式问题）

- **发现于：** Task 1 commit pre-commit hook
- **问题：** 新增 `build_parser` 注释中 `InvalidPath` 未加反引号，clippy 报 `doc_markdown` 告警（-D warnings 模式下视为错误）
- **修复：** 将注释中的 `InvalidPath` 改为 `` `InvalidPath` ``
- **文件：** src/scanner.rs
- **Commit：** 5aa43ce（含修复）

## Threat Surface Scan

本计划未引入新的网络端点、认证路径或文件访问模式。`build_parser` 复用已有的 `LogParserBuilder::new` 路径，trust boundary 与 T-56-05 完全对齐（仅 process_log_file 作用域内使用，无跨线程访问）。无超出 threat model 的新暴露面。

## Known Stubs

无。

## Self-Check: PASSED

- [x] src/scanner.rs 存在 build_parser 函数
- [x] src/cli/run/processor.rs 使用 crate::scanner::build_parser
- [x] benches/BENCHMARKS.md 末尾有 CI Artifact 章节
- [x] 5aa43ce commit 存在（refactor Task 1）
- [x] 877c9bf commit 存在（docs Task 2）
- [x] cargo test 全套通过
- [x] clippy 零告警
- [x] fmt 零差异
- [x] BENCH-01：scripts/collect_bench_results.sh 可执行，bench.yml continue-on-error: true
