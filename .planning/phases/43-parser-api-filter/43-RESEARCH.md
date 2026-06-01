# Phase 43: Parser 新 API 适配与 Filter 重构 - Research

**Researched:** 2026-05-24
**Domain:** Rust — dm-database-parser-sqllog 2.0.0 API 适配 / pipeline filter 模块重构
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Parser 新 API 适配**
- D-01: 利用 2.0.0 新增 API（`from_reader`、新字段、`FilterBuilder` 等）替换现有变通写法，目标是删除冗余的手动映射代码（可通过 `git diff` 验证行数减少）。
- D-02: prescan.rs 中现有注释 "v1.1.0 的 LogParser 不再实现 rayon 的 IntoParallelRefIterator，所以先 collect 到 Vec 再 par_iter()" ——如果 2.0.0 支持，可直接 `par_iter()` 无需 collect，删除此变通注释。
- D-03: FilterBuilder 链式过滤 API（2.0.0 新增）：如果能替代当前 `CompiledMetaFilters`/`CompiledSqlFilters` 中的部分逻辑，优先使用；但不强制全部迁移，以"减少冗余"为准，不做过度重构。

**Filter 模块重构**
- D-04: pre-scan 逻辑与 main-pass 逻辑不拆子模块，在现有文件（`compiled.rs` 或 `prescan.rs`）内以独立函数 + 注释块分隔，保持职责清晰。
- D-05: `prescan.rs`（在 cli/run/ 下）已是独立文件，其内部结构调整：确保 `scan_for_trxids_by_transaction_filters` 等函数与 filter 编译逻辑不交叉，各自独立。
- D-06: `pipeline/filters/compiled.rs` 中的 pre-scan 相关方法（如有）与 main-pass 方法通过注释 section 清晰区隔（`// === Pre-scan ===` / `// === Main-pass ===` 风格）。

**测试覆盖**
- D-07: 重构后 filter 模块的单元测试场景数不低于重构前（`cargo test` 中过滤 filter 模块全部通过）。

**质量门禁**
- D-08: `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` 通过，无新增警告。

### Claude's Discretion
- 具体的 `// section` 注释格式
- 如果 2.0.0 某个新 API 适配后反而增加代码量，不强制用

### Deferred Ideas (OUT OF SCOPE)
- AsyncLogParser tokio 异步接口 → 超出本 milestone 范围
- FilterBuilder 全量替代现有编译过滤器 → 仅删冗余，不做全量迁移
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PARSER-02 | 利用新 API（如 `from_reader` 或新字段）替换现有变通写法，删除冗余的手动映射代码 | 已确认 2.0.0 的具体新 API，并识别 prescan.rs 中的 `i64::from(result.rowcount)` 转换冗余和 collect 变通写法 |
| REFACTOR-01 | filter 模块重构后，pre-scan 与 main-pass 逻辑边界清晰，单元测试覆盖率不低于重构前，代码行数减少或复杂度降低（可 diff 验证） | 已识别具体的注释区隔方案和现有测试基线（50 个 filter 测试） |
</phase_requirements>

---

## Summary

Phase 43 涉及两个独立但相关的任务：（1）利用 dm-database-parser-sqllog 2.0.0 的新 API 删除项目中残留的变通代码；（2）对 filter 模块做轻量级重构，用注释 section 清晰区分 pre-scan 与 main-pass 的逻辑边界。

通过直接阅读 2.0.0 源码，研究员已完整掌握新 API 的签名和行为。关键发现如下：

1. **`from_reader` 不存在于 2.0.0**。CONTEXT.md 中提到的 `from_reader` 在 2.0.0 实际 API 中并未出现。2.0.0 的 `LogParserBuilder::new(path)` 在内部已改为直接 `fs::read()` 全量读取，构建 API 与 1.1.0 相同。"变通写法"的改善机会在于其他方面（见下文）。

2. **`FilterBuilder` 是真正的新 API**（2.0.0 全新引入）。它提供链式谓词组合，可以替代 prescan.rs 中现有的手动字段匹配逻辑（`filters.indicators.matches(...)` 和 `filters.sql.matches(...)` 两段）。

3. **`rowcount` 类型在 2.0.0 仍为 `u32`**（与 1.1.0 一致），因此 prescan.rs 中的 `i64::from(result.rowcount)` 类型转换可以通过调整 `IndicatorFilters::matches` 的参数签名来消除。

4. **`LogIterator` 不实现 `rayon::IntoParallelRefIterator`**（2.0.0 同样未实现），因此 prescan.rs 中 collect-then-par_iter 的变通写法**无法**依靠新 API 消除——该注释需要更新为反映准确原因（"LogIterator 不实现 rayon trait，需先 collect"），而非声称"v1.1.0 不支持"。

**主要建议：**
- prescan.rs 中的 `i64::from(result.rowcount)` 可通过修改 `IndicatorFilters::matches` 签名为 `(exec_id: i64, runtime_ms: f32, row_count: u32)` 来消除
- prescan.rs 中 collect 变通注释需从 "v1.1.0..." 改为准确表述
- `FilterBuilder` 可用于替代 prescan.rs 的手动匹配代码，但收益有限（约 10 行换 5 行），以"减少冗余"为准
- filter 模块重构：在 `compiled.rs` 和 `prescan.rs` 中添加 `// === Pre-scan ===` / `// === Main-pass ===` 注释 section

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Parser API 适配（LogParserBuilder） | 数据解析层（parser） | CLI run 层 | LogParserBuilder 在 processor.rs / prescan.rs / parallel.rs 三处调用，均属 run 层内部 |
| Pre-scan 逻辑（trxid 收集） | CLI run 层（prescan.rs） | pipeline/filters | prescan 是 run 阶段的编排逻辑，调用 pipeline/filters 的类型 |
| Main-pass 过滤（CompiledMetaFilters） | pipeline/filters 层 | CLI run 层（filter_processor.rs） | 核心热路径过滤在 compiled.rs，run 层仅通过 FilterProcessor 调用 |
| 逻辑边界注释区隔 | pipeline/filters/compiled.rs | cli/run/prescan.rs | 两文件都需添加 section 注释 |

---

## Standard Stack

本 Phase 不引入新依赖，仅使用已有依赖：

### Core（已在 Cargo.toml 中）
| Library | Version | Purpose | 说明 |
|---------|---------|---------|------|
| dm-database-parser-sqllog | 2.0.0 | SQL 日志解析器 | Phase 41 已升级，本 Phase 深化适配 |
| rayon | 1.12 | 并行处理 | prescan 两级并行不变 |
| regex | 1 | 正则编译 | CompiledMetaFilters 使用 |

无需 `npm install` 或其他包安装操作。

### 无外部包安装

本 Phase 是纯代码重构，不引入新 crate。

---

## Package Legitimacy Audit

本 Phase 不安装任何新外部包，跳过此节。

---

## Architecture Patterns

### 系统数据流（重构后不变）

```
Input .log files
    ↓ LogParserBuilder::new(path).build()  [processor.rs / prescan.rs / parallel.rs]
    ↓ LogParser::iter()  -> LogIterator<'_>
    ↓ [Pre-scan phase] scan_log_file_for_matches()
    |    collect Vec<Sqllog>  -> par_iter()  -> filter_map
    |    产出 matched trxids
    ↓ [Main-pass phase] FilterProcessor::process_with_meta()
    |    CompiledMetaFilters::should_keep()  -- include AND / exclude OR-veto
    |    CompiledSqlFilters::matches()       -- record-level SQL filter
    ↓ ExporterManager -> CSV / SQLite
```

### 关键新 API 签名（VERIFIED: 直接读取 2.0.0 源码）

```rust
// FilterBuilder — 2.0.0 新增，链式谓词组合
use dm_database_parser_sqllog::FilterBuilder;

let filter = FilterBuilder::new()
    .exec_time_gte(100.0)    // >= min_ms（与 filter_by_exec_time 语义一致）
    .sql_contains("SELECT")
    .username_eq("alice")
    .build();                // -> Filter

// Filter::matches — AND 短路求值，所有谓词必须通过
filter.matches(&sqllog_record)  // -> bool

// LogIterator 新方法（2.0.0）
parser.iter().apply_filter(filter)            // Err 记录被丢弃
parser.iter().apply_filter_keep_errors(filter) // Err 记录透传
parser.iter().skip_errors()                   // -> impl Iterator<Item = Sqllog>
```

### FilterBuilder 可替代的 prescan.rs 逻辑片段

当前代码（prescan.rs 第 28-41 行）：
```rust
// 当前：手动字段匹配
.filter_map(|result| {
    let mut matched = false;
    if filters.indicators.matches(
        result.exec_id,
        result.exectime,
        i64::from(result.rowcount),  // 不必要的类型转换
    ) {
        matched = true;
    }
    if !matched && filters.sql.has_filters() {
        matched = filters.sql.matches(&result.sql);
    }
    if matched { Some(result.trxid.clone()) } else { None }
})
```

重构后（使用 FilterBuilder 替代，或直接修正类型签名）：
```rust
// 方案 A：修改 IndicatorFilters::matches 签名消除 i64::from
// 将参数 row_count: i64 改为 row_count: u32
// 调用改为: filters.indicators.matches(result.exec_id, result.exectime, result.rowcount)

// 方案 B：FilterBuilder 替代（约减少 5 行）
// 注意：FilterBuilder 只支持字面量谓词，无法直接替代动态配置的 IndicatorFilters
// 因此完全替代不现实；部分替代 sql.contains 可能可行
```

**研究员结论：** 方案 A（修正类型签名）是最有价值的代码简化，直接消除 `i64::from` 转换。方案 B 的 FilterBuilder 替代因需要在运行时从配置动态构建谓词，实现代价较高（需 closure 捕获配置），实际代码量不减反增。**不推荐强制使用 FilterBuilder 替换 prescan 逻辑**（符合 D-03 的"不强制"条款）。

### Anti-Patterns to Avoid
- **错误：** 把 `FilterBuilder` 当作 prescan 主逻辑的完全替代。`FilterBuilder` 适合静态/字面量谓词，项目中的过滤器来自用户配置的动态数据（Vec<String>、HashSet<i64>），需要闭包捕获，会导致代码膨胀。
- **错误：** 删除 collect-then-par_iter 变通，直接 `parser.iter().par_bridge()`。`LogIterator` 在 2.0.0 同样未实现 rayon trait，`par_bridge()` 是另一种可选方案，但会改变现有并行行为，不在本 Phase 范围内。
- **错误：** 在 `compiled.rs` 的 main-pass 方法中添加 pre-scan 逻辑。两者职责已分离，重构只需添加注释 section，不移动代码。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 静态条件组合过滤 | 手写 AND/OR 谓词链 | `FilterBuilder` | 已提供 AND 短路求值、多字段谓词，无需自建 |
| 正则编译缓存 | 手动 `Regex::new()` | `CompiledMetaFilters`（现有） | 已在启动时预编译，热路径避免重复编译 |
| 并行文件扫描 | 手写线程池 | rayon `ThreadPoolBuilder` + `par_iter()` | 已有两级嵌套并行实现 |

**Key insight:** 2.0.0 的 `FilterBuilder` 设计目标是简单用例（如 demo/单次查询），项目现有的 `CompiledMetaFilters` 体系（正则预编译 + trxid HashSet + AND/OR-veto 语义）更适合生产级配置驱动过滤，不应被全量替代。

---

## Common Pitfalls

### Pitfall 1: 假设 `from_reader` 存在
**What goes wrong:** CONTEXT.md 中提到 `from_reader` 是 2.0.0 新增 API，但实际查阅 2.0.0 源码发现该方法不存在。如果规划任务包含"迁移到 from_reader"，执行时会直接编译失败。
**Why it happens:** CONTEXT.md 中的 API 列表来自 discuss 阶段的预测，非经验证的实际 API。
**How to avoid:** 以本 RESEARCH.md 中的 API 为准。2.0.0 实际新增的公共 API 为：`FilterBuilder`、`Filter`、`LogIterator::apply_filter`、`LogIterator::apply_filter_keep_errors`、`LogIterator::skip_errors`、`LogParserBuilder::encoding_hint`。
**Warning signs:** 编译错误 "method not found `from_reader`"。

### Pitfall 2: 强制用 FilterBuilder 替代动态配置过滤
**What goes wrong:** FilterBuilder 只支持字面量谓词（`filter.username_eq("alice")`），项目中的过滤条件来自用户配置（`Vec<String>` patterns、`HashSet<i64>` exec_ids），必须在闭包中捕获，无法用 FilterBuilder 的内置方法直接表达。
**Why it happens:** FilterBuilder API 看起来功能丰富，误以为可以完全替代。
**How to avoid:** 仅在字面量匹配场景使用（如 benchmark 或测试工具），不替换 CompiledMetaFilters 体系。
**Warning signs:** 需要写 `FilterBuilder::add()` 或 `.build()` 后仍需手动处理配置转换。

### Pitfall 3: 修改 IndicatorFilters::matches 签名时遗漏调用方
**What goes wrong:** 将 `row_count: i64` 改为 `row_count: u32` 后，只改了 prescan.rs 的调用点，忘记检查其他地方是否也调用了该方法。
**Why it happens:** grep 不够全面。
**How to avoid:** 修改签名后，`cargo build` 编译器会报所有不兼容调用点，逐一修复即可。当前唯一调用点就是 `prescan.rs:31-35`（已确认）。
**Warning signs:** 编译错误 "mismatched types: expected u32, found i64"。

### Pitfall 4: 注释 section 破坏 `#[cfg(test)]` 内嵌测试模块的位置
**What goes wrong:** 在 compiled.rs 添加 `// === Pre-scan ===` 注释时，意外将其插入测试模块内部，导致逻辑组织混乱。
**Why it happens:** compiled.rs 末尾有 `#[cfg(test)] #[path = "compiled_tests.rs"] mod compiled_tests;`，注释应插在该行之前。
**How to avoid:** section 注释只放在 impl 块内的方法之间，不跨越 mod 边界。

### Pitfall 5: collect 变通注释更新不充分
**What goes wrong:** 只删除"v1.1.0..."字样，但未说明为什么 2.0.0 也不支持，留下无法理解的注释。
**Why it happens:** 修改注释时注意力集中在删除而非解释。
**How to avoid:** 新注释应写明"`LogIterator` 未实现 `rayon::IntoParallelIterator` trait，需先 `collect` 到 `Vec` 再并行"——这在 2.0.0 中依然成立，且与版本无关。

---

## Code Examples

### 新增 API：FilterBuilder 链式用法
```rust
// Source: 直接读取 2.0.0 src/filter/builder.rs
use dm_database_parser_sqllog::FilterBuilder;

// 组合过滤：AND 语义，所有谓词必须通过
let filter = FilterBuilder::new()
    .exec_time_gte(100.0)
    .sql_contains("SELECT")
    .build();

// 应用到迭代器（Err 记录丢弃）
for result in parser.iter().apply_filter(filter) {
    let record = result?;
    // ...
}
```

### 新增 API：encoding_hint
```rust
// Source: 直接读取 2.0.0 src/parser/builder.rs
use dm_database_parser_sqllog::{LogParserBuilder, FileEncodingHint};

// 强制指定编码（跳过自动探测，轻微提升性能）
let parser = LogParserBuilder::new("sqllog.txt")
    .encoding_hint(FileEncodingHint::Utf8)
    .build()?;
```

### 修改 IndicatorFilters::matches 签名（消除 i64::from）
```rust
// Before (src/pipeline/filters/mod.rs):
pub fn matches(&self, exec_id: i64, runtime_ms: f32, row_count: i64) -> bool {
    // ...
    if let Some(min_r) = self.min_row_count {
        if row_count >= i64::from(min_r) {   // min_row_count: Option<u32>
            return true;
        }
    }
}

// After: row_count 直接用 u32，消除调用方的 i64::from(result.rowcount)
pub fn matches(&self, exec_id: i64, runtime_ms: f32, row_count: u32) -> bool {
    // ...
    if let Some(min_r) = self.min_row_count {
        if row_count >= min_r {              // 直接比较，无需转换
            return true;
        }
    }
}
// 调用方 prescan.rs 改为：
// filters.indicators.matches(result.exec_id, result.exectime, result.rowcount)
```

### compiled.rs 注释 section 风格（D-06）
```rust
impl CompiledMetaFilters {
    // ===== 构造 =====
    pub(crate) fn try_from_include_exclude(...) -> ... { ... }

    // ===== Pre-scan 辅助 =====
    // 以下方法在 prescan 阶段用于快速判断是否存在过滤器
    #[must_use]
    pub(crate) fn has_filters(&self) -> bool { ... }

    #[must_use]
    pub(crate) fn has_any_filters(&self) -> bool { ... }

    // ===== Main-pass（热路径）=====
    // 以下方法在主扫描循环中对每条记录调用
    #[inline]
    #[must_use]
    pub(crate) fn should_keep(&self, meta: &RecordMeta) -> bool { ... }

    fn exclude_veto(&self, meta: &RecordMeta) -> bool { ... }

    fn include_and(&self, meta: &RecordMeta) -> bool { ... }
}
```

---

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| LogParser 不支持编码提示，自动探测开销固定 | 2.0.0 新增 `encoding_hint()`，可跳过自动探测 | 可选优化，本 Phase 无需使用 |
| 无链式过滤 API | 2.0.0 新增 `FilterBuilder` + `apply_filter` | 简化简单用例，不替代生产级配置驱动过滤 |
| 无 `skip_errors()` | 2.0.0 新增 `LogIterator::skip_errors()` | 可替代 `.filter_map(Result::ok)`，可选简化 |

**Deprecated/outdated:**
- prescan.rs 中的注释 "v1.1.0 的 LogParser 不再实现 rayon 的 IntoParallelRefIterator"：该表述已过时，应改为与版本无关的准确描述。
- 代码注释中的 "v1.1.0 所有字段已物化" 说法：Phase 43 可将这类注释更新为 "2.0.0 所有字段已物化"（非功能性清理，属 Claude's Discretion）。

---

## Runtime State Inventory

本 Phase 为纯代码重构（修改源码和注释），无数据迁移、无运行时状态变更，跳过此节。

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust/cargo | 编译和测试 | ✓ | 1.85+ (edition 2024) | — |
| dm-database-parser-sqllog 2.0.0 | API 适配 | ✓ | 已在 Cargo.toml 中，Phase 41 升级完成 | — |
| rayon 1.12 | prescan 并行 | ✓ | 已在 Cargo.toml | — |
| cargo clippy | 质量门禁 D-08 | ✓ | 随 Rust 工具链 | — |

**Missing dependencies with no fallback:** 无

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test (`cargo test`) |
| Config file | 无单独配置文件 |
| Quick run command | `cargo test filter` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PARSER-02 | `i64::from(result.rowcount)` 消除后编译通过 | compile | `cargo build` | ✅ |
| PARSER-02 | `IndicatorFilters::matches(u32)` 语义不变 | unit | `cargo test indicator_matches` | ✅ `pipeline/filters/mod.rs` |
| REFACTOR-01 | filter 测试数不低于重构前（≥50） | unit | `cargo test filter` | ✅ |
| REFACTOR-01 | pre-scan 与 main-pass 注释 section 存在 | 人工 diff review | `git diff src/pipeline/filters/compiled.rs` | — |
| D-08 | clippy 无新增警告 | lint | `cargo clippy --all-targets -- -D warnings` | — |

**基线测试数量（重构前）：**
- `cargo test filter` → **50 个 filter 模块测试**（lib）+ 9 个集成测试
- 全套：`cargo test` → **487 个测试**（215 lib + 239 lib 含库依赖 + 33 集成）

注：以上数字为研究阶段当前状态，规划任务应将 "≥50" 作为 filter 模块测试下限。

### Wave 0 Gaps

无 — 现有测试基础设施完整覆盖本 Phase 所有需求，不需要新建测试文件或 fixture。

---

## Security Domain

本 Phase 不涉及网络、用户输入、认证、加密等安全敏感操作。仅为内部代码重构，跳过此节。

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `LogIterator` 在 2.0.0 不实现 rayon IntoParallelIterator（collect 变通依然需要） | Common Pitfalls / Code Examples | 若 2.0.0 已支持，collect 可省略，但注释更新依然正确 |

注：A1 已通过阅读 2.0.0 完整源码（lib.rs、parser/mod.rs、parser/iterator.rs）验证——源码中无任何 rayon trait 实现，置信度 HIGH，此处仅作形式保留。

---

## Open Questions (RESOLVED)

1. **`encoding_hint` 是否值得在 processor.rs 中启用？**
   - What we know: 2.0.0 新增 `LogParserBuilder::encoding_hint(FileEncodingHint::Utf8)`，可跳过自动探测（头部 64KB + 尾部 4KB 采样）
   - What's unclear: 对 1.55M records/sec 热路径的实际影响量（encode detection 发生在 build 阶段，非每条记录）
   - RESOLVED: 本 Phase 不引入，留给 Phase 44 性能优化阶段评估

2. **`skip_errors()` 替代 `filter_map(Result::ok)` 是否有价值？**
   - What we know: prescan.rs 第 25 行使用 `.filter_map(std::result::Result::ok)`，2.0.0 的 `skip_errors()` 等价
   - What's unclear: 是否算"减少冗余"（D-01）还是纯风格变化
   - RESOLVED: 属于 Claude's Discretion，本 Phase 不强制替换，clippy 无警告即可接受现状

---

## Sources

### Primary (HIGH confidence)
- 直接读取 `~/.cargo/registry/src/.../dm-database-parser-sqllog-2.0.0/src/lib.rs` — 公共 API 全集
- 直接读取 `~/.cargo/registry/src/.../dm-database-parser-sqllog-2.0.0/src/filter/builder.rs` — FilterBuilder 完整实现
- 直接读取 `~/.cargo/registry/src/.../dm-database-parser-sqllog-2.0.0/src/parser/iterator.rs` — LogIterator 新方法
- 直接读取 `~/.cargo/registry/src/.../dm-database-parser-sqllog-2.0.0/src/record.rs` — Sqllog 字段类型（rowcount: u32 确认）
- 直接读取项目源码 `src/cli/run/prescan.rs`、`src/pipeline/filters/compiled.rs`、`src/pipeline/filters/mod.rs` — 现有代码状态

### Secondary (MEDIUM confidence)
- `cargo test filter` 输出 — 确认当前 filter 模块 50 个测试（重构前基线）
- 比对 1.1.0 与 2.0.0 的 `Sqllog` struct — 确认字段类型未变（rowcount: u32, exec_id: i64, exectime: f32）

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — 直接读取 Cargo.toml 和 2.0.0 源码
- Architecture: HIGH — 直接阅读所有相关源码文件
- 新 API 存在性: HIGH — 直接读取 2.0.0 lib.rs 公共导出
- Pitfalls: HIGH — 基于代码实证（from_reader 不存在已确认）

**Research date:** 2026-05-24
**Valid until:** 2026-06-24（dm-database-parser-sqllog 依赖版本固定，稳定性高）
