# Phase 43: Parser 新 API 适配与 Filter 重构 - Context

**Gathered:** 2026-05-24
**Status:** Ready for planning

<domain>
## Phase Boundary

利用 dm-database-parser-sqllog 2.0.0 的新 API 删除冗余的手动映射代码；同时重构 filter 模块，使 pre-scan 逻辑与 main-pass 逻辑在**同文件内**以独立函数的形式清晰分离。不拆出子模块，不增加新过滤能力。

</domain>

<decisions>
## Implementation Decisions

### Parser 新 API 适配
- **D-01:** 利用 2.0.0 新增 API（`from_reader`、新字段、`FilterBuilder` 等，研究员需从文档/changelog 确认具体可用 API）替换现有变通写法，目标是删除冗余的手动映射代码（可通过 `git diff` 验证行数减少）。
- **D-02:** prescan.rs 中现有注释"v1.1.0 的 LogParser 不再实现 rayon 的 IntoParallelRefIterator，所以先 collect 到 Vec 再 par_iter()"——如果 2.0.0 支持，可直接 `par_iter()` 无需 collect，删除此变通注释。
- **D-03:** FilterBuilder 链式过滤 API（2.0.0 新增）：如果能替代当前 `CompiledMetaFilters`/`CompiledSqlFilters` 中的部分逻辑，优先使用；但不强制全部迁移，以"减少冗余"为准，不做过度重构。

### Filter 模块重构
- **D-04:** pre-scan 逻辑与 main-pass 逻辑**不拆子模块**，在现有文件（`compiled.rs` 或 `prescan.rs`）内以独立函数 + 注释块分隔，保持职责清晰。
- **D-05:** `prescan.rs`（在 cli/run/ 下）已是独立文件，其内部结构调整：确保 `scan_for_trxids_by_transaction_filters` 等函数与 filter 编译逻辑不交叉，各自独立。
- **D-06:** `pipeline/filters/compiled.rs` 中的 pre-scan 相关方法（如有）与 main-pass 方法通过注释 section 清晰区隔（`// === Pre-scan ===` / `// === Main-pass ===` 风格）。

### 测试覆盖
- **D-07:** 重构后 filter 模块的单元测试场景数不低于重构前（`cargo test` 中过滤 filter 模块全部通过）。

### 质量门禁
- **D-08:** `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` 通过，无新增警告。

### Claude's Discretion
- 具体的 `// section` 注释格式
- 如果 2.0.0 某个新 API 适配后反而增加代码量，不强制用

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Parser 库（必读）
- `src/cli/run/processor.rs` — 当前 API 使用：`LogParserBuilder::new().build()`, `parser.iter()`, 字段访问
- `src/cli/run/prescan.rs` — 当前变通写法：collect Vec 再 par_iter，注释说明原因
- `src/cli/run/parallel.rs` — 并行路径的 parser 用法

### Filter 模块（必读，理解现有边界）
- `src/pipeline/filters/mod.rs` — filter 模块入口，pub/pub(crate) 边界
- `src/pipeline/filters/compiled.rs` — CompiledMetaFilters / CompiledSqlFilters 实现
- `src/pipeline/filters/types.rs` — filter 类型定义
- `src/cli/run/filter_processor.rs` — 构建 pipeline 的入口（`build_pipeline`）

### Requirements
- `.planning/REQUIREMENTS.md` §PARSER-02, §REFACTOR-01

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `CompiledMetaFilters` / `CompiledSqlFilters` — 编译后的过滤器，main-pass 使用
- `scan_for_trxids_by_transaction_filters` — pre-scan 入口函数，已在独立文件

### Established Patterns
- rayon par_iter 在 prescan 中用于并行处理收集到的 Vec — 如果 2.0.0 支持直接并行迭代，可简化
- `filter_map(Result::ok)` 忽略解析错误 — 与现有错误处理策略一致，保持不变

### Integration Points
- Phase 41 升级 2.0.0 后，本 Phase 在此基础上深化适配
- Phase 42 benchmark 基础设施用于验证重构后性能无回归

</code_context>

<specifics>
## Specific Ideas

- 研究员需要查阅 dm-database-parser-sqllog 2.0.0 的 docs.rs 或 README，确认 `from_reader`、FilterBuilder 的具体 API 签名
- 重构前后用 `git diff --stat` 验证代码行数变化，纳入验收证据

</specifics>

<deferred>
## Deferred Ideas

- AsyncLogParser tokio 异步接口 → 超出本 milestone 范围
- FilterBuilder 全量替代现有编译过滤器 → 仅删冗余，不做全量迁移

</deferred>

---

*Phase: 43-Parser 新 API 适配与 Filter 重构*
*Context gathered: 2026-05-24*
