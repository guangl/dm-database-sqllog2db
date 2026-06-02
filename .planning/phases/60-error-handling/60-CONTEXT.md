# Phase 60: 错误处理路径统一 - Context

**Gathered:** 2026-06-03
**Status:** Ready for planning

<domain>
## Phase Boundary

整个代码库的错误转换和传播路径统一：冗余的 `unwrap`/`expect` 替换为显式错误传播或加注释说明不可失败原因；手动 `.map_err` 在无需携带额外上下文时替换为 `?` + `From` 自动转换；`From` 实现集中在 `src/error.rs`。不修改功能行为，不引入新错误变体。

</domain>

<decisions>
## Implementation Decisions

### map_err 替换策略

[auto] Q: "哪些 `.map_err` 可以替换为 `?`，哪些应保留？" → Selected: "按需保留" (recommended default)

- **D-01:** 当 `.map_err` 的 closure 仅做类型转换（`Error::Io(e)`）且 `From<io::Error>` 已存在时，替换为 `?`。当 closure 构造携带路径/原因字段的 `FileError`、`ExportError`、`ConfigError`（如 `WriteFailed { path, reason }`）时，保留 `.map_err`——这些需要上下文无法用 `From` 表达。

### unwrap/expect 处理方式

[auto] Q: "生产代码中不可失败的 unwrap 如何处理？" → Selected: "加注释" (recommended default)

- **D-02:** 生产代码中不可失败的 `unwrap`/`expect` 加 `// infallible: <reason>` 注释（如 `write!(String)` 永不失败、`OsStr::to_string_lossy` 后的已知有效 UTF-8）。测试代码中的 `unwrap` 保持不变（测试惯例，panic 即测试失败）。
- **D-03:** `normalizer.rs` 中已有 `expect("apply_params produced invalid UTF-8")` 注释——这符合成功标准，保持不变。

### From impl 位置

[auto] Q: "From 实现放在哪里？" → Selected: "集中在 src/error.rs" (recommended default)

- **D-04:** 所有 `From` 实现保持集中在 `src/error.rs`（现有结构）。不引入新的 Error 变体；仅审查现有 `.map_err` 调用，在 `From` 已存在的地方用 `?` 替代。

### Claude's Discretion

- `logging.rs:60` 的 `write!(buf, ...).unwrap()` 加注释 `// infallible: writing to a String`
- `scanner.rs`、`config/validate.rs` 的测试内 `unwrap` 不处理
- 整理顺序：先处理 `src/error.rs` 外围的 `?` 化，再处理 `unwrap` 注释

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求来源
- `.planning/ROADMAP.md` §"Phase 60: 错误处理路径统一" — Goal、Success Criteria（4 条）
- `.planning/REQUIREMENTS.md` §STRUCT-03

### 关键源文件（实现前必须全量阅读）
- `src/error.rs` — 所有错误类型定义及现有 `From` 实现（主要目标文件）
- `src/logging.rs` — 生产 `unwrap` 实例（`:60` infallible）
- `src/pipeline/normalizer.rs` — 已文档化的 `expect` 调用（`:310`, `:418`）
- `src/cli/run/parallel.rs` — `map_err(|e| Error::Io(...))` 可能替换为 `?`（`:120`）
- `src/cli/run/sqlite_parallel.rs` — 同上（`:119`）
- `src/cli/run/prescan.rs` — `map_err(|e| Error::Io(...))` 可能替换（`:117`）

### 存在 map_err 的生产文件（需逐一审查）
- `src/logging.rs` (`:82`, `:112`) — 构造 Error，需评估是否可 `?`
- `src/parser.rs` (`:58`, `:66`, `:108`) — 构造 ParserError，携带路径，保留 `.map_err`
- `src/config/mod.rs` (`:38`, `:45`) — 构造 ConfigError，携带路径/原因，保留 `.map_err`
- `src/exporter/csv/mod.rs` — 构造 ExportError::WriteFailed，携带路径，保留 `.map_err`
- `src/exporter/sqlite/mod.rs` — 构造 ExportError::DatabaseFailed，保留 `.map_err`
- `src/cli/init.rs` — 构造 FileError，保留 `.map_err`

</canonical_refs>

<code_context>
## Existing Code Insights

### 已有 From 实现
- `src/error.rs` 中：`#[from] ConfigError`、`#[from] FileError`、`#[from] ParserError`、`#[from] ExportError`、`#[from] io::Error`——这些已支持 `?` 自动转换

### 可直接替换为 `?` 的模式
- `parallel.rs:120`：`.map_err(|e| Error::Io(std::io::Error::other(e)))?` — `io::Error::other` 构造可保留，但外层 `Error::Io` wrap 通过 `From<io::Error>` 自动发生（如果 e 已是 `io::Error`）
- `prescan.rs:117`：同上模式

### 约束
- `thiserror` `#[from]` 属性已覆盖大多数转换，手动 `map_err` 多出现在构造携带上下文字段的变体（`WriteFailed { path, reason }`）——这些必须保留
- `scanner.rs` 的 `unwrap()` 在测试中，不在生产路径

</code_context>

<specifics>
## Specific Ideas

- 执行顺序：先审查 `parallel.rs`/`prescan.rs`/`sqlite_parallel.rs` 的 `Error::Io` wrap（最易替换），再审查 `logging.rs` 的两处 `map_err`，最后处理 `unwrap` 注释
- 成功标准 3 要求 clippy 通过且无 `unwrap_used`/`expect_used` 警告——需确认 `Cargo.toml` 是否启用了 `clippy::unwrap_used` lint；如未启用，则以 grep 扫描结果为准

</specifics>

<deferred>
## Deferred Ideas

- 引入 `anyhow` 或 `color-eyre` 替换 thiserror — 超出本阶段工程目标，且 thiserror 已满足需求
- 错误变体数量优化（合并 FileError/ExportError 的 WriteFailed）— 属于接口重构，留给后续里程碑

</deferred>

---

*Phase: 60-error-handling*
*Context gathered: 2026-06-03*
