# Requirements: sqllog2db

**Defined:** 2026-06-07
**Core Value:** 用户能够精确指定"导出哪些记录的哪些字段"——过滤逻辑清晰可配置，输出结果完全可控

## v1.20 Requirements

### 性能测量（BENCH）

- [ ] **BENCH-01**: 开发者可以用 hyperfine 测量 CLI 冷启动延迟，结果存入 BENCHMARKS.md
- [ ] **BENCH-02**: 开发者可以用 `--save-baseline` 将 criterion 结果存档到 `benches/baselines/`，版本间对比有迹可循

### SQLite 导出提速（SQLITE）

- [ ] **SQLITE-01**: SQLite 导出支持 multi-row batch INSERT（缓冲 N 条记录，一次执行 `INSERT INTO t VALUES (…),(…),…`，减少逐行调用开销）
- [ ] **SQLITE-02**: benchmark 可以量化 multi-row INSERT 相较于当前单行模式的吞吐量提升

### 内存与分配优化（MEM）

- [ ] **MEM-01**: normalizer 热路径 HashMap key 不再每条记录重复 clone String（改用 Arc 或调整生命周期，减少堆分配）
- [ ] **MEM-02**: CSV line_buf 初始容量按典型 SQL 长度预热，减少 Vec grow 次数

### 代码结构重构（STRUCT）

- [ ] **STRUCT-04**: `cli/run/parallel.rs` 与 `cli/run/sqlite_parallel.rs` 的公共逻辑（文件收集、记录处理、错误统计）提取为共享模块，消除重复代码

### 异步解析路径（ASYNC）

- [ ] **ASYNC-01**: 将解析路径从同步 API 切换为 `dm-database-parser-sqllog` 的 async API，解析主循环使用 `.await`（crate 已原生支持 async，添加 tokio 运行时并迁移调用点）

## Future Requirements

### 深度 profiling

- **PROF-01**: flamegraph/perf 热图分析真实日志文件的 CPU 占用分布
- **PROF-02**: heaptrack/massif 峰值内存 profiling（大文件场景）

## Out of Scope

| Feature | Reason |
|---------|--------|
| 新增输出格式（JSON/Parquet） | 保持 CSV/SQLite 双格式 |
| dm-database-parser-sqllog 新 API 开发 | crate 已支持 async，仅需切换调用方 |
| SQLite WAL 模式 | 已使用 JOURNAL_MODE=OFF（更快），WAL 不适用 |
| SQLite 并行写入 | 单线程导出架构约束，并行写入需要事务重设计 |
| MultiProgress 多级进度 | 单行进度条已满足需求 |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| BENCH-01 | Phase 72 | Pending |
| BENCH-02 | Phase 72 | Pending |
| SQLITE-01 | Phase 73 | Pending |
| SQLITE-02 | Phase 73 | Pending |
| MEM-01 | Phase 74 | Pending |
| MEM-02 | Phase 74 | Pending |
| STRUCT-04 | Phase 75 | Pending |
| ASYNC-01 | Phase 76 | Pending |

**Coverage:**
- v1.20 requirements: 8 total
- Mapped to phases: 8 ✓
- Unmapped: 0 ✓

---
*Requirements defined: 2026-06-07*
*Last updated: 2026-06-08 — roadmap Phase 72–76 assigned (8/8 requirements mapped)*
