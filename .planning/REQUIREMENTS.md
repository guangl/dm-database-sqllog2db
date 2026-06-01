# Requirements: sqllog2db v1.13

**Defined:** 2026-06-01
**Core Value:** 用户能够精确指定"导出哪些记录的哪些字段"——过滤逻辑清晰可配置，输出结果完全可控。

## v1.13 Requirements

### Stats 子命令

- [ ] **STATS-01**: 用户可运行 `sqllog2db stats -c config.toml` 获取 SQL 统计报告
- [ ] **STATS-02**: 用户可通过 `--top N` 参数控制每张表展示条数（默认 20）
- [ ] **STATS-03**: 用户可看到慢 SQL TOP-N，按 elapsed 降序排列，包含：SQL 文本、elapsed、时间戳
- [ ] **STATS-04**: 用户可看到高频 SQL TOP-N，按调用次数降序排列，包含：标准化 SQL、调用次数、avg elapsed、max elapsed
- [ ] **STATS-05**: 统计结果输出格式遵循 config.toml 中的 exporter 配置（CSV 或 SQLite）
- [ ] **STATS-06**: SQL 标准化将字面量参数（字符串/数字）替换为占位符 `?`，合并同模板的不同参数调用

## Future Requirements

### 扩展统计维度

- **STATS-07**: 按用户名分组统计各用户 SQL 调用情况
- **STATS-08**: 按表名分组统计各表访问频率
- **STATS-09**: 慢 SQL 阈值告警（超过阈值时输出警告）
- **STATS-10**: 时间窗口过滤（只统计指定时间段内的日志）

## Out of Scope

| Feature | Reason |
|---------|--------|
| 修改现有 `run` 命令输出 | 保持现有用户行为不变 |
| 新增 exporter 实现 | 复用现有 CSV/SQLite exporter，不引入新格式 |
| 实时统计（边 run 边统计） | 复杂度高，stats 作为独立后处理命令更清晰 |
| SQL 执行计划分析 | 超出日志解析范围 |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| STATS-01 | TBD | Pending |
| STATS-02 | TBD | Pending |
| STATS-03 | TBD | Pending |
| STATS-04 | TBD | Pending |
| STATS-05 | TBD | Pending |
| STATS-06 | TBD | Pending |

**Coverage:**
- v1.13 requirements: 6 total
- Mapped to phases: 0 (roadmap pending)
- Unmapped: 6 ⚠️

---
*Requirements defined: 2026-06-01*
*Last updated: 2026-06-01 after initial definition*
