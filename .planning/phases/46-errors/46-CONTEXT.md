# Phase 46: 错误信息优化 - Context

**Gathered:** 2026-05-31
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 46 完善用户看到的错误信息展示：将所有错误变体的 `Suggestion:` 前缀统一改为 `hint:`，并为当前缺失 hint 的 `Error::Io` 变体添加通用提示。TOML 解析错误依赖库自带的字段信息，不额外实现 serde path 提取。

本 phase 不改变 Error 枚举结构，不新增错误变体，不调整日志级别机制。

</domain>

<decisions>
## Implementation Decisions

### 错误展示格式
- **D-01:** 保留 `[SEVERITY]`（WARNING/ERROR/CRITICAL）前缀，只将 `Suggestion:` 改为 `hint:`。格式变为：
  ```
  [CRITICAL] Configuration error: At least one exporter must be configured
    hint: Enable at least one exporter: [csv] or [sqlite].
  ```
- **D-02:** TOML 格式错误（`ConfigError::ParseFailed`）依赖 `toml::from_str` 自带的字段名和行号信息，不额外实现自定义 deserializer 或 serde path 提取。

### Error::Io hint 补全
- **D-03:** 为 `Error::Io` 在 `suggestion()` 方法中添加通用 hint：`"Check filesystem permissions and disk space."`。与其他变体 hint 风格保持一致。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 核心文件
- `src/error.rs` — Error 枚举、ErrorSeverity、suggestion() 方法完整实现
- `src/main.rs` — 错误打印路径（`[{sev}] {e}` + `Suggestion: {suggestion}` 的具体代码位置）

No external specs — requirements fully captured in decisions above

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/error.rs::Error::suggestion()` — 已有方法，需要将每个 match arm 中的 `Suggestion:` 改为 `hint:`（注意：是 `main.rs` 中打印时加的前缀，不在 `suggestion()` 返回值里）
- `src/main.rs` 错误打印代码约 80 行：`eprintln!("[{sev}] {e}")` + `eprintln!("  Suggestion: {suggestion}")` — 只需改第二行

### Established Patterns
- `thiserror` 用于 Display 格式，错误文本已包含 path/field 上下文
- `Error::Io` 目前：`#[error("IO error: {0}")]`，`suggestion()` 返回空字符串 `""`

### Integration Points
- 仅需改 `main.rs` 中的 `eprintln!` 调用（`Suggestion:` → `hint:`）
- 仅需改 `error.rs` 中 `Error::Io` 的 `suggestion()` match arm（空字符串 → 通用 hint）

</code_context>

<specifics>
## Specific Ideas

无特殊参考要求——按标准修改即可。

</specifics>

<deferred>
## Deferred Ideas

无——讨论保持在 phase 边界内。

</deferred>

---

*Phase: 46-错误信息优化*
*Context gathered: 2026-05-31*
