# Phase 30: 移除模板分析 — Research

**研究日期:** 2026-05-20
**领域:** 功能移除 / 配置精简 / 依赖清理
**置信度:** HIGH (通过源代码遍历确认所有引用点)

## 摘要

Phase 30 移除 SQL 模板分析功能：聚合器 (`TemplateAggregator`)、模板报告器 (`TemplateReporter`) 及其配套配置段 `[template]` 和 `[template.report]`。同时移除 `hdrhistogram` 依赖和所有 `*_templates.csv` / SQLite 模板报告文件的生成逻辑。

这是 v1.7 系列中的第三个移除阶段，依赖 Phase 29 完成。主要操作分布在 **16 个文件**（删除 2 个，修改 14 个）。

**重要前置依赖:** Phase 28 已移除 `src/charts/` 目录（`charts/mod.rs` 引用了 `TemplateAggregator` 和 `ChartEntry`，在移除图表时已清理），Phase 29 已将 `normalize_template` 从 `fingerprint.rs` 迁移到 `normalizer.rs`。Phase 30 不需要再触及 `fingerprint.rs`。

**主要推荐:** 删除 `aggregator.rs` 和 `template_reporter.rs`；从 `Cargo.toml` 移除 `hdrhistogram`；移除 `Config` 中 `template` 字段及相关配置覆盖/校验/展示代码；从 `cli/run` 热循环中移除所有 template_agg 相关逻辑。

## `<user_constraints>` — Not applicable (no CONTEXT.md exists for this phase)

## 阶段需求

| ID | 描述 | 研究支持 |
|----|------|----------|
| RM-05 | 移除模板分析+报告（aggregator.rs、template_reporter.rs）、移除 hdrhistogram 依赖、移除 [template]/[template.report] 配置段 | 本文件覆盖所有 16 个需要修改的文件，逐个标注变更点 |

## 架构职责映射

| 能力 | 主层 | 辅助层 | 理由 |
|------|------|--------|------|
| 模板统计聚合 | 无（已移除） | — | `TemplateAggregator` 从热循环中完全移除 |
| 模板报告输出 | 无（已移除） | — | `TemplateReporter` + CSV/SQLite 写路径完全移除 |
| 模板配置 | 无（已移除） | — | `TemplateConfig` + `TemplateReportConfig` 从 Config 结构体移除 |

## 标准栈

N/A — 本阶段无新增依赖。仅移除现有代码和依赖。

## 依赖合法性审计

| 包 | 注册表 | 年龄 | 下载量 | 源仓库 | slopcheck | 处置 |
|-----|----------|-----|---------|-----------|------------|----------|
| `hdrhistogram` | crates.io | ~8年 | — | github.com/HdrHistogram/HdrHistogram_rust | N/A | **移除** — 仅被 `aggregator.rs` 使用，无其他消费者 |

## 需要修改的文件完整列表

### A. 删除文件（2 个）

| 文件 | 行数 | 原因 |
|------|-------|------|
| `src/pipeline/aggregator.rs` | 447 行 | `TemplateAggregator`, `TemplateStats`, `ChartEntry` 及其所有测试 |
| `src/pipeline/template_reporter.rs` | 278 行 | `TemplateReporter` (write_csv, write_sqlite) 及其所有测试 |

### B. 修改文件（14 个）

---

#### B1. `src/pipeline/mod.rs` — 移除模块声明、类型定义、辅助函数、测试

**删除的行:**

| 行号 | 内容 | 说明 |
|------|--------|------|
| 12 | `pub mod aggregator;` | 模块声明 |
| 13-15 | `pub(crate) use aggregator::ChartEntry;` 至 `TemplateStats;` | 三个 re-export |
| 17 | `pub(crate) mod template_reporter;` | 模块声明 |
| 129-131 | `fn default_top_n() -> usize { 10 }` | 仅 ChartsConfig 使用，Phase 28 已移除，确认可删除 |
| **133-166** | **`pub struct TemplateConfig` + `TemplateReportConfig` + `impl Default`** | 整个配置类型定义（34 行） |
| **168-198** | **`template_report_enabled()` + `derive_template_report_paths()`** | 两个辅助函数（31 行） |
| 411-433 | `test_template_config_default` 等 3 个测试 | 配置测试 |

**注意:** `ChartEntry` 同时被 `charts/` 使用，但 Phase 28 已移除 charts 目录，所以 `ChartEntry` 的 re-export 不再需要。

---

#### B2. `src/config/mod.rs` — 移除 template 导入和字段

**修改的行:**

| 行号 | 变更 |
|------|--------|
| 15 | 从 `use crate::pipeline::{...}` 中删除 `TemplateConfig` |
| 38 | 删除 `pub template: Option<TemplateConfig>,` |

---

#### B3. `src/config/apply_one.rs` — 移除 template 配置覆盖

**删除的行:**

| 行号 | 代码 | 说明 |
|------|------|------|
| 123-125 | `"template.enable" => { ... }` | template.enable 覆盖 |
| 126-132 | `"template.report.enabled" => { ... }` | template.report.enabled 覆盖 |
| 133-139 | `"template.report.csv_report_path" => { ... }` | template.report.csv_report_path 覆盖 |
| 140-146 | `"template.report.sqlite_report_path" => { ... }` | template.report.sqlite_report_path 覆盖 |

**删除的测试（同一文件 `#[cfg(test)] mod tests` 内）:**

| 行号 | 测试函数 |
|------|-----------|
| 293-298 | `test_apply_one_template_enable` |
| 301-310 | `test_apply_one_template_report_csv_path` |
| 312-321 | `test_apply_one_template_report_sqlite_path` |

---

#### B4. `src/config/validate.rs` — 移除 validate_charts 中的 template 引用

**修改的行:**

| 行号 | 变更 |
|------|--------|
| 135 | 从 `use crate::pipeline::{..., TemplateConfig}` 中删除 `TemplateConfig` |

`validate_charts()` 方法（第 102-128 行）在第 104 行引用了 `self.template.as_ref().is_some_and(|t| t.enable)`。但 Phase 28 应已将 charts 的 `validate_charts()` 移除。如果 Phase 28 未完全移除，Phase 30 需要删除第 104 行的 template 依赖检查。

**删除的测试:**

| 行号 | 测试函数 |
|------|-----------|
| 451-503 | 所有 `test_validate_charts_*` 测试（charts 在 Phase 28 已移除） |
| 507-583 | 所有 `test_validate_charts_*` 中引用 `TemplateConfig` 的测试 |

如果 Phase 28 已清理，这些测试已不存在。Phase 30 不需要额外操作。

---

#### B5. `src/cli/init.rs` — 从配置模板中移除 [template] 段

**删除的行:**

`CONFIG_TEMPLATE_ZH` (第 84-94 行)：
```
[template]
# SQL 模板归一化（v1.4 新增顶层配置）
# 启用后对 sql_text 执行注释去除、IN 列表折叠、关键字大写、空白折叠四项变换，生成稳定的模板 key
# 默认 false（不影响热循环性能）
enable = false

# 模板报告独立输出（可选配置，跟随 template.enable 自动启用）
# [template.report]
# enabled = true
# csv_report_path = ""                # 留空 = 自动派生 (out.csv → out_templates.csv)
# sqlite_report_path = ""             # 留空 = 自动派生 (out.db → out_templates.db)
```

`CONFIG_TEMPLATE_EN` (第 199-209 行) — 同上英文版本。

共删除约 22 行（中英文各 11 行，其中约 5 行是注释）。

---

#### B6. `src/cli/show_config.rs` — 移除 [template] 配置展示段

**删除的行:**

| 行号 | 代码 | 说明 |
|------|------|------|
| 194-217 | `if let Some(ta) = &cfg.template { ... }` | 整个 template 展示块 |

---

#### B7. `src/cli/run/mod.rs` — 核心：从热循环中移除模板聚合和报告

**修改的行:**

| 行号 | 变更 |
|------|--------|
| 6 | 删除 `use crate::pipeline::template_reporter::TemplateReporter;` |
| 7 | 从 `use crate::pipeline::{...}` 中删除 `TemplateAggregator` |
| 8 | 删除整行 `use crate::pipeline::{derive_template_report_paths, template_report_enabled};` |
| **27-58** | **删除整个 `write_template_reports()` 函数** |
| 130 | 删除 `let do_template = final_cfg.template.as_ref().is_some_and(|t| t.enable);` |
| 153-168 | 修改 `process_csv_parallel()` 调用：删除 `do_template` 参数传递 |
| 171-181 | 删除 `parallel_agg` 处理和图表生成 |
| 176 | 删除 `let template_stats = parallel_agg.map(TemplateAggregator::finalize);` |
| 177-181 | 删除 template_stats 相关代码块 |
| 207 | 删除 `let mut template_agg = do_template.then(TemplateAggregator::new);` |
| 239 | 删除 `template_agg.as_mut()` 参数传递给 `process_log_file` |
| 257-261 | 删除 `if let Some(ref agg) = template_agg { charts ... }` 块 |
| 266-271 | 删除 `let template_stats = ...` 和 `write_template_reports()` 调用 |

**最终路径（简化后）:**
- 顺序路径：删除 `do_template`、`template_agg`、`write_template_reports`、`template` 相关图表生成
- 并行路径：`process_csv_parallel` 不再返回 `Option<TemplateAggregator>`，删除合并逻辑
- 两个路径的 `TemplateAggregator::finalize()` 调用全部删除

---

#### B8. `src/cli/run/processor.rs` — 从热循环中移除模板聚合逻辑

**修改的行:**

| 行号 | 变更 |
|------|--------|
| 5 | 从 `use crate::pipeline::{...}` 中删除 `TemplateAggregator` |
| 27 | 将 `mut aggregator: Option<&mut TemplateAggregator>` 改为 `_aggregator: Option<&mut ()>` 或直接移除参数 |
| **91** | **改变 `include_pm || aggregator.is_some()` 条件** — 移除 `aggregator.is_some()`，保留 `include_pm` |
| **131-161** | **删除整个模板聚合代码块**（`if let Some(ref mut agg) = aggregator { ... }`），包含 `normalize_template` 调用和 `agg.observe()` |

**重要:** 第 91 行注释提到了 "若 aggregator 存在，无论 include_pm 如何都需要真实的 exectime"。移除 aggregator 后，该条件简化为仅 `include_pm`。

---

#### B9. `src/cli/run/parallel.rs` — 从并行路径移除模板聚合

**修改的行:**

| 行号 | 变更 |
|------|--------|
| 5 | 从 `use crate::pipeline:{...}` 中删除 `TemplateAggregator` |
| 82 | 删除 `do_template: bool` 参数 |
| 87 | 将 `Option<TemplateAggregator>` 从返回类型中移除，改为 `() ` |
| 130 | 将 `TaskResult` 从 `Option<TemplateAggregator>` 改为 `()` |
| 168 | 删除 `let mut task_agg = do_template.then(TemplateAggregator::new);` |
| 180 | 将 `task_agg.as_mut()` 改为 `&mut None` 或移除参数 |
| 186 | 将 `task_agg` 从四元组中移除 |
| 197 | 删除 `let mut merged_agg: Option<TemplateAggregator> = None;` |
| 205-210 | 删除整个 map-reduce 合并逻辑 |
| 返回三元组改为二元组 `(Vec<(PathBuf, usize)>, usize)` |

---

#### B10. `src/exporter/mod.rs` — 从 Exporter trait 和 ExporterKind 中移除 write_template_stats

**修改的行:**

| 行号 | 变更 |
|------|--------|
| 54-63 | 从 `Exporter` trait 中删除 `write_template_stats` 默认实现 |
| 129-148 | 从 `ExporterKind` 中删除 `write_template_stats` 方法 |
| 290-299 | 从 `ExporterManager` 中删除 `write_template_stats` 方法 |

---

#### B11. `src/exporter/csv/mod.rs` — 移除 CSV 写模板统计

**修改的行:**

| 行号 | 变更 |
|------|--------|
| 10 | 删除 `mod companion;` |
| 13 | 删除 `pub(crate) use self::companion::write_companion_rows;` |
| 243-251 | 从 `CsvExporter` 中删除 `write_template_stats` 方法覆盖 |

**注意:** 删除 `mod companion;` 后，`src/exporter/csv/companion.rs` 的整个文件也可以删除或移动到 cleanup 阶段处理。若 Phase 32 处理文件清理，可以在此阶段仅删除 `mod` 声明。

---

#### B12. `src/exporter/sqlite/mod.rs` — 移除 SQLite 写模板统计

**修改的行:**

| 行号 | 变更 |
|------|--------|
| 250+ | 删除 `fn write_template_stats` 方法（约 30 行） |

---

#### B13. `src/cli/run/tests.rs` — 移除模板相关测试

**修改的行:**

| 行号 | 变更 |
|------|--------|
| 57-95 | 保留 `test_aggregator_disabled_none_path`（验证无 template 时 handle_run 正常，仍然有效） |
| 97-161 | 修改 `test_parallel_merge_consistent` — 从 toml 配置中移除 `[template]\nenable = true`（测试本身验证并行 CSV 一致性，不依赖 template） |
| 163-205 | **删除** `test_no_template_stats_when_disabled` |
| 207-268 | **删除** `test_template_stats_enabled_end_to_end_sequential` |

---

#### B14. `tests/integration.rs` — 移除集成测试中的模板引用

**修改的行:**

| 行号 | 变更 |
|------|--------|
| 15 | 从 `use dm_database_sqllog2db::pipeline::{...}` 中删除 `TemplateConfig` |
| 1191-1193 | 将 `assert!(content.contains("[template]"), ...)` 从 `test_init_generated_content_zh` 中删除 |
| 1254-1281 | 修改 `test_validate_rejects_legacy_pipeline_template_analysis` — 删除 `[pipeline.template_analysis] → [template]` 断言行（第 1269-1272 行） |
| **1418-1468** | **删除整个 `test_e2e_template_normalization`** 测试函数 |

---

#### B15. 测试文件：`src/exporter/csv/tests.rs` — 移除 template_stats 测试

| 行号 | 函数 |
|------|--------|
| 468-534 | `test_csv_write_template_stats` — 删除 |
| 569-593 | 匿名 inline test — 删除 |
| 605-629 | `test_csv_write_template_stats_none_skips` — 删除 |
| 639-661 | `test_csv_write_template_stats_empty_path_skips` — 删除 |

#### B16. 测试文件：`src/exporter/sqlite/tests.rs` — 移除 template_stats 测试

| 行号 | 函数 |
|------|--------|
| 448-462 | `make_template_stats_sqlite` 辅助函数 — 删除 |
| 464-494 | `test_sqlite_write_template_stats` — 删除 |
| 所有后续 write_template_stats 测试（约 300 行持续到文件末尾） — 全部删除 |

#### B17. 测试文件：`src/exporter/tests.rs` — 移除 write_template_stats 测试

| 行号 | 内容 |
|------|--------|
| 238-351 | 整个 `write_template_stats` 测试块（4 个测试 + 辅助函数 `make_template_stats`）— 删除 |

---

### C. Cargo.toml 变更

| 行号 | 变更 |
|------|--------|
| 60 | 删除 `hdrhistogram = "7.5.4"` |

`hdrhistogram` 仅被 `aggregator.rs` 使用。根据源代码确认：
- `aggregator.rs` 第 3 行: `use hdrhistogram::Histogram;`
- `charts/latency_hist.rs` 第 10 行: `histogram: &hdrhistogram::Histogram<u64>` — 但 Phase 28 已移除整个 charts 目录
- 无其他文件引用 `hdrhistogram`

因此移除 `aggregator.rs` 后，`hdrhistogram` 没有其他消费者，可以安全删除。

---

## 架构模式

### 推荐操作策略（3 阶段）

#### Plan 30-01: 配置层清理
1. `src/pipeline/mod.rs` — 删除 `TemplateConfig`、`TemplateReportConfig`、`default_top_n()`、`template_report_enabled()`、`derive_template_report_paths()`、aggregator/template_reporter 模块声明和 re-export、相关测试
2. `src/config/mod.rs` — 删除 `template` 字段和 `TemplateConfig` 导入
3. `src/config/apply_one.rs` — 删除 4 个 template.* 覆盖处理 + 3 个测试
4. `src/config/validate.rs` — 删除 `TemplateConfig` 导入
5. `src/cli/init.rs` — 从配置模板中删除 `[template]` 段
6. `src/cli/show_config.rs` — 删除 `[template]` 展示块
7. `Cargo.toml` — 删除 `hdrhistogram` 依赖

#### Plan 30-02: 运行时代码清理
1. `src/cli/run/mod.rs` — 删除 `write_template_reports`、`do_template`、`template_agg`、`write_template_reports` 调用、并行路径聚合器处理
2. `src/cli/run/processor.rs` — 删除 aggregator 参数和模板聚合热循环代码块
3. `src/cli/run/parallel.rs` — 删除 `do_template` 参数、`TemplateAggregator` 类型（返回、任务、合并）
4. `src/exporter/mod.rs` — 从 trait/ExporterKind/ExporterManager 删除 `write_template_stats`
5. `src/exporter/csv/mod.rs` — 删除 `mod companion` 和 `write_template_stats`
6. `src/exporter/sqlite/mod.rs` — 删除 `write_template_stats`
7. 删除 `src/pipeline/aggregator.rs` 和 `src/pipeline/template_reporter.rs`

#### Plan 30-03: 测试清理 + 编译验证
1. `src/cli/run/tests.rs` — 删除 2 个模板测试, 修改 1 个测试
2. `src/exporter/csv/tests.rs` — 删除 4 个 write_template_stats 测试
3. `src/exporter/sqlite/tests.rs` — 删除所有 write_template_stats 测试（约 300 行）
4. `src/exporter/tests.rs` — 删除 write_template_stats 测试块（约 110 行）
5. `tests/integration.rs` — 删除 template 导入、`[template]` 断言、`test_e2e_template_normalization`、更新旧路径检测测试
6. 运行 `cargo build --release` + `cargo test` + `cargo clippy` 验证无错误

---

## 不要手写

N/A — 本阶段只移除代码，不引入新实现。

---

## 常见陷阱

### 陷阱 1: 忘记 `apply_one.rs` 中的 template 覆盖
`src/config/apply_one.rs` 有 4 个 `template.*` 路径处理（第 123-146 行）。如果只从 `Config` 结构体移除字段但忘记删除这些处理函数，`--set template.enable=true` 不会报错但配置也不会生效。必须确保 `apply_one` 方法中所有 `template.*` 路径都返回 `unknown()` 错误。

### 陷阱 2: `pipeline/mod.rs` 中 `normalize_template` 的 re-export
Phase 29 已将 `normalize_template` 从 `fingerprint.rs` 迁移到 `normalizer.rs`。Phase 30 的 `processor.rs` 第 138 行在模板聚合块内调用 `crate::pipeline::normalize_template()`。这个调用在删除模板聚合块时会被移除，但 `pipeline/mod.rs` 中的 `pub(crate) use fingerprint::normalize_template;` re-export 需要保留（其他路径可能仍使用）。

### 陷阱 3: 并行路径中 `TemplateAggregator` 的 map-reduce
`parallel.rs` 第 197-210 行有 map-reduce 合并逻辑。删除时需要注意：
- 删除 `merged_agg` 变量
- 删除每组结果的 merge 调用
- 调整 TaskResult 类型签名（从四元组变为三元组）
- 调整 `process_csv_parallel` 的返回类型（从三元组变为二元组）

### 陷阱 4: `processor.rs` 第 91 行的条件
```rust
let pm = if include_pm || aggregator.is_some() {
```
移除 aggregator 后，应改为 `if include_pm`，因为不再需要无条件调用 `parse_performance_metrics`。

### 陷阱 5: 测试文件分散在 4 个位置
模板相关测试分布在 `src/cli/run/tests.rs`、`src/exporter/csv/tests.rs`、`src/exporter/sqlite/tests.rs`、`src/exporter/tests.rs`、`src/config/validate.rs`、`src/config/apply_one.rs`、`src/pipeline/mod.rs`、`tests/integration.rs` 共 8 个文件。删除时需要遍历每个文件。

### 陷阱 6: `exporter/csv/companion.rs` 的依赖链
`write_companion_rows` 只被 `template_reporter.rs` 调用。移除 template_reporter 后，`companion.rs` 变成死代码。需要同时移除 `csv/mod.rs` 中的 `mod companion;` 声明。

---

## 代码示例

### 修改 `processor.rs` 第 91 行的条件
**修改前:**
```rust
let pm = if include_pm || aggregator.is_some() {
```

**修改后:**
```rust
let pm = if include_pm {
```

### 删除 `processor.rs` 中整个模板聚合块（第 131-161 行）
**修改前:**
```rust
// 模板聚合：仅对 DML 记录（有 tag）生效；PARAMS 记录不计入统计。
if let Some(ref mut agg) = aggregator {
    // 防御性检查：...
    if record.tag.is_some() {
        let tmpl_key = crate::pipeline::normalize_template(pm.sql.as_ref());
        // ...
        agg.observe(&tmpl_key, exectime_us, record.ts.as_ref(), meta.username.as_ref());
    }
}
```

**修改后:** 整个 `if let Some(ref mut agg)` 块被移除。

### 修改 `parallel.rs` 返回类型
**修改前:**
```rust
) -> Result<(Vec<(PathBuf, usize)>, usize, Option<TemplateAggregator>)> {
```

**修改后:**
```rust
) -> Result<(Vec<(PathBuf, usize)>, usize)> {
```

---

## 运行状态清单

| 类别 | 发现的项目 | 所需操作 |
|----------|-------------|-------------|
| 存储的数据 | 无 — `*_templates.csv` 和 `*_templates.db` 是运行期生成的文件，非持久状态 | 无 — 运行 `sqllog2db run` 不会再生成这些文件 |
| 实时服务配置 | 无 | — |
| OS 注册状态 | 无 | — |
| 密钥/环境变量 | 无 | — |
| 构建产物 | `hdrhistogram` 从 `Cargo.lock` 中移除 | 运行 `cargo update` 清理锁定文件 |

---

## 验证架构

### 测试框架

| 属性 | 值 |
|----------|-------|
| 框架 | cargo test + cargo clippy |
| 快速运行命令 | `cargo test --lib` |
| 完整套件命令 | `cargo test && cargo clippy --all-targets -- -D warnings` |

### 阶段需求 -> 测试映射

| 需求 ID | 行为 | 测试类型 | 自动化命令 | 文件存在？ |
|---------|----------|-----------|---------------------|--------------|
| RM-05a | `aggregator.rs` 和 `template_reporter.rs` 已删除 | 编译 | `cargo build --release` | 编译失败 = 存在引用 |
| RM-05b | `hdrhistogram` 依赖已移除 | 编译 | `cargo build --release` | 编译失败 = 隐含引用 |
| RM-05c | `[template]` 配置段从 Config 移除 | 单元 | `cargo test --lib config::tests` | ✅ |
| RM-05d | 未使用标识符警告为零 | lint | `cargo clippy --all-targets -- -D warnings` | ✅ |
| RM-05e | 核心 CSV/SQLite 导出不受影响 | 集成 | `cargo test --test integration test_e2e_csv_basic` | ✅ |

### Wave 0 缺口
- 无 — 现有测试基础设施覆盖所有阶段需求

---

## 安全领域

N/A — 本阶段不涉及安全相关的变更。移除的是 SQL 模板分析聚合和报告功能，不涉及认证、授权、输入验证或加密。

---

## 来源

### 主要（HIGH 置信度）
- 源代码: 通过 `grep -rn` 遍历所有 60 多个 .rs 文件确认每个引用点
- Cargo.toml: `hdrhistogram` 依赖声明和 `cargo tree` 确认无其他消费者
- `src/pipeline/aggregator.rs` — 447 行，全部与模板统计相关
- `src/pipeline/template_reporter.rs` — 278 行，全部与报告输出相关

### 次要（MEDIUM 置信度）
- 测试文件: 确认 4 个测试文件中包含 template_stats 相关测试
- 集成测试: `tests/integration.rs` 中 `test_e2e_template_normalization` 和 `[template]` 断言

---

## 元数据

**置信度细分:**
- 标准栈: HIGH — 通过源码确认
- 架构: HIGH — 所有引用点已遍历
- 陷阱: HIGH — 基于代码审查

**研究日期:** 2026-05-20
**有效期至:** 2026-06-20
