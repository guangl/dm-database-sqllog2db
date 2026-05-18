# Phase 19: 代码结构重构 - Context

**Gathered:** 2026-05-17
**Status:** Ready for planning

<domain>
## Phase Boundary

系统性重构源码结构：将所有超千行文件（filters.rs / config/mod.rs / sqlite.rs / run.rs / csv.rs）按职责拆分为子模块；提取 CSV 与 SQLite 共用的字段投影辅助函数至 `exporter/projection.rs`；清理 Exporter trait 中冗余的特化分支并整合 DryRunExporter；全面收紧 `pub` 可见性为 `pub(crate)` / `pub(super)`。

**本 Phase 不改变任何对外行为或公开 API 语义**——只重组代码结构、消除重复、降低跨层暴露。

</domain>

<decisions>
## Implementation Decisions

### 文件拆分粒度与优先级

- **D-01:** 所有 5 个超千行文件**同等优先**，全部需要拆分：
  - `src/pipeline/filters.rs`（1481 行）
  - `src/config/mod.rs`（1418 行）
  - `src/exporter/sqlite.rs`（1302 行）
  - `src/cli/run.rs`（1281 行）
  - `src/exporter/csv.rs`（1260 行）

- **D-02:** 拆分后每个子文件行数**不超过 300 行**（对应 ROADMAP 原文"原超过 300 行的源文件"的意图）。

- **D-03:** 拆分方式为**就地转子模块**：原文件改为目录下的 `mod.rs`（或保留同名），子职责抽为独立 `.rs` 文件，通过 `mod.rs` re-export 使外部调用路径不变。

- **D-04:** 拆分与可见性收紧**同步进行**——在每个文件拆分时一并评估 `pub` 是否必要，不做两轮扫描。

### 字段投影共用逻辑

- **D-05:** 新建 `src/exporter/projection.rs`，提取计算选定列名的共用辅助函数（如 `projected_field_names(ordered_indices: &[usize]) -> Vec<&'static str>`）。

- **D-06:** CSV 和 SQLite 各自保留自己的序列化格式实现（`build_header`/`build_insert_sql` 等）；共用层只负责从 `ordered_indices` 到字段名列表的映射，不强行统一序列化逻辑。

### Exporter trait 统一

- **D-07:** 清理 `export_one_normalized` / `export_one_preparsed` / `ExporterKind` 默认实现中的冗余 `match` 分支——凡是可以通过更简单的转发或消除 match arm 实现的，均应简化。

- **D-08:** 将 `DryRunExporter` 整合进 `ExporterKind`（不再单独 `impl Exporter for DryRunExporter`），消除这一特化分支。

- **D-09:** 如果字段投影共用逻辑需要，可**按需**将内部方法（如 `ordered_indices` 访问器）提升进 trait；不预先扩展 trait 接口，以最小改动为原则。

### 可见性收紧

- **D-10:** **全面收紧**：对整个 codebase 所有 `pub` 项逐一评估。判断标准：
  - 同一模块内访问 → `pub(super)` 或去掉 `pub`
  - 跨模块（crate 内）访问 → `pub(crate)`
  - 测试代码跨模块访问 → `pub(crate)` 即可（binary crate 无对外 API）
  - 无访问者 → 去掉 `pub`

- **D-11:** 本 Phase **不保留任何无意义的 `pub`**（因为这是 binary crate，没有跨 crate 暴露需求）。

### Claude's Discretion

- 子模块的具体命名（如 `filters/record.rs` vs `filters/meta.rs`）——按语义清晰度自行决定。
- `run.rs` 的拆分边界（如是否按"预扫描/主循环/并行路径"拆）——由实现阶段根据代码依赖关系决定。
- 是否在拆分过程中顺手修复 CONCERNS.md 中的低风险 tech debt（如 `conn.as_ref().unwrap()` → `conn_ref()` 辅助函数）——可以，但不要引入行为变化。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 重构目标文件（全部需要拆分）

- `src/pipeline/filters.rs` — 过滤器核心逻辑，含 `RecordMeta`、`IncludeFilters`、`ExcludeFilters`、`SqlFilters`、两阶段过滤流程（1481 行）
- `src/config/mod.rs` — `Config` struct、`validate()`、`apply_one()`、所有子配置 struct（1418 行）
- `src/exporter/sqlite.rs` — `SqliteExporter` 含 `build_insert_sql`、`build_create_sql`、`write_batch`（1302 行）
- `src/cli/run.rs` — 主编排逻辑，含预扫描、热循环、并行路径、模板聚合（1281 行）
- `src/exporter/csv.rs` — `CsvExporter` 含 `build_header`、`write_record_preparsed`、并行 CSV 拼接（1260 行）

### 共用字段映射（投影辅助函数的依据）

- `src/pipeline/mod.rs` — `FIELD_NAMES` 常量（字段名数组，CSV 和 SQLite 共同引用）；新建 `exporter/projection.rs` 应从此处引用

### Exporter trait 与 enum dispatch

- `src/exporter/mod.rs` — `Exporter` trait（6 个方法）、`ExporterKind` enum（Csv/Sqlite/DryRun）、`ExporterManager`（756 行，本 Phase 也需评估是否拆分）
- `src/exporter/mod.rs:12-65` — trait 方法定义及默认实现（D-07 清理对象）
- `src/exporter/mod.rs:174-228` — `DryRunExporter` 独立 struct impl（D-08 整合对象）

### 需求规范

- `.planning/REQUIREMENTS.md` — REFACTOR-01 / REFACTOR-02 / REFACTOR-03 / REFACTOR-04（Phase 19 的四条需求）
- `.planning/ROADMAP.md` §Phase 19 — Success Criteria（5 条验收标准）

### 技术债参考（可顺带修复）

- `.planning/codebase/CONCERNS.md` — 已记录的 tech debt（`conn.as_ref().unwrap()`、`apply_one` 硬编码白名单等）；修复须不引入行为变化

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- `crate::pipeline::FIELD_NAMES` (`src/pipeline/mod.rs`) — 字段名常量数组，CSV/SQLite 都已引用；`projection.rs` 直接复用
- `ExporterKind` enum (`src/exporter/mod.rs:67`) — 已实现无虚表 enum dispatch，DryRunExporter 整合后继续用此模式
- `ordered_indices: Vec<usize>` — CSV 中 `pub(crate)`，SQLite 中 `pub(super)`；拆分时各自保留，通过 `projection.rs` 的函数统一映射逻辑

### Established Patterns

- **就地子模块**：`src/config/` 已经是目录（`mod.rs` + `exporter.rs` / `logging.rs` 等子文件），`filters.rs` 拆分时复用同一模式
- **`#[serde(default)]`**：config 子模块所有字段均有默认值，拆分后保持不变
- **`pub(crate)` 优先**：`src/exporter/csv.rs:107` 中 `ordered_indices` 已是 `pub(crate)`，新代码遵循同一策略

### Integration Points

- `src/cli/run.rs` 是最大的调用方，同时也是拆分对象之一——拆分 `run.rs` 时需保持对 `ExporterManager`、`Pipeline`、`SqllogParser` 的引用路径不变
- 拆分 `filters.rs` 后，`RecordMeta` / `IncludeFilters` / `ExcludeFilters` 的路径可能变化，需通过 `pipeline/filters/mod.rs` re-export 保持 `crate::pipeline::filters::*` 路径稳定
- `exporter/projection.rs` 新增后，`csv.rs` 和 `sqlite.rs` 的 `build_header` / `build_insert_sql` 应调用 `projection::projected_field_names()`

</code_context>

<specifics>
## Specific Ideas

- 拆分目录结构示例（`filters.rs` → 子模块）：
  ```
  src/pipeline/filters/
    mod.rs          — re-export + 两阶段过滤入口
    record.rs       — RecordMeta struct
    include.rs      — IncludeFilters + has_filters
    exclude.rs      — ExcludeFilters + has_filters
    sql.rs          — SqlFilters（SQL 内容过滤）
    compiled.rs     — CompiledMetaFilters + TrxidSet
  ```

- 投影辅助函数示意：
  ```rust
  // src/exporter/projection.rs
  pub(crate) fn projected_field_names(ordered_indices: &[usize]) -> Vec<&'static str> {
      use crate::pipeline::FIELD_NAMES;
      ordered_indices.iter().map(|&i| FIELD_NAMES[i]).collect()
  }
  ```

- DryRunExporter 整合后，`ExporterKind` 可能变为：
  ```rust
  pub enum ExporterKind {
      Csv(CsvExporter),
      Sqlite(SqliteExporter),
      DryRun,  // 无需独立 struct
  }
  ```

</specifics>

<deferred>
## Deferred Ideas

- `ResumeState.processed` 线性搜索改为 `HashMap`（CONCERNS.md 记录的 tech debt）— 性能影响仅在数千文件场景下显现，不属于结构重构范畴，放入 Phase 20 或独立 Phase
- SQLite PRAGMA 崩溃安全性改进（`WAL + synchronous=NORMAL`）— 行为变化，超出本 Phase 范围

### Reviewed Todos (not folded)

- "调研 dm-database-parser-sqllog 1.0.0 新特性" — Phase 18 CONTEXT.md 已标记为"已在 Phase 6 关闭（PERF-07），无关"，继续跳过

</deferred>

---

*Phase: 19-代码结构重构*
*Context gathered: 2026-05-17*
