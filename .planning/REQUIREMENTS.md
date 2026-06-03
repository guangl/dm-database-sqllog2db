# Requirements: sqllog2db v1.16 工程质量深化

## Milestone Goal

在 v1.15 的基础上，全面提升代码结构、测试覆盖、文档质量和构建可复现性。不新增用户可见功能，专注工程基础。

---

## v1 Requirements

### 代码结构整理（STRUCT）

- [ ] **STRUCT-01**: cli/run 中超过 40 行的函数（除已拆分的 handle_run 外）识别并进行语义拆分
- [ ] **STRUCT-02**: exporter/pipeline 模块内重复代码消除，模块边界清晰化
- [x] **STRUCT-03**: 错误转换和传播路径统一，删除冗余 unwrap/expect

### CI/CD 配置（CROSS）

- [ ] **CROSS-01**: Cross.toml 中 aarch64-linux cross-rs image 的 edge 浮动标签替换为固定 SHA digest，提升构建可复现性

### 文档完善（DOC）

- [x] **DOC-01**: README.md 更新，添加 v1.13（stats）、v1.14（--from/--to）、v1.15（CI/CD 修复）新功能示例，更新完整功能列表
- [x] **DOC-02**: CHANGELOG.md 新建，采用 Keep a Changelog 格式，补全 v1.0 至 v1.15 全部版本变更记录
- [x] **DOC-03**: config.toml init 模板全字段内联注释确认/补全（v1.12 已完成部分，本次补全遗漏字段）

### 测试覆盖提升（TEST）

- [x] **TEST-01**: 运行 cargo-llvm-cov 或 cargo-tarpaulin 生成当前覆盖率报告，识别覆盖不足区域
- [x] **TEST-02**: 按覆盖率分析结果，补全关键路径测试（过滤器 edge case、exporter 单元测试、错误路径测试等，优先级按覆盖率缺口决定）

---

## Future Requirements

*（本里程碑不涉及新用户功能；如后续需求出现，在此记录）*

---

## Out of Scope

| Feature | Reason |
|---------|--------|
| 新输出格式（JSON/Parquet） | 保持 CSV/SQLite 双格式 |
| OR 条件组合 / 跨字段联合条件 | 保持简单过滤模型 |
| MultiProgress 多级进度条 | 单行进度条已满足需求 |
| Cross.toml 完全固化（换用其他镜像策略）| 仅固定现有 cross-rs 镜像 SHA，不变更镜像来源 |

---

## Traceability

*由 gsd-roadmapper 填写（相位映射）*

| REQ-ID | Phase | Notes |
|--------|-------|-------|
| STRUCT-01 | — | — |
| STRUCT-02 | — | — |
| STRUCT-03 | — | — |
| CROSS-01 | — | — |
| DOC-01 | — | — |
| DOC-02 | — | — |
| DOC-03 | — | — |
| TEST-01 | — | — |
| TEST-02 | — | — |
