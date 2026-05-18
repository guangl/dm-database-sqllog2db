# Phase 19: 代码结构重构 - Research

**Researched:** 2026-05-18
**Domain:** Rust 模块拆分 / 可见性收紧 / 代码重组（零行为变更）
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** 5 个超千行文件同等优先，全部拆分：
  - `src/pipeline/filters.rs`（1481 行）
  - `src/config/mod.rs`（1418 行）
  - `src/exporter/sqlite.rs`（1302 行）
  - `src/cli/run.rs`（1281 行）
  - `src/exporter/csv.rs`（1260 行）
- **D-02:** 拆分后每个子文件不超过 300 行
- **D-03:** 就地转子模块——原文件改为目录下的 `mod.rs`，子职责抽为独立 `.rs`，re-export 保持外部路径不变
- **D-04:** 拆分与可见性收紧同步进行，不做两轮扫描
- **D-05:** 新建 `src/exporter/projection.rs`，提取 `projected_field_names(ordered_indices: &[usize]) -> Vec<&'static str>`
- **D-06:** CSV/SQLite 各自保留序列化格式实现，共用层只负责 ordered_indices → 字段名列表映射
- **D-07:** 清理 `export_one_normalized` / `export_one_preparsed` / `ExporterKind` 默认实现中冗余的 match 分支
- **D-08:** 将 `DryRunExporter` 整合进 `ExporterKind`（不再单独 `impl Exporter for DryRunExporter`）
- **D-09:** 按需将内部方法提升进 trait，最小改动原则
- **D-10:** 全面收紧——对整个 codebase 所有 `pub` 项逐一评估；binary crate 无跨 crate 暴露需求，不保留任何无意义的 `pub`

### Claude's Discretion

- 子模块的具体命名（如 `filters/record.rs` vs `filters/meta.rs`）
- `run.rs` 的拆分边界（按预扫描/主循环/并行路径拆）
- 是否顺手修复 CONCERNS.md 中的低风险 tech debt（`conn.as_ref().unwrap()` → `conn_ref()` 辅助函数）——可以，但不引入行为变化

### Deferred Ideas (OUT OF SCOPE)

- `ResumeState.processed` 线性搜索改为 HashMap（Phase 20 或独立 Phase）
- SQLite PRAGMA 崩溃安全性改进（行为变化）
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REFACTOR-01 | 超过 300 行的源文件按职责拆分为独立子模块 | 见"拆分方案"章节，每个文件均有完整职责边界分析 |
| REFACTOR-02 | CsvExporter 与 SqliteExporter 中重复的字段投影逻辑抽取到共用辅助函数 | 见"共用投影层"章节，确认两处重复 + projection.rs 方案 |
| REFACTOR-03 | Exporter trait 接口统一，消除不必要的特化分支 | 见"Exporter trait 清理"章节，DryRunExporter 整合方案 |
| REFACTOR-04 | 内部类型可见性收紧（pub → pub(crate) / pub(super)） | 见"可见性收紧"章节，逐文件分析 |
</phase_requirements>

---

## Summary

本 Phase 是纯结构性重构，**不改变任何对外行为**。目标是将 5 个超千行文件拆分为职责清晰的子模块，提取 CSV/SQLite 共用的字段投影逻辑，整合 `DryRunExporter`，并全面收紧 `pub` 可见性。

当前代码库基线良好：55 个测试全部通过，clippy 零警告。重构期间必须保持这一基线。核心风险点是 `run.rs` 的热循环逻辑——它包含 parallel/sequential 两路径以及复杂的参数替换状态机，拆分时需特别小心借用关系。

`config/mod.rs` 特殊：其 1418 行中有约 1000 行是测试代码（89 个测试函数），真正的业务逻辑仅 400 行（`Config` struct + validate/apply_one 方法）。拆分时把测试随对应业务逻辑迁移即可，不需要像其他文件那样做太多子模块划分。

**Primary recommendation:** 按文件依赖复杂度排序逐个拆分——先 `filters.rs`（独立性强），再 `config/mod.rs`（测试占主体），再 `exporter/csv.rs` 和 `sqlite.rs`（共建 projection.rs），最后 `run.rs`（依赖前四个）。

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| 字段投影映射（indices→名称） | `exporter/projection.rs`（新建） | — | 纯数据映射，与序列化格式无关 |
| CSV 行序列化 | `exporter/csv/writer.rs` | projection | 格式特定，保留在 csv 模块 |
| SQLite INSERT/CREATE 构造 | `exporter/sqlite/sql_builder.rs` | projection | 格式特定，保留在 sqlite 模块 |
| 过滤器数据结构 | `pipeline/filters/types.rs` | — | Serde 序列化结构，独立于过滤逻辑 |
| 预编译过滤器 | `pipeline/filters/compiled.rs` | — | 运行时热路径，与配置结构分离 |
| 主编排逻辑 | `cli/run/orchestrate.rs` | — | 顶层入口 handle_run |
| 预扫描逻辑 | `cli/run/prescan.rs` | — | scan_log_file_for_matches 等 |
| 并行 CSV 处理 | `cli/run/parallel.rs` | — | process_csv_parallel + concat_csv_parts |
| 热循环（单文件处理） | `cli/run/processor.rs` | — | process_log_file |
| config validate/apply | `config/validate.rs` | — | 验证与覆盖逻辑，从 mod.rs 分出 |

---

## Standard Stack

### Core（无新依赖，所有工具均已在 Cargo.toml 中）

| 工具 | 版本 | 用途 | 说明 |
|------|------|------|------|
| Rust 模块系统 | stable | 就地子模块转换 | `mod.rs` + 子文件 + `pub use` re-export |
| `cargo clippy` | —（CI 已有） | 可见性检查 | `--all-targets -D warnings` |
| `cargo test` | —（CI 已有） | 回归验证 | 55 个现有测试是基准 |

本 Phase 不安装任何新包。

---

## Package Legitimacy Audit

本 Phase 不引入任何新外部依赖，跳过此节。

---

## Architecture Patterns

### System Architecture Diagram（重构后目标结构）

```
src/
├── pipeline/
│   └── filters/
│       ├── mod.rs          ← re-export + FiltersFeature 主逻辑
│       ├── types.rs        ← RecordMeta / IncludeFilters / ExcludeFilters / SqlFilters / IndicatorFilters / RawFiltersFeature（~300行）
│       ├── compiled.rs     ← CompiledMetaFilters / CompiledSqlFilters（~250行）
│       └── serde_helpers.rs← vec_to_hashset / vec_to_i64_hashset / compile_patterns / match_any_regex（~80行）
├── config/
│   ├── mod.rs              ← Config struct + from_file（精简到~80行）
│   ├── validate.rs         ← validate / validate_and_compile / validate_* 私有方法（~200行）
│   ├── apply_one.rs        ← apply_overrides / apply_one（~200行）
│   ├── exporter.rs         ← 已有，保留
│   ├── logging.rs          ← 已有，保留
│   ├── resume.rs           ← 已有，保留
│   └── sqllog.rs           ← 已有，保留
├── exporter/
│   ├── mod.rs              ← Exporter trait / ExporterKind（DryRun 整合） / ExporterManager / 工具函数（精简~350行）
│   ├── projection.rs       ← projected_field_names()（新建，~30行）
│   ├── csv/
│   │   ├── mod.rs          ← CsvExporter struct + from_config + new + re-export（~150行）
│   │   ├── writer.rs       ← write_record_preparsed + write_csv_escaped（~300行）
│   │   └── companion.rs    ← write_companion_rows + format_companion_row（~100行）
│   └── sqlite/
│       ├── mod.rs          ← SqliteExporter struct + from_config + new + Exporter impl（~200行）
│       ├── sql_builder.rs  ← build_insert_sql / build_create_sql（~100行）
│       └── write.rs        ← do_insert_preparsed / write_template_stats（~300行）
└── cli/
    ├── run/
    │   ├── mod.rs          ← handle_run + 公开接口（~150行）
    │   ├── processor.rs    ← process_log_file（单文件热循环）（~250行）
    │   ├── parallel.rs     ← process_csv_parallel / concat_csv_parts（~280行）
    │   └── prescan.rs      ← scan_log_file / scan_for_trxids / build_pipeline / recompile_meta（~180行）
    └── stats.rs            ← 保留现状（1039行，不在拆分范围内）
```

### Recommended Project Structure

```
src/
├── pipeline/
│   └── filters/            # 原 filters.rs → 子模块目录
├── config/
│   ├── validate.rs         # 新：验证逻辑
│   └── apply_one.rs        # 新：覆盖逻辑
├── exporter/
│   ├── projection.rs       # 新：字段映射辅助函数
│   ├── csv/                # 原 csv.rs → 子模块目录
│   └── sqlite/             # 原 sqlite.rs → 子模块目录
└── cli/
    └── run/                # 原 run.rs → 子模块目录
```

### Pattern 1: 就地转子模块（Rust 惯用法）

**What:** 将单文件 `foo.rs` 改为目录 `foo/mod.rs`，子职责抽到 `foo/child.rs`，通过 `mod.rs` re-export 保持外部路径不变。

**When to use:** 文件超过 300 行且包含多个独立职责时。

**Example:**

```rust
// 原：src/pipeline/filters.rs（1481行）
// 新：src/pipeline/filters/mod.rs

pub mod types;      // RecordMeta, IncludeFilters, ExcludeFilters, ...
pub mod compiled;   // CompiledMetaFilters, CompiledSqlFilters
mod serde_helpers;  // 私有辅助函数

// re-export：保持 crate::pipeline::filters::* 路径稳定
pub use types::{
    FiltersFeature, IncludeFilters, ExcludeFilters,
    RecordMeta, SqlFilters, IndicatorFilters,
};
pub use compiled::{CompiledMetaFilters, CompiledSqlFilters};
```

```rust
// 原：src/exporter/csv.rs（1260行）
// 新：src/exporter/csv/mod.rs

mod writer;         // write_record_preparsed（热路径）
mod companion;      // write_companion_rows

pub use self::companion::write_companion_rows;  // 保持 crate::exporter::csv::write_companion_rows 可用

pub struct CsvExporter { ... }
// ...
```

**Source:** [ASSUMED] — Rust 标准模块转换惯用法，rustbook.com 有记录

### Pattern 2: DryRunExporter 整合进 ExporterKind（D-08）

**What:** 删除独立的 `DryRunExporter` struct 及其 `impl Exporter`，改为在 `ExporterKind::DryRun` variant 的 match arm 中直接内联实现。

**Before:**
```rust
pub enum ExporterKind {
    Csv(CsvExporter),
    Sqlite(SqliteExporter),
    DryRun(DryRunExporter),   // 包裹独立 struct
}

pub struct DryRunExporter { stats: ExportStats }
impl Exporter for DryRunExporter { ... }   // 独立 impl
```

**After:**
```rust
pub enum ExporterKind {
    Csv(CsvExporter),
    Sqlite(SqliteExporter),
    DryRun { stats: ExportStats },  // struct variant，无需独立类型
}

// ExporterKind 自身处理 DryRun variant 的各 match arm
impl ExporterKind {
    fn export_one_preparsed(...) -> Result<()> {
        match self {
            Self::Csv(e) => e.export_one_preparsed(...),
            Self::Sqlite(e) => e.export_one_preparsed(...),
            Self::DryRun { stats } => { stats.exported += 1; Ok(()) },
        }
    }
}
```

**注意：** `ExporterManager::dry_run()` 和 `ExporterManager::from_csv()` 的公开签名不变。

**Source:** [ASSUMED] — 根据 CONTEXT.md D-08 决策和现有代码结构推导

### Pattern 3: projection.rs 共用辅助函数（D-05/D-06）

**What:** 提取两处相同的 `ordered_indices → Vec<&'static str>` 映射到独立文件。

**代码示例（来自 CONTEXT.md）：**
```rust
// src/exporter/projection.rs
pub(crate) fn projected_field_names(ordered_indices: &[usize]) -> Vec<&'static str> {
    use crate::pipeline::FIELD_NAMES;
    ordered_indices.iter().map(|&i| FIELD_NAMES[i]).collect()
}
```

**Where to use:**
- `sqlite/sql_builder.rs::build_create_sql` 的 `cols` 构造
- `sqlite/sql_builder.rs::build_insert_sql` 的 `cols` 构造
- （CSV 的 `write_record_preparsed` 不使用此函数，它直接按 idx 分 match arm 写入，保持不变）

**Source:** [ASSUMED] — 根据 CONTEXT.md D-05/D-06 和代码分析

### Pattern 4: 可见性层级决策树（D-10）

```
pub 项评估流程：
  被跨 crate 访问？→ 不可能（binary crate）→ 不需要 pub
  被 integration test 跨模块访问？→ pub(crate)
  被同层兄弟模块访问？→ pub(crate)
  仅被父模块访问？→ pub(super)
  仅在本模块内？→ 去掉 pub（或 private）
```

**现状中需要收紧的典型情况：**

| 位置 | 当前 | 应改为 | 理由 |
|------|------|--------|------|
| `filters.rs` 所有结构体字段（`pub users: ...`） | `pub` | `pub(crate)` | 仅 crate 内使用 |
| `CompiledMetaFilters` 字段（`pub usernames: Option<Vec<Regex>>`） | `pub` | `pub(crate)` | 仅 crate 内使用 |
| `ExportStats` 字段（`pub exported: usize` 等） | `pub` | `pub(crate)` | 仅 crate 内使用 |
| `ExportStats::new()` / `record_success()` / `total()` | `pub` | `pub(crate)` | 仅 crate 内使用 |
| `ExporterManager` 大多数方法 | `pub` | `pub(crate)` | cli 模块调用，无跨 crate |
| `CsvExporter::new()` / `from_config()` | `pub` | `pub(crate)` | 仅 cli/run.rs 调用 |
| `SqliteExporter::new()` / `from_config()` | `pub` | `pub(crate)` | 仅 cli/run.rs 调用 |
| `Exporter` trait | `pub` | `pub(crate)` | binary crate，无外部使用 |
| `lib.rs: pub use exporter::{CsvExporter, Exporter, SqliteExporter}` | `pub use` | 评估是否保留 | integration test 若需要则 pub(crate) |

**例外：** `lib.rs` 中的 `pub mod` 和 `pub use` 若有 integration test 依赖，可以保留或改为 `pub(crate)`。需逐一核查测试引用路径。

### Anti-Patterns to Avoid

- **拆分后忘记 re-export：** 子模块改变路径后，调用方 `use` 路径会断裂。每次拆分后必须在 `mod.rs` 中添加 `pub use` / `pub(crate) use`，并且 `cargo build` 验证无编译错误。
- **拆分顺序颠倒（run.rs 先动）：** `run.rs` 依赖 filters、exporter，应最后处理。
- **可见性过度收紧破坏测试：** integration test 在 `tests/` 目录下访问的类型需要保留至少 `pub(crate)` 或 `pub`。
- **同时在热路径中引入间接层：** `projection.rs` 的 `projected_field_names` 返回 `Vec`，在 `write_record_preparsed`（热路径）中每条记录都调用会引入分配。只在初始化路径（`build_insert_sql`、`build_create_sql`）中使用，不在逐记录路径中调用。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 模块间 re-export | 手写重复的类型转换层 | Rust `pub use` | 零成本，保持路径稳定 |
| pub 可见性审计 | 手动逐行检查 | `cargo clippy --all-targets -D warnings` + `dead_code` lint | clippy 能捕获未使用的 pub 项 |
| 测试回归 | 手动运行 | `cargo test` | 55 个现有测试是自动化安全网 |
| 编译验证 | 目测检查路径 | `cargo build` 在每次拆分后立即运行 | 编译器是最可靠的路径验证工具 |

**Key insight:** 重构的核心安全保障是"小步前进 + 每步编译"。每拆完一个文件立即 `cargo build && cargo test`，而不是所有文件拆完再测试。

---

## Runtime State Inventory

本 Phase 是纯代码结构重构，不涉及任何数据库、外部服务、OS 注册状态或已安装二进制。所有变更均在源代码文件中。

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — 不改变数据格式 | 无 |
| Live service config | None | 无 |
| OS-registered state | None | 无 |
| Secrets/env vars | None | 无 |
| Build artifacts | `target/` 目录中的编译缓存 | 重构后 `cargo build` 自动重建 |

---

## Common Pitfalls

### Pitfall 1: 拆分 csv.rs 后 write_record_preparsed 的借用分裂

**What goes wrong:** `CsvExporter::write_record_preparsed` 是静态方法，接受分散的 `&mut` 参数（`itoa_buf`、`line_buf`、`writer`、`path`），这是为了规避 Rust 的借用规则（不能同时借用 `self` 的多个字段）。将此方法移到子模块后，如果签名或调用方式改变，可能引发借用冲突。

**Why it happens:** Rust 借用检查器对 `self` 字段的细粒度借用分析不跨越函数调用边界。

**How to avoid:** 保持 `write_record_preparsed` 的静态方法签名不变，只移动文件位置，不改变参数结构。

**Warning signs:** 编译错误 `cannot borrow *self as mutable more than once at a time`。

### Pitfall 2: DryRunExporter 测试断裂（D-08）

**What goes wrong:** `exporter/mod.rs` 的测试中有多处直接构造 `DryRunExporter::default()`、`DryRunExporter { stats }`。整合后这些测试需要更新为使用 `ExporterKind::DryRun { stats: ExportStats::default() }` 或 `ExporterManager::dry_run()`。

**How to avoid:** 整合 DryRun 时同步更新所有测试，`cargo test` 验证。

**Warning signs:** 编译错误 `cannot find type DryRunExporter`。

### Pitfall 3: config/mod.rs 测试迁移时 use 路径断裂

**What goes wrong:** `config/mod.rs` 的测试（89 个）大量使用 `use super::*`，拆分后 validate 方法移到 `validate.rs`，如果测试不跟着迁移，`super::*` 不再覆盖相关方法。

**How to avoid:** 将各 validate 测试随对应私有函数一起迁移到 `validate.rs` 的 `#[cfg(test)]` 块，或在 `mod.rs` 中 `pub(super) use validate::*` 保持可见性。

**Warning signs:** 编译错误 `method not found` 或 `cannot find function test_validate_*`。

### Pitfall 4: pub 收紧导致 lib.rs 集成测试断裂

**What goes wrong:** `lib.rs` 当前 `pub use exporter::{CsvExporter, Exporter, SqliteExporter}`，若改为 `pub(crate) use`，tests/ 目录下（若有）的集成测试会断裂。

**How to avoid:** 检查 `tests/` 目录，确认哪些类型被外部测试引用后再收紧。

**Warning signs:** `cargo test` 时 `tests/` 中的编译错误。

**当前状态：** [VERIFIED: grep] 检查发现 `tests/` 目录不存在，当前所有测试都在各模块的 `#[cfg(test)]` 块中。因此 `lib.rs` 的 `pub use` 可以安全改为 `pub(crate) use`。

### Pitfall 5: run.rs 拆分时 prescan → pipeline 的双重编译

**What goes wrong:** `recompile_meta_if_needed` 在 prescan 后重新编译 `CompiledMetaFilters`，这段逻辑引用了多个 crate 内部类型。若拆分时这段代码的位置与 `filters` 模块路径发生变化，可能需要调整 `use` 语句。

**How to avoid:** 拆分 `run.rs` 时保持 `use crate::pipeline::*` 路径引用，不改变 pipeline 模块的导出路径。

---

## Code Examples

### 典型 pub 收紧模式

```rust
// 收紧前（filters.rs 中的结构体字段）
pub struct CompiledMetaFilters {
    pub usernames: Option<Vec<Regex>>,
    pub client_ips: Option<Vec<Regex>>,
    // ...
}

// 收紧后
pub(crate) struct CompiledMetaFilters {
    pub(crate) usernames: Option<Vec<Regex>>,
    pub(crate) client_ips: Option<Vec<Regex>>,
    // ...
}
```

### mod.rs re-export 保持路径稳定

```rust
// src/pipeline/filters/mod.rs
mod types;
mod compiled;
mod serde_helpers;

pub(crate) use types::{
    FiltersFeature, IncludeFilters, ExcludeFilters,
    RecordMeta, SqlFilters, IndicatorFilters,
};
pub(crate) use compiled::{CompiledMetaFilters, CompiledSqlFilters};

// 保持原路径：crate::pipeline::filters::FiltersFeature 等不变
```

### conn_ref() 辅助函数（CONCERNS.md tech debt，可顺带修复）

```rust
// sqlite/mod.rs — 取代 self.conn.as_ref().unwrap()
fn conn_ref(&self) -> crate::error::Result<&rusqlite::Connection> {
    self.conn
        .as_ref()
        .ok_or_else(|| Self::db_err("not initialized"))
}
```

---

## State of the Art

| Old Approach | Current Approach | Impact for This Phase |
|--------------|------------------|----------------------|
| 单个大文件 | 子模块目录 + mod.rs re-export | 是本 Phase 的核心操作 |
| `pub` 默认暴露 | `pub(crate)` / `pub(super)` 精确控制 | binary crate 标准实践 |
| 独立 struct per exporter variant | enum variant 直接内联状态 | D-08 目标 |

---

## Open Questions

1. **`cli/stats.rs`（1039 行）是否需要拆分**
   - What we know: 不在 D-01 的 5 个目标文件中；CONTEXT.md 未提及
   - What's unclear: stats.rs 超过了 300 行上限，但决策文档只列了 5 个文件
   - Recommendation: 本 Phase 不拆分，遵循 D-01 的明确范围。若有余力可在 Claude's Discretion 范围内评估，但不作为成功标准

2. **`exporter/mod.rs`（756 行）是否需要拆分**
   - What we know: CONTEXT.md 明确提到"本 Phase 也需评估是否拆分"（Canonical Refs 节）
   - What's unclear: 756 行超过 300 行但不在 D-01 明确列表中
   - Recommendation: 在 DryRunExporter 整合（D-08）后评估行数，若还超 300 行则拆分；`ExporterManager` + `ExporterKind` + `ExportStats` + 工具函数可作为拆分边界

---

## Environment Availability

本 Phase 仅涉及源代码结构重组，无外部工具依赖。

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo test` | 回归验证 | ✓ | Rust stable | — |
| `cargo clippy` | 可见性检查 | ✓ | stable | — |
| `cargo build` | 每步编译验证 | ✓ | stable | — |

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust 内置测试 + criterion（bench） |
| Config file | `Cargo.toml`（无独立配置文件） |
| Quick run command | `cargo test` |
| Full suite command | `cargo test && cargo clippy --all-targets -- -D warnings` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REFACTOR-01 | 拆分后各子文件 ≤300 行 | structural check | `wc -l src/**/*.rs` | ✅（验证步骤） |
| REFACTOR-01 | 拆分后现有测试全部通过 | regression | `cargo test` | ✅（55 个现有测试） |
| REFACTOR-02 | projection.rs 存在且被 csv/sqlite 调用 | structural | `grep -r "projection"` | ❌ Wave 0：新建文件 |
| REFACTOR-03 | DryRunExporter struct 已删除 | structural | `grep -r "DryRunExporter"` | ✅（验证步骤） |
| REFACTOR-04 | 无多余 pub（clippy 零警告） | lint | `cargo clippy --all-targets -- -D warnings` | ✅ |

### Sampling Rate

- **每个文件拆分后：** `cargo build`（快速编译验证）
- **每个文件拆分+测试后：** `cargo test`
- **Phase 完成前：** `cargo test && cargo clippy --all-targets -- -D warnings`

### Wave 0 Gaps

- [ ] `src/exporter/projection.rs` — 新文件，需在 Wave 1 创建（REFACTOR-02）
- [ ] `src/pipeline/filters/` 目录结构 — 需在 Wave 1 创建（REFACTOR-01）
- [ ] `src/cli/run/` 目录结构 — 需在最后一个 Wave 创建（REFACTOR-01）

---

## Security Domain

本 Phase 不引入新的数据流或输入处理，安全面不变。CONCERNS.md 中已记录的 `table_name` SQL 拼接和 `unsafe_code = "warn"` 问题不属于本 Phase 范围。

---

## Sources

### Primary (HIGH confidence)

- [VERIFIED: grep] `src/` 目录下所有目标文件的直接代码分析
- [VERIFIED: cargo test] 基线测试状态：55 tests passed
- [CITED: CONTEXT.md] 所有 D-01 ~ D-11 决策

### Secondary (MEDIUM confidence)

- [CITED: .planning/codebase/CONCERNS.md] tech debt 参考（conn_ref、apply_one 白名单）
- [CITED: .planning/REQUIREMENTS.md] REFACTOR-01 ~ REFACTOR-04 需求定义

### Tertiary (LOW confidence)

无。所有核心结论来自直接代码分析，无需依赖 WebSearch。

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `write_record_preparsed` 静态方法签名不变可规避借用问题 | Architecture Patterns | 可能需要重新设计参数传递方式，影响 csv 拆分方案 |
| A2 | `ExporterKind::DryRun` 改为 struct variant 后测试可直接构造 | Architecture Patterns | 需要额外的构造辅助函数，影响 D-08 实现 |
| A3 | `tests/` 目录不存在（已通过 grep 验证） | Pitfall 4 | 若存在 integration test，需保留部分 pub |
| A4 | `cli/stats.rs` 不在本 Phase 拆分范围内 | Open Questions | 若 D-01 意图覆盖 stats.rs，则需增加工作量 |

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — 无新依赖，基于已有 Rust 工具链
- Architecture: HIGH — 基于直接代码阅读，拆分方案有明确行数依据
- Pitfalls: HIGH — 基于实际代码中的借用模式和测试结构分析

**Research date:** 2026-05-18
**Valid until:** 本 Phase 完成为止（重构完成后结构即已固化）
