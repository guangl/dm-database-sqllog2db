# Phase 20: 测试覆盖深化 - Context

**Gathered:** 2026-05-18
**Status:** Ready for planning

<domain>
## Phase Boundary

补全所有历史遗留的 VERIFICATION.md（Phase 12/13/14/15/16/17/18），新增端到端集成测试（带过滤器流水线 + 模板分析 + 字段投影）、边界条件测试（空文件/全过滤/格式错误/超长字段），以及 normalize_template 的 proptest 属性测试（幂等性 + 字面量保护不变性）。

**本 Phase 不改变任何生产代码逻辑**——只补充文档和测试。

</domain>

<decisions>
## Implementation Decisions

### VERIFICATION.md 补写范围（TEST-01 扩展）

- **D-01:** 补写范围扩展为 **Phase 12/13/14/15/16/17/18** 全部七个阶段（TEST-01 原文只说 12/13/14/16，但 Phase 15/17/18 同样缺失，一并补全）

- **D-02:** 写在各阶段**原目录**中：
  - Phase 12-16 → `.planning/milestones/v1.3-phases/{nn}-*/`（若目录不存在则新建）
  - Phase 17-18 → `.planning/phases/17-filter-nesting/` 和 `.planning/phases/18-template-chart-nesting/`

- **D-03:** 内容采用**标准格式**：UAT 标准 + 成功标准逐条验证 + 实际验证方法（参照 Phase 19 VERIFICATION.md 格式）。每条 Success Criteria 对应一个具体的验证步骤（`cargo test` / `cargo run` / 文件目测）

### 端到端集成测试（TEST-02）

- **D-04:** 使用**程序生成 log**（延续现有 `write_test_log()` 模式），不建立 `tests/fixtures/` 目录

- **D-05:** 验证 **CSV 输出**格式（最简单，SQLite 已有其他覆盖）

- **D-06:** 新增三条端到端测试，各自验证一个功能路径：
  1. **带过滤器的完整流水线**：配置 include/exclude 过滤器 → 验证输出 CSV 中仅保留通过过滤的记录（具体字段值正确）
  2. **模板分析流水线**：启用 `enable_template_normalization = true` → 验证输出 CSV 中 `template_key` 列存在且非空
  3. **字段投影**：配置 `ordered_fields` 指定子集顺序 → 验证输出 CSV header 和数据列顺序正确

### 边界条件测试（TEST-03）

- **D-07:** 新增四个边界条件测试（在 `tests/integration.rs` 或相关源文件中）：
  1. **空 log 文件**：目录下只有一个 0 字节 .log 文件 → `handle_run` 不 panic，输出 CSV 只有 header
  2. **全部记录被过滤为空**：过滤条件导致所有记录被排除 → 输出 CSV 只有 header，ExportStats.total() == 0
  3. **格式错误行被跳过**：log 文件中混入无法解析的行 → 不 panic，错误计入 error log，正常行正常导出
  4. **超长 SQL 字段**：构造一条 sql_text 超过 1MB 的记录 → 不 panic，正常写入 CSV（无截断或错误）

- **D-08:** 边界测试放在 `tests/integration.rs`，与现有 integration tests 保持一致

### proptest 属性测试（TEST-04）

- **D-09:** Generator 策略：使用 proptest 默认的 **任意 ASCII 字符串**（`any::<String>()`）——覆盖最广，若 panic 即暴露 bug

- **D-10:** 覆盖目标：**仅 normalize_template**（按 TEST-04 范围），不额外测 fingerprint()

- **D-11:** 测试放在 **`src/pipeline/fingerprint.rs` 的 `#[cfg(test)] mod tests`** 中（与现有 normalize_template 单元测试放在一起）

- **D-12:** 两条属性测试：
  1. **幂等性**：`normalize_template(normalize_template(s)) == normalize_template(s)`
  2. **字面量保护不变性**：`normalize_template(s)` 中不应出现以 `'` 开头的内容外部的注释符号（即字符串字面量内的 `--` 不被消除）——具体策略由 Claude 根据实现选择最合适的不变量表达

### Claude's Discretion

- TEST-03 各边界 case 的具体测试函数命名、arrange/act/assert 结构——按现有 integration test 风格
- VERIFICATION.md 中各阶段实际运行命令的具体写法（`cargo test` filter 参数等）
- proptest `#[proptest]` 宏的参数（cases 数量等）——使用 proptest 默认即可

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求规范

- `.planning/REQUIREMENTS.md` — TEST-01 / TEST-02 / TEST-03 / TEST-04（Phase 20 的四条需求）
- `.planning/ROADMAP.md` §Phase 20 — Success Criteria（5 条验收标准）

### 待补写 VERIFICATION.md 的阶段目标

- `.planning/milestones/v1.3-ROADMAP.md` §Phase 12/13/14/15/16 — 各阶段 Goal + Success Criteria（补写 VERIFICATION.md 的依据）
- `.planning/ROADMAP.md` §Phase 17/18 — Phase 17/18 的 Goal + Success Criteria

### 测试文件（新增测试的写入位置）

- `tests/integration.rs` — 现有 1293 行集成测试，端到端测试 + 边界测试写入此文件
- `src/pipeline/fingerprint.rs` — proptest 属性测试写入此文件的 `#[cfg(test)] mod tests`

### 关键实现文件（了解接口用于测试）

- `src/cli/run/mod.rs` — `handle_run` 函数签名（端到端测试的入口）
- `src/config/mod.rs` — Config / FiltersFeature / TemplateConfig 结构（构造测试配置）
- `src/pipeline/fingerprint.rs:40` — `pub fn normalize_template(sql: &str) -> String`（proptest 测试对象）

### 现有测试模式参考

- `tests/integration.rs` 中的 `write_test_log()` helper — 程序生成 log 的标准模式
- `tests/integration.rs` 中的 `make_run_config()` — 构造 Config 的工厂函数模式
- `.planning/phases/19-code-refactor/19-VERIFICATION.md` — VERIFICATION.md 标准格式参考

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- `write_test_log(path, count)` (`tests/integration.rs`) — 程序生成 .log 文件的 helper，所有新集成测试直接复用
- `make_run_config(log_dir, csv_file)` (`tests/integration.rs`) — 基础 Config 工厂，新测试在此基础上叠加过滤器/模板/字段投影配置
- `normalize_template(sql)` (`src/pipeline/fingerprint.rs:40`) — proptest 直接测试此函数

### Established Patterns

- **Integration test 模式**：`tempfile::TempDir` + `write_test_log()` + `handle_run()` + 读取输出文件内容验证 — 直接沿用
- **`#[cfg(test)] mod tests` 组织**：单元测试与源码同文件，section divider 注释分组 — proptest 测试遵循同一模式
- **无 mock**：所有测试通过 tempdir + 真实 I/O，不引入 mock crate
- **`assert!(result.is_err())` 风格**：错误路径不用 `#[should_panic]`

### Integration Points

- 端到端测试通过 `handle_run(&cfg, None, false, true, &interrupted, 80, false, None, 1)` 调用完整流水线
- 过滤器配置通过 `FiltersFeature { include: IncludeFilters { users: vec![...], .. }, .. }` 构造（Phase 17/19 重构后的新 API）
- 模板归一化通过 `TemplateConfig { enable: true, .. }` 启用

</code_context>

<specifics>
## Specific Ideas

- **proptest 幂等性测试写法示例**：
  ```rust
  use proptest::prelude::*;

  proptest! {
      #[test]
      fn prop_normalize_template_is_idempotent(s in ".*") {
          let once = normalize_template(&s);
          let twice = normalize_template(&once);
          prop_assert_eq!(once, twice);
      }
  }
  ```

- **端到端过滤器测试思路**：构造 10 条 log 记录，其中 3 条 user="TARGETUSER"，配置 `include.users = ["TARGETUSER"]`，断言输出 CSV 中恰好 3 行数据

- **空 log 文件边界测试思路**：`std::fs::write(log_dir.join("empty.log"), b"")` 后调用 `handle_run`，断言输出 CSV `lines().count() == 1`（只有 header）

</specifics>

<deferred>
## Deferred Ideas

- fingerprint() 的属性测试（输出不含数字字面量等不变量）— 超出 TEST-04 范围，可在 v1.5 测试强化阶段考虑
- SQLite 输出的端到端验证 — 超出 TEST-02 当前范围
- `cargo llvm-cov` 覆盖率门控 — 可在 v1.5 引入

</deferred>

---

*Phase: 20-测试覆盖深化*
*Context gathered: 2026-05-18*
