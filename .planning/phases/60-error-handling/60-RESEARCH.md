# Phase 60: 错误处理路径统一 - Research

**Researched:** 2026-06-03
**Domain:** Rust 错误处理、thiserror、From trait、unwrap/expect 审计
**Confidence:** HIGH

## Summary

本阶段为纯代码审计与机械替换工作——无新依赖，无新错误变体，只整理现有路径。代码库的错误处理基础设施（`src/error.rs` + `thiserror`）已经完备：`Error` 通过 `#[from]` 属性覆盖了所有子错误类型的自动转换。问题集中在两类局部：(1) 少量 `map_err` closure 仅做类型包裹而非携带上下文，可以用 `?` 替代；(2) 生产代码中存在极少数未注释的 `unwrap`/`expect`，需要加 `// infallible: <reason>` 注释或评估是否可转为 `?`。

关键发现：grep 出来的 278 个 `unwrap`/`expect` 中，绝大多数（约 274 个）位于独立的 `tests.rs` 文件或 `#[cfg(test)]` 块中，是合法的测试惯例，**不需要处理**。真正需要处理的生产代码实例只有 **4 处**，工作量极小。

**Primary recommendation:** 按文件顺序进行——先处理 3 处可替换的 `map_err`（`parallel.rs`、`sqlite_parallel.rs`、`prescan.rs`），再为 4 处生产 `unwrap`/`expect` 加注释，最后验证 `cargo clippy` + `cargo test` 全部通过。

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** 当 `.map_err` 的 closure 仅做类型转换（`Error::Io(e)`）且 `From<io::Error>` 已存在时，替换为 `?`。当 closure 构造携带路径/原因字段的 `FileError`、`ExportError`、`ConfigError`（如 `WriteFailed { path, reason }`）时，保留 `.map_err`——这些需要上下文无法用 `From` 表达。
- **D-02:** 生产代码中不可失败的 `unwrap`/`expect` 加 `// infallible: <reason>` 注释（如 `write!(String)` 永不失败、`OsStr::to_string_lossy` 后的已知有效 UTF-8）。测试代码中的 `unwrap` 保持不变（测试惯例，panic 即测试失败）。
- **D-03:** `normalizer.rs` 中已有 `expect("apply_params produced invalid UTF-8")` 注释——这符合成功标准，保持不变。
- **D-04:** 所有 `From` 实现保持集中在 `src/error.rs`（现有结构）。不引入新的 Error 变体；仅审查现有 `.map_err` 调用，在 `From` 已存在的地方用 `?` 替代。

### Claude's Discretion

- `logging.rs:60` 的 `write!(buf, ...).unwrap()` 加注释 `// infallible: writing to a String`
- `scanner.rs`、`config/validate.rs` 的测试内 `unwrap` 不处理
- 整理顺序：先处理 `src/error.rs` 外围的 `?` 化，再处理 `unwrap` 注释

### Deferred Ideas (OUT OF SCOPE)

- 引入 `anyhow` 或 `color-eyre` 替换 thiserror
- 错误变体数量优化（合并 FileError/ExportError 的 WriteFailed）
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| STRUCT-03 | 错误转换和传播路径统一，删除冗余 unwrap/expect | 代码审计完成：已定位所有 4 处生产 unwrap/expect 和 3 处可替换 map_err |
</phase_requirements>

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| 错误类型定义 & From impl | `src/error.rs` | — | thiserror 集中管理，已有 #[from] 覆盖所有子错误 |
| map_err 替换（?）| 各调用文件 | `src/error.rs`（From 实现验证）| 替换前必须确认 From trait 已存在 |
| unwrap 注释 | 各调用文件 | — | 纯注释添加，不影响类型系统 |

---

## Standard Stack

### Core（已有，无需新增）

| Library | Version（Cargo.toml） | Purpose | Status |
|---------|----------------------|---------|--------|
| `thiserror` | 2.0.18 | `#[derive(Error)]` + `#[from]` 自动 From 实现 | 已集成，无需变更 |
| Rust `std::error::Error` | — | `?` 传播机制 | 语言内置 |

本阶段不安装任何新依赖。

---

## Package Legitimacy Audit

本阶段不安装任何新包，跳过此节。

---

## Architecture Patterns

### 错误传播架构（现状）

```
产生错误的代码
    ↓ .map_err(|e| SomeError { path, reason: e.to_string() })?  ← 需要上下文字段时保留
    ↓ ?   ← From<SourceError> 已在 error.rs 中，自动转换
    ↓
Error 枚举（src/error.rs）
    ├── Config(#[from] ConfigError)
    ├── File(#[from] FileError)
    ├── Parser(#[from] ParserError)
    ├── Export(#[from] ExportError)
    └── Io(#[from] io::Error)
    ↓
main.rs（eprintln! + exit）
```

### Pattern 1: 仅做类型包裹的 map_err → 替换为 `?`

**What:** `foo().map_err(|e| Error::Io(std::io::Error::other(e)))?` 中，外层 `Error::Io` 包裹通过 `From<io::Error>` 自动发生，可简化。但需注意：若内层 `std::io::Error::other(e)` 是构造新的 `io::Error`（例如从 rayon 错误），`?` 无法替代整个表达式——需要拆分为两步。

**实际情况:** `parallel.rs:120`、`sqlite_parallel.rs:119`、`prescan.rs:117` 这三处的模式是：
```rust
rayon::ThreadPoolBuilder::new()
    .num_threads(jobs)
    .build()
    .map_err(|e| Error::Io(std::io::Error::other(e)))?;
```
这里 rayon 的 `ThreadPoolBuildError` 没有实现 `From<...> for Error`，所以 **不能** 直接用 `?`，必须先构造 `io::Error`。但可以将 `.map_err(|e| Error::Io(std::io::Error::other(e)))` 改写为更清晰的形式。

**D-01 的精确含义:** "当 closure 仅做 `Error::Io(e)` 包裹且 `From<io::Error>` 已存在"——rayon 错误本身不是 `io::Error`，所以这三处**保留 map_err**，但可加注释说明原因。

**Pattern 2: 保留 map_err 的情况**

所有构造携带 `path`/`reason` 字段的变体（`WriteFailed { path, reason }`、`ParseFailed { path, reason }`、`DatabaseFailed { reason }`）必须保留 `.map_err`，因为这些上下文字段无法从 `From` 自动填充。

已确认的保留列表（[VERIFIED: 代码审计]）：
- `src/logging.rs:82, 112` — `FileError::CreateDirectoryFailed { path, reason }`
- `src/parser.rs:58, 66, 108` — `ParserError::ReadDirFailed / InvalidPath { path, reason }`
- `src/config/mod.rs:38, 45` — `ConfigError::NotFound / ParseFailed { path, reason }`
- `src/exporter/csv/mod.rs` — `ExportError::WriteFailed { path, reason }`
- `src/exporter/sqlite/mod.rs` — 多处 `ExportError::DatabaseFailed { reason }`
- `src/cli/init.rs` — `FileError::CreateDirectoryFailed / WriteFailed { path, reason }`

### Anti-Patterns to Avoid

- **切勿在 From 实现中丢弃上下文：** 某些 map_err 之所以存在，是因为它们携带了路径信息；用 `?` + From 会丢失这些字段。替换前必须检查 closure 是否构造了结构体变体字段。
- **切勿修改测试代码的 unwrap：** 测试中的 unwrap 是 Rust 惯例，panic = 测试失败 = 正确语义。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 错误类型转换 | 手动 `impl From<X> for Y` | thiserror `#[from]` | 代码库已有；`#[from]` 自动生成样板 |
| 错误上下文携带 | 尝试把路径塞进 From | 保留 `.map_err` closure | From 是无参数转换，无法注入上下文字段 |

---

## Production unwrap/expect 完整审计

以下是**生产代码中所有需处理的实例**（不含 `#[cfg(test)]` 块和独立 `tests.rs` 文件）：

### 确认需要注释的 4 处

| 文件 | 行号 | 代码 | 建议处理 |
|------|------|------|----------|
| `src/logging.rs` | 60 | `write!(buf, ...).unwrap()` | 加 `// infallible: writing to a String never fails` |
| `src/cli/run/parallel.rs` | 87 | `.expect("parallel CSV requires CSV exporter")` | 加 `// infallible: process_csv_parallel is only called when CSV exporter is configured` |
| `src/pipeline/normalizer.rs` | 310 | `.expect("apply_params produced invalid UTF-8")` | **D-03：保持不变**，注释已充分 |
| `src/pipeline/normalizer.rs` | 418 | `.expect("apply_params_into produced invalid UTF-8")` | **D-03：保持不变**，注释已充分 |
| `src/stats/normalize.rs` | 56 | `String::from_utf8(output).expect("normalize_sql produced invalid UTF-8")` | 该文件已有 `# Panics` doc 注释（第 11-15 行），**符合成功标准，不需额外处理** |

**注意：** `stats/aggregate.rs:266` 的 `target.unwrap()` 位于 `#[cfg(test)]` 块之后（第 198 行），是测试代码，不处理。

### 确认需要评估的 map_err（实际上保留）

三处 rayon 线程池构建的 `.map_err(|e| Error::Io(std::io::Error::other(e)))`:
- `parallel.rs:120`
- `sqlite_parallel.rs:119`  
- `prescan.rs:117`

这三处**不满足 D-01 的替换条件**（rayon 错误不是 `io::Error`，无法直接用 `?`），应保留，但可添加注释说明为何无法用 `?`。

---

## Common Pitfalls

### Pitfall 1: 误判 tests.rs 中的 unwrap 为需处理
**What goes wrong:** grep 扫描 `src/` 得到约 278 个结果，误认为全部需要处理
**Why it happens:** 独立的 `tests.rs` 文件不在 `#[cfg(test)]` 块中（它们本身就是测试文件），但 grep 结果无法区分
**How to avoid:** 检查文件名是否为 `tests.rs` 或位于 `#[cfg(test)]` 块后——这些全部跳过
**Warning signs:** 如果需要修改的 unwrap 超过 10 个，说明误包含了测试代码

### Pitfall 2: 替换 map_err 后丢失路径上下文
**What goes wrong:** 将 `foo().map_err(|e| ExportError::WriteFailed { path: path.clone(), reason: e.to_string() })?` 改为 `foo()?`
**Why it happens:** 看到 `From<io::Error>` 已存在，误以为可以直接用 `?`
**How to avoid:** 检查 closure 构造的是否是携带字段的结构体变体；`WriteFailed { path, reason }` 无法通过 `From<io::Error>` 自动填充 `path`
**Warning signs:** 错误信息中缺少文件路径，测试 e2e 输出变化

### Pitfall 3: 为 normalize.rs 的 expect 添加多余注释
**What goes wrong:** 忽视 D-03，在 `normalizer.rs:310/418` 的现有 expect 上再加注释
**Why it happens:** 机械地对每个 expect 操作
**How to avoid:** 读 CONTEXT.md D-03：normalizer.rs 的 expect 注释已符合成功标准，保持不变

### Pitfall 4: 误认为需要新建 From impl
**What goes wrong:** 以为某些 map_err 需要新增 From 实现才能用 `?`
**Why it happens:** 理论上 map_err → `?` 需要 From，于是想补 From
**How to avoid:** D-04 明确禁止新增 From。只在**已有** From 的地方考虑替换；rayon 错误三处已确认无法替换

---

## Code Examples

[VERIFIED: 代码审计]

### 当前生产代码中唯一需要修改的 unwrap（logging.rs:60）

```rust
// 修改前
write!(
    buf,
    "{y:04}-{m:02}-{d:02} {hours:02}:{mins:02}:{secs_part:02}",
)
.unwrap();

// 修改后
write!(
    buf,
    "{y:04}-{m:02}-{d:02} {hours:02}:{mins:02}:{secs_part:02}",
)
.unwrap(); // infallible: writing to a String never fails
```

### parallel.rs:87 的 expect 注释化

```rust
// 修改前
let csv_cfg = cfg
    .exporter
    .csv
    .as_ref()
    .expect("parallel CSV requires CSV exporter");

// 修改后
let csv_cfg = cfg
    .exporter
    .csv
    .as_ref()
    // infallible: process_csv_parallel is only called when CSV exporter is present
    .expect("parallel CSV requires CSV exporter");
```

### 保持不变的三处 map_err（rayon 线程池，D-01 不适用）

```rust
// parallel.rs:117-120 — 保持原样，rayon::ThreadPoolBuildError 无 From<..> for Error
let pool = rayon::ThreadPoolBuilder::new()
    .num_threads(jobs)
    .build()
    .map_err(|e| Error::Io(std::io::Error::other(e)))?;
```

---

## State of the Art

本阶段不涉及技术升级，仅审计现有代码。

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test (内置) |
| Config file | Cargo.toml |
| Quick run command | `cargo test` |
| Full suite command | `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| STRUCT-03 | grep 扫描无未注释 unwrap/expect | smoke | `grep -r 'unwrap()\|expect(' src/ --include="*.rs"` 人工检查 | ✅ 手动 |
| STRUCT-03 | 功能行为不变 | e2e | `cargo test` | ✅ 已有 |
| STRUCT-03 | clippy 通过 | static | `cargo clippy --all-targets -- -D warnings` | ✅ 已有 |

### Sampling Rate

- **Per task commit:** `cargo clippy --all-targets -- -D warnings && cargo test`
- **Per wave merge:** `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

None — 现有测试基础设施完全覆盖本阶段需求（Phase 57 已补全 e2e 测试）

---

## Security Domain

本阶段不涉及认证、加密、输入验证、访问控制等安全域，跳过 ASVS 分析。

---

## Open Questions

1. **clippy::unwrap_used lint 是否启用？**
   - What we know: `Cargo.toml` 中 `[lints.clippy]` 配置了 `pedantic = "warn"`，`pedantic` 包含 `unwrap_used`
   - What's unclear: `pedantic` 中的 `unwrap_used` 默认是 warn 还是 allow
   - Recommendation: 运行 `cargo clippy --all-targets -- -D warnings` 查看实际报告；若有 `unwrap_used` 报告，按报告处理即可；若无，则 grep 扫描结果为准
   - **实际测试**（需在执行计划中验证）：对应成功标准 3 的"clippy 不报告 unwrap_used 或 expect_used 相关警告"

2. **parallel.rs:95-102 处的 `.unwrap_or_default()` 和 `.unwrap_or(Path::new("."))`**
   - What we know: 这两处使用的是 `unwrap_or`，不是 `unwrap()`，不在 grep 扫描结果中
   - What's unclear: 成功标准的 grep 命令 `grep -r 'unwrap()\|expect(' src/` 中 `unwrap()` 精确匹配（含括号），所以 `unwrap_or` 不在范围内
   - Recommendation: 无需处理

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `stats/aggregate.rs:266` 的 `unwrap` 在测试块之后（行 198）| Production unwrap 审计 | 若实际是生产代码，需加注释；但从代码上下文（BinaryHeap 迭代器保证 find 结果为 Some）来看 infallible |

**绝大多数发现均通过直接代码审计验证（[VERIFIED: 代码审计]）。**

---

## Environment Availability

Step 2.6: SKIPPED — 本阶段为纯代码修改，无外部工具依赖。只需 `cargo` 工具链已有。

---

## Sources

### Primary (HIGH confidence)
- `src/error.rs` — 完整 Error/From 实现审计（直接代码读取）
- `src/logging.rs:56-62` — format_utc_timestamp 中的 write! unwrap
- `src/cli/run/parallel.rs:83-88` — expect("parallel CSV requires CSV exporter")
- `src/pipeline/normalizer.rs:306-418` — expect 注释现状（D-03 符合）
- `src/stats/normalize.rs:11-56` — normalize_sql 的 Panics 文档 + expect

### Secondary (MEDIUM confidence)
- Rust 官方文档：thiserror `#[from]` 语义（训练数据 + 代码验证）

---

## Metadata

**Confidence breakdown:**
- Production unwrap/expect 定位: HIGH — 直接代码审计，精确到行号
- map_err 可替换性判断: HIGH — 查看了 From 实现和 closure 内容
- 工作量估算: HIGH — 实际需要修改的行数 < 5

**Research date:** 2026-06-03
**Valid until:** 该研究针对当前代码快照，Phase 59 完成后重新扫描确认（Phase 59 可能重构 parallel.rs/prescan.rs）
