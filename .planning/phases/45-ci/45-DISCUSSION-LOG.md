# Phase 45: 并行扩展与 CI 基准集成 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-24
**Phase:** 45-并行扩展与 CI 基准集成
**Areas discussed:** SQLite 并行策略, CI Benchmark 格式

---

## SQLite 并行策略

| Option | Description | Selected |
|--------|-------------|----------|
| 多文件跨文件并行解析＋SQLite WAL | 对标 PERF-03：多输入文件时，各文件并行解析（rayon），内存内合并结果后单线程写入 SQLite（WAL 模式）。避免 SQLite 写入争用。 | ✓ |
| SQLite 批量并行写入（channel + writer thread） | 解析线程 pool + 单个 writer thread。复杂度更高，但写入吸能能跟上解析速度。 | |
| 只做跨文件并行解析，SQLite 不变 | CSV 已经有并行路径。将并行能力扩展到 SQLite 导出时内存内合并结果 + 单线程写入即可满足 PERF-03。 | |

**User's choice:** 多文件跨文件并行解析＋SQLite WAL
**Notes:** 与 CSV 并行路径（process_csv_parallel）设计模式一致，WAL 模式提升 SQLite 读写并发性。

---

## CI Benchmark 报告格式

| Option | Description | Selected |
|--------|-------------|----------|
| JSON（critcmp 兼容） | 保存 criterion 输出的 JSON，用 critcmp 或手写脚本对比历史基线。结构化数据，不依赖 criterion 的 HTML 生成。 | ✓ |
| HTML（criterion 内置生成） | 直接将 target/criterion/ 上传为 artifact。可视化好，但文件大，历史对比不如 JSON 灵活。 | |
| JSON + HTML 全部保存 | 两层都要：JSON 用于程序化对比，HTML 用于人工阅读。Artifact 体积大一些。 | |

**User's choice:** JSON（critcmp 兼容）
**Notes:** 优先结构化数据，便于 CI 脚本自动化对比。

---

## Claude's Discretion

- CI workflow 触发条件（PR 还是也包含 push to main）
- artifact retention 天数（建议 30-90 天）
- critcmp vs 自定义比较脚本

## Deferred Ideas

- critcmp PR comment bot → 超出本 milestone 范围
- AsyncLogParser tokio 异步 SQLite 写入 → 过度工程
