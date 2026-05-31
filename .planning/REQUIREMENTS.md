# Requirements: sqllog2db

**Defined:** 2026-05-31
**Core Value:** 用户能够精确指定"导出哪些记录的哪些字段"——过滤逻辑清晰可配置，输出结果完全可控

## v1.12 Requirements

### 错误信息优化

- [ ] **ERROR-01**: 用户看到的错误信息包含具体出错的配置字段名称和原因（不只是错误类型枚举名）
- [ ] **ERROR-02**: 错误信息包含可操作的修复建议（Hint），帮助用户快速解决问题

### 配置文件体验

- [ ] **CONFIG-01**: `init` 命令生成带行内注释的配置模板，每个字段标注用途和合法值示例
- [ ] **CONFIG-02**: `validate` 命令逐项输出每个校验条件的通过/失败状态，而非仅返回最终成功/失败

### 日志级别与运行提示

- [ ] **LOG-01**: 用户可通过 `--verbose` 标志开启详细输出（显示每个处理文件、过滤匹配详情等）
- [ ] **LOG-02**: 用户可通过 `--quiet` 标志抑制进度条和运行摘要，仅显示错误信息
- [ ] **LOG-03**: 运行结束摘要根据 verbose/quiet 模式自动调整输出内容和详细程度

### Glob 输入支持

- [ ] **INPUT-01**: `config.toml` 的 `input` 字段支持 glob 模式（如 `sqllogs/*.log`），自动展开匹配文件列表
- [ ] **INPUT-02**: 命令行 `--input` 参数支持 glob 模式（如 `--input 'logs/*.log'`），与配置文件行为一致

## Out of Scope

| Feature | Reason |
|---------|--------|
| OR 条件组合 | 之前已排除，保持简单过滤模型 |
| 跨字段联合条件 | 之前已排除 |
| 新输出格式（JSON/Parquet） | 保持 CSV/SQLite 双格式 |
| MultiProgress 多级进度条 | 单行进度条已满足需求 |
| 数值错误码系统（E001/E002） | 过度工程化，thiserror Display 足够 |
| 断点续传 | v1.7 已移除，不恢复 |
| 交互式配置向导 | 过度工程化，带注释模板足够 |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| ERROR-01 | 46 | Pending |
| ERROR-02 | 46 | Pending |
| CONFIG-01 | 47 | Pending |
| CONFIG-02 | 47 | Pending |
| LOG-01 | 48 | Pending |
| LOG-02 | 48 | Pending |
| LOG-03 | 48 | Pending |
| INPUT-01 | 49 | Pending |
| INPUT-02 | 49 | Pending |

**Coverage:**
- v1.12 requirements: 9 total
- Mapped to phases: 9
- Unmapped: 0

---
*Requirements defined: 2026-05-31*
*Last updated: 2026-05-31 — v1.12 roadmap created, all 9 requirements mapped to Phases 46–49*
