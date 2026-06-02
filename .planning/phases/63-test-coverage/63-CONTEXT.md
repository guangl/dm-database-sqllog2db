# Phase 63: 测试覆盖提升 - Context

**Gathered:** 2026-06-03
**Status:** Ready for planning

<domain>
## Phase Boundary

使用 `cargo-llvm-cov` 生成覆盖率报告，识别至少 3 个覆盖不足区域（行覆盖率 < 60% 的函数/模块），并按分析结果补全关键路径测试，使识别到的区域行覆盖率达到 80% 以上（或文档化无法测试的原因）。不修改生产代码逻辑，不依赖外部服务或网络。

</domain>

<decisions>
## Implementation Decisions

### 覆盖率工具选择

[auto] Q: "使用哪种覆盖率工具？" → Selected: "cargo-llvm-cov" (recommended default)

- **D-01:** 使用 `cargo llvm-cov --html` 生成 HTML 覆盖率报告（`cargo-llvm-cov 0.8.5` 已安装，ROADMAP.md 明确提及）。报告路径：`target/llvm-cov/html/`。同时用 `cargo llvm-cov --text` 输出文本摘要便于脚本分析。

### 覆盖率优先区域

[auto] Q: "优先补全哪些模块的测试？" → Selected: "过滤器 + exporter + 错误路径" (recommended default)

- **D-02:** 优先顺序：
  1. `src/pipeline/filters/mod.rs` — 过滤器 edge case（空值、多值 AND/OR 组合、指标过滤器边界）
  2. `src/exporter/csv/writer.rs` / `src/exporter/sqlite/mod.rs` — exporter 单元逻辑（字段序列化、overwrite/append 模式、错误路径）
  3. `src/error.rs` — 各变体的 `is_fatal()`、`severity()`、`suggestion()` 方法（已有部分测试，补全缺失变体）
  4. Phase 60 修改的错误传播路径（如被 Phase 60 改动则视情况补充）

### 测试策略

[auto] Q: "优先单元测试还是集成测试？" → Selected: "单元测试优先" (recommended default)

- **D-03:** 在各模块的 `mod tests` 中添加单元测试（`#[test]`），精准覆盖低覆盖率函数。集成测试（`src/cli/run/tests.rs`）仅在单元测试难以覆盖的端到端路径时才添加。
- **D-04:** 对于"难以测试"的路径（如依赖操作系统 I/O 错误、特定文件系统状态），在 Phase 计划文档中明确标注并说明理由，不强行添加脆弱测试。

### Claude's Discretion

- 覆盖率报告分析后，具体补充哪 3+ 个区域由分析结果决定——规划阶段先预测，执行时以实际报告为准
- 测试数据：使用 `tempfile` 创建临时目录/文件（项目已有此模式），无外部依赖

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 63: 测试覆盖提升" — Goal、Success Criteria（4 条）
- `.planning/REQUIREMENTS.md` §TEST-01、TEST-02

### 关键文件（现有测试集中位置）
- `src/pipeline/filters/mod.rs` — 过滤器逻辑 + 现有测试（mod tests 在文件末尾）
- `src/exporter/csv/tests.rs` — CSV exporter 现有测试
- `src/exporter/sqlite/tests.rs` — SQLite exporter 现有测试
- `src/exporter/tests.rs` — ExporterManager 测试
- `src/error.rs` — 错误类型测试（末尾 mod tests）
- `src/cli/run/tests.rs` — run 命令集成测试
- `src/pipeline/normalizer.rs` — normalizer 测试

### 工具
- `cargo-llvm-cov 0.8.5`（已安装）— `cargo llvm-cov --html` 生成报告
- 报告路径：`target/llvm-cov/html/index.html`

</canonical_refs>

<code_context>
## Existing Code Insights

### 已有测试模式
- 各模块使用 `#[cfg(test)] mod tests { ... }` 结构，测试与生产代码同文件
- 测试数据：`tempfile::TempDir` 创建临时目录，`std::fs::write` 写入测试文件
- 集成测试（`tests.rs` 单独文件）：`src/cli/run/tests.rs`、`src/exporter/tests.rs` 等

### 预期低覆盖区域（推测，待报告确认）
- `src/pipeline/filters/mod.rs` — 多字段组合 include/exclude、指标过滤器边界值
- `src/exporter/csv/writer.rs` — `has_metrics` 条件分支（`rowcount != 0`）、字段投影
- `src/error.rs` — `suggestion()` 方法中部分变体的返回路径

### 约束
- 测试不依赖外部服务或网络（成功标准 4）
- `cargo test` 全部通过 + `cargo clippy --all-targets -- -D warnings` 通过

</code_context>

<specifics>
## Specific Ideas

- 生成报告命令：`cargo llvm-cov --html && open target/llvm-cov/html/index.html`（macOS）
- 文本摘要：`cargo llvm-cov --text 2>&1 | grep -E "TOTAL|Uncovered"` 快速识别低覆盖函数
- 计划阶段：先列出预测的低覆盖区域（占位），执行阶段以实际报告为准替换

</specifics>

<deferred>
## Deferred Ideas

- CI 中自动发布覆盖率报告（Codecov/Coveralls 集成）— 超出本阶段范围
- 设置覆盖率门槛（cargo-llvm-cov --fail-under-lines）— 后续里程碑工程化方向
- Property-based testing（proptest/quickcheck）— 超出范围

</deferred>

---

*Phase: 63-test-coverage*
*Context gathered: 2026-06-03*
