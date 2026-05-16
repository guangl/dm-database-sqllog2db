# Roadmap: sqllog2db

## Milestones

- ✅ **v1.0 增强 SQL 内容过滤与字段投影** — Phases 1–2 (shipped 2026-04-18)
- ✅ **v1.1 性能优化** — Phases 3–6 (shipped 2026-05-10)
- ✅ **v1.2 质量强化 & 性能深化** — Phases 7–11 (shipped 2026-05-15)
- ✅ **v1.3 SQL 模板分析 & 可视化** — Phases 12–16 (shipped 2026-05-17)
- 🚧 **v1.4 代码重构 & 质量深化** — Phases 17–20 (in progress)

## Phases

<details>
<summary>✅ v1.0 增强 SQL 内容过滤与字段投影 (Phases 1–2) — SHIPPED 2026-04-18</summary>

- [x] Phase 1: 正则字段过滤 (2/2 plans) — completed 2026-04-18
- [x] Phase 2: 输出字段控制 (4/4 plans) — completed 2026-04-18

Full details: `.planning/milestones/v1.0-ROADMAP.md`

</details>

<details>
<summary>✅ v1.1 性能优化 (Phases 3–6) — SHIPPED 2026-05-10</summary>

- [x] Phase 3: Profiling & Benchmarking (3/3 plans) — completed 2026-04-27
- [x] Phase 4: CSV 性能优化 (4/4 plans) — completed 2026-05-09
- [x] Phase 5: SQLite 性能优化 (3/3 plans) — completed 2026-05-10
- [x] Phase 6: 解析库集成 + 验收 (2/2 plans) — completed 2026-05-10

Full details: `.planning/milestones/v1.1-ROADMAP.md`

</details>

<details>
<summary>✅ v1.2 质量强化 & 性能深化 (Phases 7–11) — SHIPPED 2026-05-15</summary>

- [x] Phase 7: 技术债修复 (1/1 plans) — completed 2026-05-10
- [x] Phase 8: 排除过滤器 (2/2 plans) — completed 2026-05-10
- [x] Phase 9: CLI 启动提速 (5/5 plans) — completed 2026-05-14
- [x] Phase 10: 热路径优化 (3/3 plans) — completed 2026-05-15
- [x] Phase 11: Nyquist 补签 (2/2 plans) — completed 2026-05-15

Full details: `.planning/milestones/v1.2-ROADMAP.md`

</details>

<details>
<summary>✅ v1.3 SQL 模板分析 & 可视化 (Phases 12–16) — SHIPPED 2026-05-17</summary>

- [x] Phase 12: SQL 模板归一化引擎 (3/3 plans) — completed 2026-05-15
- [x] Phase 13: TemplateAggregator 流式统计累积器 (2/2 plans) — completed 2026-05-15
- [x] Phase 14: Exporter 集成输出 (4/4 plans) — completed 2026-05-16
- [x] Phase 15: SVG 图表基础设施 + 前两类图表 (5/5 plans) — completed 2026-05-17
- [x] Phase 16: 剩余图表 (5/5 plans) — completed 2026-05-17

Full details: `.planning/milestones/v1.3-ROADMAP.md`

</details>

### 🚧 v1.4 代码重构 & 质量深化 (In Progress)

**Milestone Goal:** 系统性重构配置模型与代码结构，补全测试覆盖，大幅提升项目可维护性

- [ ] **Phase 17: 过滤器配置嵌套化** - 将 [filter] 扁平字段重组为 [filter.include] / [filter.exclude] 子表，serde alias 向后兼容
- [ ] **Phase 18: 模板 & 图表配置嵌套化** - 将模板与图表配置集中至 [template] / [charts] 子表
- [ ] **Phase 19: 代码结构重构** - 拆分超大文件，消除重复代码，收紧可见性，统一 Exporter trait
- [ ] **Phase 20: 测试覆盖深化** - 补全 VERIFICATION.md，添加端到端集成测试、边界测试、proptest 属性测试

## Phase Details

### Phase 17: 过滤器配置嵌套化
**Goal**: 用户可用 [filter.include] / [filter.exclude] 嵌套子表配置过滤条件，旧版扁平格式仍可正确解析
**Depends on**: Phase 16
**Requirements**: CONFIG-01, CONFIG-02, CONFIG-05
**Success Criteria** (what must be TRUE):
  1. 新格式 config 文件使用 [filter.include] / [filter.exclude] 子表可正常运行，过滤结果与旧格式一致
  2. 旧版扁平字段配置文件（include_users / exclude_users 等）无需修改即可被正确解析，行为不变
  3. `cargo run -- validate -c config.toml` 对新旧两种格式均通过验证，无报错
  4. `pipeline.is_empty()` 热路径快速退出逻辑在新配置结构下保持不变（clippy + 测试全通过）
**Plans**: TBD

### Phase 18: 模板 & 图表配置嵌套化
**Goal**: 用户可在 [template] 和 [charts] 子表中集中管理模板分析与图表生成配置
**Depends on**: Phase 17
**Requirements**: CONFIG-03, CONFIG-04
**Success Criteria** (what must be TRUE):
  1. 新格式 config 使用 [template] 子表（enable_template_normalization / enable_template_aggregation / output_*）可正常运行
  2. 新格式 config 使用 [charts] 子表（output_dir / top_n）可正常生成 SVG 图表
  3. `cargo run -- init -o config.toml --force` 生成的默认配置文件使用新嵌套格式
  4. cargo clippy --all-targets -- -D warnings 零警告，cargo test 全通过
**Plans**: TBD

### Phase 19: 代码结构重构
**Goal**: 源代码文件按职责合理拆分，重复逻辑消除，可见性收紧，Exporter trait 统一
**Depends on**: Phase 18
**Requirements**: REFACTOR-01, REFACTOR-02, REFACTOR-03, REFACTOR-04
**Success Criteria** (what must be TRUE):
  1. 原超过 300 行的源文件（filters.rs / config.rs / run.rs 等）已按职责拆分，各子模块行数合理
  2. CsvExporter 与 SqliteExporter 的字段投影逻辑已合并至共用辅助函数，无 copy-paste 重复片段
  3. Exporter trait 涵盖 write_record / finalize 等核心方法，不必要的特化分支已消除
  4. pub 可见性已收紧为 pub(crate) / pub(super)，跨层漏出的实现细节减少
  5. cargo clippy --all-targets -- -D warnings 零警告，cargo test 全通过，性能基准无回归
**Plans**: TBD

### Phase 20: 测试覆盖深化
**Goal**: 补全历史遗留的 VERIFICATION.md，新增端到端集成测试、边界条件测试和属性测试
**Depends on**: Phase 19
**Requirements**: TEST-01, TEST-02, TEST-03, TEST-04
**Success Criteria** (what must be TRUE):
  1. Phase 12 / 13 / 14 / 16 各有完整 VERIFICATION.md，覆盖 UAT 标准与成功标准
  2. 至少一条端到端集成测试：读取 fixture .log 文件 → 运行完整 pipeline → 验证 CSV 或 SQLite 输出内容正确
  3. 边界条件测试覆盖：空 log 文件、全部记录被过滤输出为空、格式错误行被跳过并计入 error log、超长 SQL 字段
  4. normalize_template 有 proptest 属性测试，验证幂等性（归一化两次 = 归一化一次）和字面量保护不变性
  5. cargo test 全通过（含新增测试），cargo clippy --all-targets -- -D warnings 零警告
**Plans**: TBD

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. 正则字段过滤 | v1.0 | 2/2 | Complete | 2026-04-18 |
| 2. 输出字段控制 | v1.0 | 4/4 | Complete | 2026-04-18 |
| 3. Profiling & Benchmarking | v1.1 | 3/3 | Complete | 2026-04-27 |
| 4. CSV 性能优化 | v1.1 | 4/4 | Complete | 2026-05-09 |
| 5. SQLite 性能优化 | v1.1 | 3/3 | Complete | 2026-05-10 |
| 6. 解析库集成 + 验收 | v1.1 | 2/2 | Complete | 2026-05-10 |
| 7. 技术债修复 | v1.2 | 1/1 | Complete | 2026-05-10 |
| 8. 排除过滤器 | v1.2 | 2/2 | Complete | 2026-05-10 |
| 9. CLI 启动提速 | v1.2 | 5/5 | Complete | 2026-05-14 |
| 10. 热路径优化 | v1.2 | 3/3 | Complete | 2026-05-15 |
| 11. Nyquist 补签 | v1.2 | 2/2 | Complete | 2026-05-15 |
| 12. SQL 模板归一化引擎 | v1.3 | 3/3 | Complete | 2026-05-15 |
| 13. TemplateAggregator 流式统计累积器 | v1.3 | 2/2 | Complete | 2026-05-15 |
| 14. Exporter 集成输出 | v1.3 | 4/4 | Complete | 2026-05-16 |
| 15. SVG 图表基础设施 + 前两类图表 | v1.3 | 5/5 | Complete | 2026-05-17 |
| 16. 剩余图表 | v1.3 | 5/5 | Complete | 2026-05-17 |
| 17. 过滤器配置嵌套化 | v1.4 | 0/TBD | Not started | - |
| 18. 模板 & 图表配置嵌套化 | v1.4 | 0/TBD | Not started | - |
| 19. 代码结构重构 | v1.4 | 0/TBD | Not started | - |
| 20. 测试覆盖深化 | v1.4 | 0/TBD | Not started | - |
