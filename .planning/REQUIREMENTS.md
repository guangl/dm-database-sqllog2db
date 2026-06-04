# Requirements: sqllog2db v1.17

**Milestone:** v1.17 多文件并行提速  
**Created:** 2026-06-04  
**Status:** Active

## v1.17 Requirements

### 并行处理 (PARALLEL)

- [x] **PARALLEL-01**: 当输入包含多个文件且输出为 CSV 时，自动使用多文件并行解析路径（无需修改 config.toml）
<!-- 原始设计意图为 channel 写入线程；D-01 决策改为 temp-file 方案，channel 留后续里程碑 -->
- [x] **PARALLEL-02**: 并行路径写入不全量缓冲内存：每个 rayon 线程将单文件记录收集到 Vec 后写入临时 CSV，写入完成后 Vec 立即释放；最终按原始顺序拼接为单一输出文件（temp-file 方案，per D-01）
- [ ] **PARALLEL-03**: 并行路径的 CSV 字段格式与单线程路径完全一致（字段类型、转义规则、has_metrics 条件）
- [ ] **PARALLEL-04**: 过滤管道（include/exclude/sql/indicators filters）在并行路径下产生与单线程路径等价的过滤结果
- [ ] **PARALLEL-05**: `--verbose` 逐文件输出、`--quiet` 完全抑制、处理摘要（行数/错误数）在并行路径下正常显示

### I/O 优化 (IO)

- [x] **IO-01**: 读取 .log 文件由 dm-database-parser-sqllog 通过 fs::read() 一次性全量读取（单次 syscall），效果优于扩大 BufReader 缓冲区（D-01，Phase 65）

### 兼容性保障 (COMPAT)

- [ ] **COMPAT-01**: 现有 740+ 测试（lib/integration/benchmark）全部通过，无行为回归
- [ ] **COMPAT-02**: 并行路径新增至少 2 条集成测试：多文件 CSV 内容一致性断言（对比单线程路径结果）
- [ ] **COMPAT-03**: 不修改现有 config.toml 格式或 init 模板

## Future Requirements

- CSV 并行路径进度条多行显示（MultiProgress）— 当前单行进度已满足，多行进度条过度工程
- 每文件独立输出 CSV（sharding）— 超出当前单一输出文件设计
- SQLite 并行路径 I/O 优化 — 留待后续里程碑

## Out of Scope

| Feature | Reason |
|---------|--------|
| SQLite 并行路径修改 | 本里程碑仅聚焦 CSV，SQLite 已有并行路径 |
| 跨文件全局行顺序保证 | 并行路径文件间行顺序不确定，可接受 |
| mmap 替代 BufReader | BufReader 缓冲区调优已足够，mmap 增加代码复杂度 |
| OR 条件组合 | 已在 Out of Scope 列表，保持简单过滤模型 |

## Traceability

| REQ-ID | Phase |
|--------|-------|
| PARALLEL-01 | 64 |
| PARALLEL-02 | 64 |
| PARALLEL-03 | 65 |
| PARALLEL-04 | 65 |
| PARALLEL-05 | 65 |
| IO-01 | 65 |
| COMPAT-01 | 66 |
| COMPAT-02 | 66 |
| COMPAT-03 | 66 |

*Coverage: 9/9 requirements mapped — 100%*
