# Phase 50: SQL 标准化引擎 - Context

**Gathered:** 2026-06-01
**Status:** Ready for planning

<domain>
## Phase Boundary

实现内部 SQL 标准化模块：将 SQL 文本中的字符串字面量和数字字面量替换为 `?` 占位符，从而把参数不同但模板相同的 SQL 调用归并为同一组。此阶段仅提供 `normalize_sql` 函数，不涉及 CLI 或输出逻辑。

</domain>

<decisions>
## Implementation Decisions

### 模块位置
- **D-01:** 新建 `src/stats/` 目录，`normalize_sql` 函数放在 `src/stats/normalize.rs`（或 `src/stats/mod.rs` 中）。Phase 51/52 的聚合器也放在此目录，不污染 `pipeline/` 模块。
- **D-02:** 不要放在 `src/pipeline/normalizer.rs` 旁边——现有 normalizer.rs 是参数绑定替换器（PARAMS → `?`），职责完全不同。

### 实现方式
- **D-03:** 使用字符扫描状态机（char-by-char state machine），一次遍历完成替换。不使用 regex crate（虽然已在依赖中），以便精确处理转义引号（`''` 两个单引号）和数字边界。
- **D-04:** 替换规则：
  - 单引号字符串（含 `''` 转义引号）→ `?`
  - 整数和浮点数字面量（非标识符中的数字）→ `?`
  - 不含字面量的 SQL 原样返回（无误替换）

### 测试覆盖
- **D-05:** 单元测试覆盖至少 5 种典型模式（简单 WHERE 条件、多字面量、带转义引号的字符串、纯无字面量 SQL、INSERT VALUES 多列）。测试放在 `src/stats/normalize.rs` 内的 `#[cfg(test)]` 块。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 50: SQL 标准化引擎" — Goal、Success Criteria（5 条）
- `.planning/REQUIREMENTS.md` §STATS-06 — 用户需求原文

### 现有关联模块（理解职责边界，不复用实现）
- `src/pipeline/normalizer.rs` — 参数绑定替换器（PARAMS → SQL），与本 Phase 职责相反，不复用

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/pipeline/normalizer.rs::ParamValue`：了解现有 `?` 占位符的使用方式，但本 Phase 不复用此类型
- `regex` crate 已在 `Cargo.toml` 中——但本 Phase 决定用状态机不用 regex

### Established Patterns
- 测试放在模块文件内的 `#[cfg(test)]` 块（参照 `src/pipeline/normalizer.rs` 末尾测试区域）
- 函数签名风格：`pub fn normalize_sql(sql: &str) -> String`（或 `Cow<str>` 如果不变时避免分配）

### Integration Points
- Phase 51/52 会在同一个 `src/stats/` 目录中调用 `normalize_sql`

</code_context>

<specifics>
## Specific Ideas

- ROADMAP Success Criteria 中的示例直接作为第一个单元测试：`normalize_sql("SELECT * FROM t WHERE id = 42 AND name = 'alice'")` → `"SELECT * FROM t WHERE id = ? AND name = ?"`

</specifics>

<deferred>
## Deferred Ideas

None — 讨论始终在阶段范围内。

</deferred>

---

*Phase: 50-SQL 标准化引擎*
*Context gathered: 2026-06-01*
