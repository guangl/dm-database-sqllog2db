# Phase 41: 依赖升级与 Parser 库适配 - Context

**Gathered:** 2026-05-24
**Status:** Ready for planning

<domain>
## Phase Boundary

将所有 Cargo 依赖升级到最新兼容版本，重点是将 `dm-database-parser-sqllog` 从 1.1.0 升级到 2.0.0（major 版本）。目标：编译通过、无 deprecated 警告、所有测试通过。API 深度适配留给 Phase 43。

</domain>

<decisions>
## Implementation Decisions

### 目标版本策略
- **D-01:** 直接升级 `dm-database-parser-sqllog` 到 2.0.0（major 版本升级）。
- **D-02:** Phase 41 只做"升级 + 编译通过 + 无 deprecated 警告"，不做深度 API 重构（留给 Phase 43）。
- **D-03:** 其他依赖同步 `cargo update` 到最新兼容 minor/patch。

### 编译质量要求
- **D-04:** `cargo build --release` 无任何 `warning:` 行（包括 deprecated），`cargo test` 全部通过，`cargo clippy --all-targets -- -D warnings` 通过。
- **D-05:** Cargo.lock 中 `dm-database-parser-sqllog` 版本号高于当前 1.1.0。

### Claude's Discretion
- 如果 2.0.0 有编译级 breaking changes（如字段重命名），做最小化适配使编译通过，记录待深度重构的 TODO 供 Phase 43 参考。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 依赖配置
- `Cargo.toml` — 当前依赖版本，升级目标在此修改
- `Cargo.lock` — 锁定版本基线（v1.10 基线：dm-database-parser-sqllog = 1.1.0）

### Parser 库使用位置
- `src/cli/run/processor.rs` — `LogParserBuilder::new(file_path).build()` + `parser.iter()` + 字段访问 (exec_id, exectime, rowcount, sql)
- `src/cli/run/prescan.rs` — `LogParserBuilder::new(file_path).build()` + `.iter()` + `par_iter()` rayon 并行
- `src/cli/run/parallel.rs` — 并行 CSV 路径中的 parser 使用

### Requirements
- `.planning/REQUIREMENTS.md` §PARSER-01, §REFACTOR-02

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `LogParserBuilder::new(file_path).build()` — 当前 API，2.0.0 需确认是否保留或变更
- `parser.iter()` — 迭代器 API，2.0.0 可能变更为 `into_iter()` 或其他
- `result.exec_id`, `result.exectime`, `result.rowcount`, `result.sql` — 字段访问，需确认 2.0.0 字段名

### Established Patterns
- prescan.rs 注释已标注"v1.1.0 的 LogParser 不再实现 rayon 的 IntoParallelRefIterator"，说明存在版本适配注释，升级后可能可删除此变通

### Integration Points
- 3 个文件使用 parser API：`processor.rs`, `prescan.rs`, `parallel.rs`
- 编译错误会精确定位需要修改的位置

</code_context>

<specifics>
## Specific Ideas

- 升级后先跑 `cargo build 2>&1 | grep -E "error|deprecated|warning"` 确认问题范围
- 如果 2.0.0 有 changelog/CHANGELOG.md，研究员应读取以了解 breaking changes

</specifics>

<deferred>
## Deferred Ideas

- 利用新 API（FilterBuilder、from_reader 等）删除冗余映射代码 → Phase 43
- AsyncLogParser tokio 异步接口 → 超出本 milestone 范围，暂不考虑

</deferred>

---

*Phase: 41-依赖升级与 Parser 库适配*
*Context gathered: 2026-05-24*
