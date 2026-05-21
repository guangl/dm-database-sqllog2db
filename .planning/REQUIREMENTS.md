# Requirements: sqllog2db

**Defined:** 2026-05-21
**Core Value:** 用户能够精确指定"导出哪些记录的哪些字段"——过滤逻辑清晰可配置，输出结果完全可控

## v1.10 Requirements

### 核心验证

- [ ] **VER-01**: CSV 导出端到端验证通过（含边界情况：空文件、大文件、特殊字符）
- [ ] **VER-02**: SQLite 导出端到端验证通过（含 schema 正确性、数据完整性）
- [ ] **VER-03**: Pipeline 过滤器（include/exclude/indicators/sql）验证通过
- [ ] **VER-04**: 参数归一化验证通过（达梦 SQL 参数替换正确）
- [ ] **VER-05**: 并行 CSV 处理验证通过（rayon 多线程输出正确拼接）
- [ ] **VER-06**: `cargo build --release` + `cargo test` + `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` 全通过

### 错误处理

- [ ] **ERR-01**: 错误类型细分（IO 错误 / 格式解析错误 / 配置错误 / 导出错误），每种错误包含路径和行号上下文
- [ ] **ERR-02**: 非致命错误（解析失败、单条导出失败）记录到 error log 后继续处理，不终止整个导出
- [ ] **ERR-03**: 错误信息包含三要素：发生了什么、在哪发生（文件:行号）、建议修复

### 管道输入

- [ ] **PIPE-01**: 支持 stdin 作为输入源（`--input -` 或 `cat log | sqllog2db run`），通过 `/dev/stdin` 路径映射实现
- [ ] **PIPE-02**: stdin 模式下自动跳过文件发现和 pre-scan，事务级过滤降级时明确警告用户

### CLI 体验

- [ ] **UX-01**: 处理进度实时显示（每 1024 条更新一次，非终端自动退化为文本状态）
- [ ] **UX-02**: 处理完成后输出统计摘要（总记录数、成功数、错误数、处理速率、总耗时）
- [ ] **UX-03**: `--help` 输出包含 3-4 个达梦场景的实用示例（通过 clap `after_help`）
- [ ] **UX-04**: 错误输出格式统一，非致命错误实时输出到 stderr（不含进度条干扰）

## Out of Scope

| Feature | Reason |
|---------|--------|
| OR 条件组合 | 之前已排除，保持简单过滤模型 |
| 跨字段联合条件 | 之前已排除 |
| 新输出格式（JSON/Parquet） | 保持 CSV/SQLite 双格式，v1.10 聚焦质量 |
| 上游 parser crate 添加 `from_reader()` API | 超出 sqllog2db 范围，后续考虑向上游 PR |
| MultiProgress 多级进度条 | P1，第一版用单行进度条 |
| 数值错误码系统（E001/E002） | 过度工程化，thiserror Display 足够 |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| VER-01 | 39 | Pending |
| VER-02 | 40 | Pending |
| VER-03 | 39 | Pending |
| VER-04 | 39 | Pending |
| VER-05 | 40 | Pending |
| VER-06 | 40 | Pending |
| ERR-01 | 36 | Pending |
| ERR-02 | 36 | Pending |
| ERR-03 | 36 | Pending |
| PIPE-01 | 37 | Pending |
| PIPE-02 | 37 | Pending |
| UX-01 | 38 | Pending |
| UX-02 | 38 | Pending |
| UX-03 | 35 | Pending |
| UX-04 | 37 | Pending |

**Coverage:**
- v1.10 requirements: 15 total
- Mapped to phases: 15
- Unmapped: 0 ✅

---
*Requirements defined: 2026-05-21*
*Last updated: 2026-05-21 after v1.10 milestone definition*
