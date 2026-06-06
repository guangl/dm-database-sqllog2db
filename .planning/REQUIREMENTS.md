# Requirements: sqllog2db

**Defined:** 2026-06-05
**Core Value:** 用户能够精确指定"导出哪些记录的哪些字段"——过滤逻辑清晰可配置，输出结果完全可控。

## v1.18 Requirements

### Watch 模式

- [x] **WATCH-01**: 用户可通过 `sqllog2db watch -c config.toml` 启动目录监听，程序持续运行直到 Ctrl+C
- [x] **WATCH-02**: 监听目录内新增 `.log` 文件时自动触发处理（代码完整，macOS FSEvents e2e 测试标 #[ignore]）
- [x] **WATCH-03**: 已有文件追加内容（文件变大）时触发增量处理
- [x] **WATCH-04**: SQLite 导出模式下仅插入新行（按字节偏移记录进度，避免重复）
- [x] **WATCH-05**: 实时显示当前监听路径、上次触发时间、累计已处理行数
- [x] **WATCH-06**: Ctrl+C 优雅退出，打印最终摘要

### 交互式配置向导

- [x] **INIT-01**: 用户可通过 `sqllog2db init --interactive` 启动对话式向导
- [x] **INIT-02**: 向导逐字段引导（输入路径、导出格式、输出路径），每步给出示例和默认值
- [x] **INIT-03**: 向导生成的 config.toml 格式与非交互式 `init` 完全一致（含注释）

### 运行时异常诊断

- [x] **DIAG-01**: 错误日志每条 parse error 包含行号和原始内容前 120 字符
- [x] **DIAG-02**: 导出摘要按错误类型分组统计（field_missing / parse_failed / encoding_error）
- [x] **DIAG-03**: 常见错误模式触发具体 hint（如"多行编码错误：建议检查文件编码"）

### 进度/摘要增强

- [x] **PROG-01**: 多文件运行时进度条显示 `[当前/总数]` 文件计数器
- [x] **PROG-02**: 进度条显示实时 records/sec 和预计剩余时间（ETA）
- [x] **PROG-03**: 导出摘要新增过滤率（filtered_out/total_read）和错误类型分布

## Future Requirements

### watch 扩展

- **WATCH-F01**: watch 模式支持 CSV 格式（全量重写或追加模式）
- **WATCH-F02**: watch 模式支持多目录监听

### 诊断扩展

- **DIAG-F01**: 生成 HTML 错误报告，包含错误分布可视化
- **DIAG-F02**: 错误行自动建议修复（如字段顺序调整）

## Out of Scope

| Feature | Reason |
|---------|--------|
| watch + CSV 增量插入 | CSV 不支持原位增量写，全量重写语义复杂，延后 |
| OR 条件组合 | 之前已排除，保持简单过滤模型 |
| 新输出格式（JSON/Parquet） | 保持 CSV/SQLite 双格式 |
| MultiProgress 多级进度条 | 单行进度条已满足需求 |
| 数值错误码系统 | thiserror Display 足够 |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| WATCH-01 | Phase 69 | Complete |
| WATCH-02 | Phase 69 | Complete (e2e #[ignore] macOS 限制) |
| WATCH-03 | Phase 70 | Complete |
| WATCH-04 | Phase 70 | Complete |
| WATCH-05 | Phase 69 | Complete |
| WATCH-06 | Phase 69 | Complete |
| INIT-01 | Phase 68 | Complete |
| INIT-02 | Phase 68 | Complete |
| INIT-03 | Phase 68 | Complete |
| DIAG-01 | Phase 67 | Complete |
| DIAG-02 | Phase 67 | Complete |
| DIAG-03 | Phase 67 | Complete |
| PROG-01 | Phase 67 | Complete |
| PROG-02 | Phase 67 | Complete |
| PROG-03 | Phase 67 | Complete |

**Coverage:**
- v1.18 requirements: 15 total
- Mapped to phases: 15 (Phases 67–70)
- Unmapped: 0 ✓

---
*Requirements defined: 2026-06-05*
*Last updated: 2026-06-06 — all requirements marked complete per v1.18 audit*
