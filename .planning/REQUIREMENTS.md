# Requirements: sqllog2db v1.4

**Defined:** 2026-05-17
**Core Value:** 用户能够精确指定"导出哪些记录的哪些字段"——过滤逻辑清晰可配置，输出结果完全可控

## v1.4 Requirements

### 配置重构（CONFIG）

- [x] **CONFIG-01**: 用户可在 `[filter.include]` 嵌套子表中配置所有包含过滤条件（users / ips / sessions / threads / statements / apps / tags），替代现有扁平字段
- [x] **CONFIG-02**: 用户可在 `[filter.exclude]` 嵌套子表中配置所有排除过滤条件（同上字段），替代现有扁平字段
- [x] **CONFIG-03**: 用户可在 `[template]` 子表中集中管理所有模板分析配置（enable_template_normalization / enable_template_aggregation / output_*）
- [x] **CONFIG-04**: 用户可在 `[charts]` 子表中集中管理所有图表配置（output_dir / top_n 等）
- [x] **CONFIG-05**: 旧版扁平格式配置文件仍可被正确解析（serde alias 向后兼容，不破坏现有用户配置）

### 代码重构（REFACTOR）

- [x] **REFACTOR-01**: 超过 300 行的源文件按职责拆分为独立子模块（目标：filters.rs / config.rs / run.rs 等）
- [ ] **REFACTOR-02**: CsvExporter 与 SqliteExporter 中重复的字段投影逻辑抽取到共用辅助函数，消除 copy-paste 片段
- [ ] **REFACTOR-03**: Exporter trait 接口统一，涵盖 write_record / finalize 等核心方法，消除不必要的特化分支
- [x] **REFACTOR-04**: 内部类型可见性收紧（pub → pub(crate) / pub(super)），减少跨层漏出的实现细节

### 测试覆盖（TEST）

- [ ] **TEST-01**: Phase 12 / 13 / 14 / 16 各补全 VERIFICATION.md，覆盖 UAT 标准与成功标准
- [ ] **TEST-02**: 至少一条端到端集成测试：读取 fixture .log 文件 → 运行完整 pipeline → 验证 CSV 或 SQLite 输出内容正确
- [ ] **TEST-03**: 边界条件覆盖：空 log 文件、全部记录被过滤输出为空、格式错误行被跳过并计入 error log、超长 SQL 字段
- [ ] **TEST-04**: normalize_template 有 proptest 属性测试，验证幂等性（归一化两次 = 归一化一次）和字面量保护不变性

## Future Requirements

### 延后功能

- **TMPL-03**: 模板统计结果输出为独立 JSON 报告文件（DBA 可读，config 指定路径）— 延后至 v1.5+
- **TMPL-03b**: 模板统计结果输出为独立 CSV 摘要文件（DBA 可用 Excel 打开）— 延后至 v1.5+
- **FILTER-04**: OR 条件组合过滤 — 延后，需求不明确
- **FILTER-05**: 跨字段联合条件谓词 — 延后，复杂度高

## Out of Scope

| Feature | Reason |
|---------|--------|
| 运行时热重载配置 | 配置在启动时加载，不支持动态修改 |
| 配置自动迁移 CLI | 用户量小，文档说明足够；自动迁移引入额外测试负担 |
| JSON / Parquet 导出格式 | 超出当前里程碑范围 |
| SQLite WAL 模式 | v1.2 用户决策移除，理由不变 |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CONFIG-01 | Phase 17 | Complete |
| CONFIG-02 | Phase 17 | Complete |
| CONFIG-05 | Phase 17 | Complete |
| CONFIG-03 | Phase 18 | Complete |
| CONFIG-04 | Phase 18 | Complete |
| REFACTOR-01 | Phase 19 | Complete |
| REFACTOR-02 | Phase 19 | Pending |
| REFACTOR-03 | Phase 19 | Pending |
| REFACTOR-04 | Phase 19 | Complete |
| TEST-01 | Phase 20 | Pending |
| TEST-02 | Phase 20 | Pending |
| TEST-03 | Phase 20 | Pending |
| TEST-04 | Phase 20 | Pending |

**Coverage:**
- v1.4 requirements: 13 total
- Mapped to phases: 13 (100%)
- Unmapped: 0

---
*Requirements defined: 2026-05-17*
*Last updated: 2026-05-17 — Phase 18 Plan 01 complete; CONFIG-03/04 marked complete*
