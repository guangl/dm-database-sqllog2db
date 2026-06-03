# Phase 59: cli/run 与 exporter/pipeline 结构整理 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-03
**Phase:** 59-cli-run-exporter-pipeline
**Areas discussed:** process_log_file 拆分, process_csv_parallel 拆分, 并行路径重复代码, run_sequential + FilterProcessor 小函数

---

## process_log_file 拆分

| Option | Description | Selected |
|--------|-------------|----------|
| extract process_one_record | 把整个 Ok(record) 分支提取为 process_one_record，主函数主循环只剩 ~30 行 | |
| extract setup + teardown | 不动主循环，仅提取初始化和收尾，分拆收益小 | |
| extract normalize_and_export | 把 normalize+export+错误处理提取，pass 判断留在调用方 | ✓ |

**User's choice:** normalize→export→err处理（pass判断在外面）
**Notes:** 同时提取 setup_progress_bar（开头进度条初始化）和 log_file_result（结尾 warn+info+pb）。主函数保留 params_buffer.clear、include_pm、build_parser 和主循环骨架。

---

## process_csv_parallel 拆分

| Option | Description | Selected |
|--------|-------------|----------|
| extract run_parallel_tasks + collect_parallel_results | 并行 map 单独提取，结果收集单独提取，主函数 ~25 行 | ✓ |
| extract setup_parts_dir + collect_results_or_cleanup | 只拆临时目录准备和清理，并行 map 不动 | |

**User's choice:** extract run_parallel_tasks + collect_parallel_results
**Notes:** parts_dir setup (~25 行) 也一并提取为 setup_parts_dir，主函数最终为 4 个函数调用。

---

## 并行路径重复代码

| Option | Description | Selected |
|--------|-------------|----------|
| 保持分离，只提取公共 process_record 函数 | 两个模块各自保留结构，只共享单记录处理逻辑 | |
| 合并为共享 collect_log_file，返回 Vec | 新建 collector.rs，两个并行路径均调用此函数 | ✓ |

**User's choice:** 合并为共享 collect_log_file，返回 Vec
**Notes:** 新建 `src/cli/run/collector.rs`。CSV 并行路径由"流式写临时文件"改为"先 collect Vec 再写"，内存占用略增。

---

## run_sequential + FilterProcessor 小函数

| Option | Description | Selected |
|--------|-------------|----------|
| extract run_file_loop（循环体） | 把 for 循环体提取，主函数只剩 exporter lifecycle + 调用，~20 行 | ✓ |
| 不拆，只是不内联 | run_sequential 仅超出 12 行，不强制拆分 | |

**User's choice:** extract run_file_loop（循环体）

**FilterProcessor::from_feature：**

| Option | Description | Selected |
|--------|-------------|----------|
| extract build_include_groups + build_exclude_groups | 各 7 个 build_or_group 调用分别提取 | ✓ |
| 忽略（仅比 40 行多 3 行） | 43 行极小超载，不拆 | |

**User's choice:** extract build_include_groups + build_exclude_groups

---

## Claude's Discretion

- `concat_csv_parts`（48 行）是否拆分：未明确讨论，规划时可视具体情况决定。

## Deferred Ideas

- `ParallelRunConfig` struct 消除 too_many_arguments — 属于接口重构，超出本阶段范围。
- `ExporterManager::from_config` 中 CSV/SQLite 配置重复赋值 — 行数未超标，暂不处理。
