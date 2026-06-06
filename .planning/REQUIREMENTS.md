# Requirements: sqllog2db

**Defined:** 2026-06-06
**Milestone:** v1.19 watch完善与文档对齐
**Core Value:** 用户能够精确指定"导出哪些记录的哪些字段"——过滤逻辑清晰可配置，输出结果完全可控。

## v1.19 Requirements

### watch 增强

- [ ] **WATCH-07**: 用户可通过 watch 子命令使用 CSV 导出格式（增量追加到 CSV 文件）
- [ ] **WATCH-08**: watch 长时间运行时 error log 以追加模式写入，不覆盖历史错误
- [ ] **WATCH-09**: watch 收到 Ctrl+C 时退出码为 130（与 `run` 保持一致）

### 工程质量

- [ ] **QUAL-01**: Phase 67/68/69 VALIDATION.md 草稿补全为正式文件，Phase 70 VALIDATION.md 新建
- [ ] **QUAL-02**: watch 功能测试补充，整体行覆盖率达到 92%+
- [ ] **QUAL-03**: macOS FSEvents 限制的 `#[ignore]` 测试调研落地方案（跨平台或 mock 解决）

### 文档

- [ ] **DOC-04**: README 补充 `watch` 用法、`init --interactive` 说明、进度选项（续 v1.16 DOC-01/02/03）
- [ ] **DOC-05**: `watch` / `validate` / `stats` 子命令 `--help` 补充示例和选项说明

## Future Requirements

### watch 扩展

- **WATCH-10**: watch 支持多目录监听（glob 模式）
- **WATCH-11**: watch 支持远程推送（webhook on event）

### 输出格式

- **FMT-01**: JSON Lines 输出格式
- **FMT-02**: Parquet 输出格式

## Out of Scope

| Feature | Reason |
|---------|--------|
| OR 条件组合 | 之前已排除，保持简单过滤模型 |
| 跨字段联合条件 | 之前已排除 |
| 新输出格式（JSON/Parquet） | 保持 CSV/SQLite 双格式（v1.19 不扩展） |
| MultiProgress 多级进度条 | 单行进度条已满足需求 |
| 数值错误码系统（E001/E002） | 过度工程化，thiserror Display 足够 |
| CHANGELOG.md 更新至 v1.18 | 由 DOC-04 README 更新中自然覆盖，不单独作为需求 |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| WATCH-07 | — | Pending |
| WATCH-08 | — | Pending |
| WATCH-09 | — | Pending |
| QUAL-01 | — | Pending |
| QUAL-02 | — | Pending |
| QUAL-03 | — | Pending |
| DOC-04 | — | Pending |
| DOC-05 | — | Pending |

**Coverage:**
- v1.19 requirements: 8 total
- Mapped to phases: 0 (roadmap pending)
- Unmapped: 8 ⚠

---
*Requirements defined: 2026-06-06*
*Last updated: 2026-06-06 — v1.19 initial definition*
