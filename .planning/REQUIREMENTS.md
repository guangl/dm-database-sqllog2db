# Requirements: sqllog2db

**Defined:** 2026-06-01
**Core Value:** 用户能够精确指定"导出哪些记录的哪些字段"——过滤逻辑清晰可配置，输出结果完全可控。

## v1.14 Requirements

### stats 时间段过滤

- [ ] **STATS-07**: 用户可通过 `--from`/`--to` CLI 参数为 `stats` 命令指定时间段（如 `--from "2024-01-01"` `--to "2024-01-31"`）
- [ ] **STATS-08**: 用户可在 config.toml `[stats]` 节配置 `from`/`to` 字段作为 `stats` 命令的默认时间段
- [ ] **STATS-09**: CLI 参数 `--from`/`--to` 优先于 config.toml 中的 `from`/`to` 值，两者均缺省时不做时间过滤
- [ ] **STATS-10**: `stats` 聚合时自动跳过 `ts` 字段不在指定时间段内的记录（字符串前缀比较，`ts >= from` 且 `ts <= to`）
- [ ] **STATS-11**: 支持 `"YYYY-MM-DD"` 和 `"YYYY-MM-DD HH:MM:SS"` 两种时间格式，格式不合法时给出明确错误提示

## v1.13 Requirements（已完成）

### Stats 子命令

- [x] **STATS-01**: 用户可运行 `sqllog2db stats -c config.toml` 获取 SQL 统计报告
- [x] **STATS-02**: 用户可通过 `--top N` 参数控制每张表展示条数（默认 20）
- [x] **STATS-03**: 用户可看到慢 SQL TOP-N，按 elapsed 降序排列，包含：SQL 文本、elapsed、时间戳
- [x] **STATS-04**: 用户可看到高频 SQL TOP-N，按调用次数降序排列，包含：标准化 SQL、调用次数、avg elapsed、max elapsed
- [x] **STATS-05**: 统计结果输出格式遵循 config.toml 中的 exporter 配置（CSV 或 SQLite）
- [x] **STATS-06**: SQL 标准化将字面量参数（字符串/数字）替换为占位符 `?`，合并同模板的不同参数调用

## Future Requirements

### 扩展统计维度

- **STATS-12**: 按用户名分组统计各用户 SQL 调用情况
- **STATS-13**: 按表名分组统计各表访问频率
- **STATS-14**: 慢 SQL 阈值告警（超过阈值时输出警告）

## Out of Scope

| Feature | Reason |
|---------|--------|
| `run` 命令新增 `--from`/`--to` | 已有 `[pipeline.include]` `start_ts`/`end_ts`，路径已覆盖 |
| 相对时间（如 `--from "1d ago"`） | 增加复杂度，绝对时间格式已满足需求 |
| 时区处理 | 达梦日志 `ts` 无时区信息，本地时间字符串比较足够 |
| 修改现有 `run` 命令输出 | 保持现有用户行为不变 |
| 新增 exporter 实现 | 复用现有 CSV/SQLite exporter，不引入新格式 |
| 实时统计（边 run 边统计） | 复杂度高，stats 作为独立后处理命令更清晰 |
| SQL 执行计划分析 | 超出日志解析范围 |

## Traceability

### v1.14

| Requirement | Phase | Status |
|-------------|-------|--------|
| STATS-07 | Phase 53 | Pending |
| STATS-08 | Phase 53 | Pending |
| STATS-09 | Phase 53 | Pending |
| STATS-10 | Phase 54 | Pending |
| STATS-11 | Phase 53 | Pending |

**v1.14 Coverage:**
- v1.14 requirements: 5 total
- Mapped to phases: 5
- Unmapped: 0 ✓

### v1.13

| Requirement | Phase | Status |
|-------------|-------|--------|
| STATS-01 | Phase 51 | Complete |
| STATS-02 | Phase 51 | Complete |
| STATS-03 | Phase 52 | Complete |
| STATS-04 | Phase 52 | Complete |
| STATS-05 | Phase 52 | Complete |
| STATS-06 | Phase 50 | Complete |

**v1.13 Coverage:** 6/6 (100%)

---
*Requirements defined: 2026-06-01*
*Last updated: 2026-06-01 — v1.14 traceability populated (Phase 53: STATS-07/08/09/11, Phase 54: STATS-10)*
