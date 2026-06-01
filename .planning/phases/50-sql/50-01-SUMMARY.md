---
phase: 50-sql
plan: 01
subsystem: stats
tags: [rust, state-machine, sql-normalization, memchr]

# Dependency graph
requires: []
provides:
  - "pub fn normalize_sql(sql: &str) -> String — 将 SQL 字面量替换为 ? 占位符"
  - "src/stats/ 模块骨架，Phase 51/52 聚合器落脚点"
  - "crate::stats::normalize_sql 可通过 pub re-export 直接调用"
affects: [51-stats-cli, 52-exporter]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "字符扫描状态机（char-by-byte state machine）用于 SQL 字面量识别"
    - "辅助函数拆分：skip_string_literal + skip_number_literal，各自 ≤20 行"
    - "prev_was_ident_char 标志区分数字字面量与标识符中的数字"

key-files:
  created:
    - src/stats/mod.rs
    - src/stats/normalize.rs
  modified:
    - src/lib.rs

key-decisions:
  - "使用字节级状态机而非 regex，精确处理 '' 转义引号和数字边界（CONTEXT D-03）"
  - "normalize_sql 返回 String（非 Cow<str>），保持简单；未来如有分配压力再改"
  - "String::from_utf8(...).expect(...) 替代 unsafe from_utf8_unchecked，遵循项目 lint 要求"
  - "添加 #[must_use] + # Panics doc，满足 clippy::must_use_candidate 和 clippy::missing_panics_doc"

patterns-established:
  - "测试放在模块文件内 #[cfg(test)] 块，与实现同文件（项目惯例）"
  - "辅助函数使用 bytes/cursor/len 描述性变量名，无单字母变量"

requirements-completed: [STATS-06]

# Metrics
duration: 15min
completed: 2026-06-01
---

# Phase 50 Plan 01: SQL 标准化引擎 Summary

**字节级状态机 normalize_sql 函数：将 SQL 字符串/数字字面量替换为 `?` 占位符，支持 `''` 转义和浮点，标识符中的数字保持原样**

## Performance

- **Duration:** 约 15 min
- **Started:** 2026-06-01T00:00:00Z
- **Completed:** 2026-06-01T00:15:00Z
- **Tasks:** 1
- **Files modified:** 3

## Accomplishments
- 实现 `pub fn normalize_sql(sql: &str) -> String`，单遍字节扫描，无外部依赖新增
- 7 个单元测试全部通过，覆盖 CONTEXT D-05 的 5 种典型模式 + 2 个边界案例
- `cargo clippy --all-targets -- -D warnings` 与 `cargo fmt --check` 全绿，无任何警告

## Task Commits

每个任务原子提交：

1. **Task 1: 创建 stats 模块骨架与 normalize_sql 实现 + 单元测试** - `ee05a21` (feat)

**Plan metadata:** （见下方 final commit）

## Files Created/Modified
- `src/stats/mod.rs` — stats 模块根，声明 `pub mod normalize` 并 re-export `normalize_sql`
- `src/stats/normalize.rs` — `normalize_sql` 函数实现（30 行）+ `skip_string_literal`（17 行）+ `skip_number_literal`（15 行）+ 7 个单元测试
- `src/lib.rs` — 新增 `pub mod stats;` 一行

## 测试覆盖矩阵

| 测试函数 | 对应契约 | CONTEXT D-05 |
|----------|----------|--------------|
| `test_basic_where_number_and_string` | 契约 1：混合字面量 | 简单 WHERE 条件 |
| `test_multiple_numeric_literals` | 契约 2：多连续数字 | 多字面量 |
| `test_escaped_quote_in_string` | 契约 3：`''` 转义引号 | 带转义引号字符串 |
| `test_no_literals_unchanged` | 契约 4：无字面量原样返回 | 纯无字面量 SQL |
| `test_insert_multiple_columns_with_float` | 契约 5：浮点数 | INSERT VALUES 多列 |
| `test_identifier_with_digits_not_replaced` | 契约 6：标识符边界 | （额外边界测试）|
| `test_unclosed_string_does_not_panic` | 契约 7：未闭合字符串 | （额外健壮性测试）|

## Decisions Made
- 使用 `#[must_use]` 属性 + `# Panics` 文档注释以满足 clippy 要求（Rule 1 自动修复）
- `String::from_utf8(...).expect(...)` 替代 `unsafe from_utf8_unchecked`，与 RESEARCH "Open Question 2" 推荐一致
- 函数拆分为三个：主函数 `normalize_sql`（30 行）+ `skip_string_literal`（17 行）+ `skip_number_literal`（15 行），均在 CLAUDE.md 40 行限制内

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 添加 `#[must_use]` + `# Panics` 文档注释**
- **Found during:** Task 1 (质量门禁 clippy 阶段)
- **Issue:** `cargo clippy -D warnings` 报告 `clippy::must_use_candidate`（函数返回值未标注 `#[must_use]`）和 `clippy::missing_panics_doc`（含 `expect` 的函数文档缺少 `# Panics` 节）
- **Fix:** 在 `normalize_sql` 函数上添加 `#[must_use]` 属性，并在文档注释中增加 `# Panics` 章节说明实践中不会 panic
- **Files modified:** `src/stats/normalize.rs`
- **Verification:** `cargo clippy --all-targets -- -D warnings` 退出码 0，无任何警告
- **Committed in:** `ee05a21`（Task 1 提交）

---

**Total deviations:** 1 auto-fixed (1 Rule 1 bug fix)
**Impact on plan:** 必要修复，满足项目 clippy lint 要求。无范围蔓延。

## Issues Encountered
无其他问题。

## Known Stubs
无——`normalize_sql` 完整实现，无 placeholder 或 TODO 残留。

## Threat Flags
无——此 Phase 为纯内存字符串处理函数，不涉及 IO、网络、认证或持久化。

## Next Phase Readiness
- Phase 51（stats 子命令脚手架）可直接 `use crate::stats::normalize_sql;`
- Phase 52（统计输出与 Exporter 集成）同样可通过 `crate::stats` 路径调用
- 如果 Phase 51/52 显示 `normalize_sql` 是热路径瓶颈，可考虑改为 `Cow<str>` 返回类型以减少分配
- 无已知阻塞项

## Self-Check: PASSED
- `src/lib.rs` 含 `pub mod stats;` — FOUND
- `src/stats/mod.rs` 存在且含 `pub mod normalize;` — FOUND
- `src/stats/normalize.rs` 存在且含 `pub fn normalize_sql` — FOUND
- Task 1 commit `ee05a21` — FOUND
- 7 个测试全部通过 — VERIFIED
- `cargo clippy --all-targets -- -D warnings` 退出码 0 — VERIFIED
- `cargo fmt --check` 退出码 0 — VERIFIED

---
*Phase: 50-sql*
*Completed: 2026-06-01*
