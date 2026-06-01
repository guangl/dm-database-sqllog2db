# Phase 50-52: v1.13 SQL 统计分析 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-01
**Phase:** 50/51/52 — v1.13 SQL 统计分析（里程碑联合讨论）
**Areas discussed:** 标准化模块位置与方式, stats 子命令 config 需求, TOP-N 聚合与内存策略, 两张统计表的输出策略

---

## 标准化模块位置（Phase 50）

| Option | Description | Selected |
|--------|-------------|----------|
| `src/stats/` 新目录 | stats 子命令所有逻辑集中局部，不污染 pipeline/ | ✓ |
| `src/pipeline/sql_normalize.rs` | 放在 pipeline/ 目录下，和现有 normalizer.rs 相邻 | |
| `src/sql_normalizer.rs` 顶层 | 独立顶层模块，最简单但后续统计逻辑没地方放 | |

**User's choice:** `src/stats/` 新目录
**Notes:** 现有 `normalizer.rs` 是参数绑定替换器，职责完全不同，放在一起会混乱。

---

## 标准化实现方式（Phase 50）

| Option | Description | Selected |
|--------|-------------|----------|
| 字符扫描状态机 | 一次遍历，对转义引号和数字边界处理更精确，无需 regex 依赖 | ✓ |
| regex 替换 | 两个模式替换，代码简洁但对 `''` 转义引号和内嵌引号处理较弱 | |

**User's choice:** 字符扫描状态机
**Notes:** regex crate 虽已在依赖中，但状态机在边界情况处理上更可靠。

---

## stats 子命令 config 需求（Phase 51）

| Option | Description | Selected |
|--------|-------------|----------|
| 与 run 相同：完整 config | 读 [sqllog]+[csv/sqlite]，复用 load_config | ✓ |
| 仅读 [sqllog] + exporter，忽略 [filter] | 强调差异，但读全量 config 也无害 | |

**User's choice:** 完整 config（同 run）
**Notes:** 简单一致。[filter] 节存在时静默忽略。

---

## stats config 不存在时（Phase 51）

| Option | Description | Selected |
|--------|-------------|----------|
| 报错退出 | 与 run 一贯，stats 需要知道读哪些文件、写到哪里 | ✓ |
| 回落默认配置 | 和 run 命令一样，但默认配置没有 exporter 会失败 | |

**User's choice:** 报错退出

---

## stats 日志初始化（Phase 51）

| Option | Description | Selected |
|--------|-------------|----------|
| 和 run 相同：完整日志栈 | 调用 logging::init_logging，支持 --verbose/--quiet | ✓ |
| 简单日志（和 init/validate 一样） | 较轻量，但 stats 处理大量日志文件时用户无法看到进度 | |

**User's choice:** 完整日志栈

---

## 慢 SQL TOP-N 聚合策略（Phase 52）

| Option | Description | Selected |
|--------|-------------|----------|
| BinaryHeap 最小堆，大小 = N | O(M log N) 时间，O(N) 内存，对超大日志友好 | ✓ |
| Vec 全收完排序取前 N | 实现简单，但内存 O(M)，对 1.1GB 真实日志可能耗尽内存 | |

**User's choice:** BinaryHeap 最小堆

---

## 高频 SQL 分组聚合数据结构（Phase 52）

| Option | Description | Selected |
|--------|-------------|----------|
| HashMap<normalized_sql, AggState> 一次扫描 | 内存里只保留每种模板的聚合状态，模板数远小于记录数 | ✓ |
| HashMap + BinaryHeap 双练 | 边展边维护堆，但模板数多时堆操作多，过度工程化 | |

**User's choice:** HashMap<normalized_sql, AggState>

---

## 两种聚合是否合并为一次扫描（Phase 52）

| Option | Description | Selected |
|--------|-------------|----------|
| 合并为一次扫描 | 每条记录同时更新慢 SQL 堆和高频 HashMap，只读一遍文件 | ✓ |
| 两次扫描 | 第一次扫慢 SQL，第二次扫高频，逻辑分明但头文件读两遍 | |

**User's choice:** 合并为一次扫描

---

## stats 输出集成方式（Phase 52）

| Option | Description | Selected |
|--------|-------------|----------|
| 独立输出函数，不复用 Exporter trait | write_csv_stats / write_sqlite_stats，逻辑清晰 | ✓ |
| 新建 StatsWriter trait 谐写现有结构 | 抽象层清晰但增加复杂度，stats 不需要这个抽象 | |
| 直接在 ExporterManager 上添加 stats 方法 | ExporterManager 变臃肿、职责不清 | |

**User's choice:** 独立输出函数，不复用 Exporter trait

---

## CSV 输出文件命名（Phase 52）

| Option | Description | Selected |
|--------|-------------|----------|
| 硬编码文件名，和配置 CSV 同目录 | slow_sql.csv / frequent_sql.csv，简单可预期 | ✓ |
| 从配置路径衍生：data_slow.csv / data_frequent.csv | 着搜驱动，但文件名不典型时可读性差 | |
| 当前目录硬编码名称 | 最简单但不尊重用户的 CSV 路径配置 | |

**User's choice:** 硬编码文件名（`slow_sql.csv`/`frequent_sql.csv`），和配置 CSV 同目录

---

## SQLite 输出表名（Phase 52）

| Option | Description | Selected |
|--------|-------------|----------|
| 硬编码 slow_sql 和 frequent_sql | 和 ROADMAP success criteria 示例一致，简单可预期 | ✓ |
| 可配置表名 | 引入新配置字段，过度工程化 | |

**User's choice:** 硬编码表名

---

## Claude's Discretion

- Phase 52 D-10：SQLite 写入是 DROP + CREATE（确保每次 stats 的输出是最新结果）还是 `CREATE TABLE IF NOT EXISTS`（允许追加）——用户未明确，Claude 推荐 DROP + CREATE，确保结果幂等。

## Deferred Ideas

None — 讨论始终在里程碑 v1.13 范围内。
