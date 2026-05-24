# Requirements: sqllog2db

**Defined:** 2026-05-24
**Core Value:** 用户能够精确指定"导出哪些记录的哪些字段"——过滤逻辑清晰可配置，输出结果完全可控

## v1.11 Requirements

### Parser 适配

- [ ] **PARSER-01**: 用户使用最新版 `dm-database-parser-sqllog`，`cargo build --release` 编译成功，无 deprecated 警告
- [ ] **PARSER-02**: 利用新 API（如 `from_reader` 或新字段）替换现有变通写法，删除冗余的手动映射代码

### 性能优化

- [ ] **PERF-01**: 解析热路径优化后，criterion benchmark 显示单线程吞吐量提升（相对 v1.10 基线 1.55M records/sec 有可量化改善）
- [ ] **PERF-02**: 处理 1GB+ 日志文件时，Heaptrack/Valgrind 或 jemalloc 统计显示峰值堆分配明显减少
- [ ] **PERF-03**: 并行处理范围扩展——SQLite 导出支持批量并行写入，或多输入文件支持跨文件并行解析

### 代码重构

- [ ] **REFACTOR-01**: filter 模块重构后，pre-scan 与 main-pass 逻辑边界清晰，单元测试覆盖率不低于重构前，代码行数减少或复杂度降低（可 diff 验证）
- [ ] **REFACTOR-02**: Cargo.toml 所有依赖升级到最新兼容版本，`cargo update` 后 `cargo test` 全部通过

### 基准测试

- [ ] **BENCH-01**: criterion 基准覆盖 CSV 导出、SQLite 导出、filter 路径（含启用/禁用两种模式）、parser 原始解析速度四个场景，`cargo bench` 可独立运行
- [ ] **BENCH-02**: GitHub Actions CI 集成 benchmark，每次 PR 导出基准报告（HTML 或 JSON），可对比历史基线

## Out of Scope

| Feature | Reason |
|---------|--------|
| OR 条件组合 | 之前已排除，保持简单过滤模型 |
| 跨字段联合条件 | 之前已排除 |
| 新输出格式（JSON/Parquet） | 保持 CSV/SQLite 双格式 |
| MultiProgress 多级进度条 | 单行进度条已满足需求 |
| 数值错误码系统（E001/E002） | 过度工程化，thiserror Display 足够 |
| 断点续传 | v1.7 已移除，不恢复 |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| PARSER-01 | Phase 41 | Pending |
| PARSER-02 | Phase 43 | Pending |
| PERF-01 | Phase 44 | Pending |
| PERF-02 | Phase 44 | Pending |
| PERF-03 | Phase 45 | Pending |
| REFACTOR-01 | Phase 43 | Pending |
| REFACTOR-02 | Phase 41 | Pending |
| BENCH-01 | Phase 42 | Pending |
| BENCH-02 | Phase 45 | Pending |

**Coverage:**
- v1.11 requirements: 9 total
- Mapped to phases: 9 (100%)
- Unmapped: 0

---
*Requirements defined: 2026-05-24*
*Last updated: 2026-05-24 — v1.11 roadmap created, traceability filled*
