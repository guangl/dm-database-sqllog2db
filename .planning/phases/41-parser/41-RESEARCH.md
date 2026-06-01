# Phase 41: 依赖升级与 Parser 库适配 - Research

**Researched:** 2026-05-24
**Domain:** Rust 依赖管理 / dm-database-parser-sqllog API 迁移
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** 直接升级 `dm-database-parser-sqllog` 到 2.0.0（major 版本升级）。
- **D-02:** Phase 41 只做"升级 + 编译通过 + 无 deprecated 警告"，不做深度 API 重构（留给 Phase 43）。
- **D-03:** 其他依赖同步 `cargo update` 到最新兼容 minor/patch。
- **D-04:** `cargo build --release` 无任何 `warning:` 行（包括 deprecated），`cargo test` 全部通过，`cargo clippy --all-targets -- -D warnings` 通过。
- **D-05:** Cargo.lock 中 `dm-database-parser-sqllog` 版本号高于当前 1.1.0。

### Claude's Discretion
- 如果 2.0.0 有编译级 breaking changes（如字段重命名），做最小化适配使编译通过，记录待深度重构的 TODO 供 Phase 43 参考。

### Deferred Ideas (OUT OF SCOPE)
- 利用新 API（FilterBuilder、from_reader 等）删除冗余映射代码 → Phase 43
- AsyncLogParser tokio 异步接口 → 超出本 milestone 范围，暂不考虑
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REFACTOR-02 | Cargo.toml 所有依赖升级到最新兼容版本，`cargo update` 后 `cargo test` 全部通过 | `cargo update --dry-run` 已确认可升级的 patch 版本列表；`dm-database-parser-sqllog` 需手动 bump major 版本 |
| PARSER-01 | 用户使用最新版 `dm-database-parser-sqllog`，`cargo build --release` 编译成功，无 deprecated 警告 | 2.0.0 的 `LogParserBuilder` / `Sqllog` 字段 API 与 1.1.0 完全一致（无 breaking changes）；编译通过无需代码修改 |
</phase_requirements>

---

## Summary

`dm-database-parser-sqllog 2.0.0`（2026-05-23 发布）是纯加法式 major 版本升级：**公共 API 与 1.1.0 完全兼容**，没有任何 breaking changes。新版本增加了 `FilterBuilder` 链式过滤系统、`AsyncLogParser` tokio 接口（可选 feature）和 `LogIterator` 的两个新方法，但这些新 API 均为加法，现有代码无需修改即可编译通过。[VERIFIED: docs.rs/dm-database-parser-sqllog + CHANGELOG.md]

`Sqllog` 结构体的 14 个字段（`ts`、`tag`、`ep`、`sess_id`、`thrd_id`、`username`、`trxid`、`statement`、`appname`、`client_ip`、`sql`、`exectime`、`rowcount`、`exec_id`）在两个版本中**完全相同**，类型也相同。`LogParserBuilder::new().build()` 和 `parser.iter()` 调用签名不变。[VERIFIED: docs.rs]

对于其他依赖，`cargo update --dry-run` 显示 10 个包可升级到最新兼容版本（均为 minor/patch），`criterion` 因 rust-version 约束（0.8.x 需要 Rust 1.86，项目声明 rust-version = "1.85"）被锁定在 0.7.0，无法通过 `cargo update` 自动升级。[VERIFIED: cargo info + cargo update --dry-run]

**Primary recommendation:** 修改 `Cargo.toml` 中 `dm-database-parser-sqllog = "2.0.0"` 后直接运行 `cargo build --release`，理论上零代码修改即可编译通过；同时运行 `cargo update` 升级其他 patch 版本依赖。

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| 依赖版本管理 | 构建系统 (Cargo.toml) | — | Cargo 的语义化版本控制 |
| Parser API 适配 | 业务逻辑层 (src/cli/run/) | 管线层 (src/pipeline/) | parser 被 processor.rs / prescan.rs / pipeline/mod.rs 直接调用 |
| 编译验证 | CI/本地构建 | — | `cargo build --release` + `cargo clippy` |

---

## Standard Stack

### Core（本阶段修改）

| Library | 当前版本 | 目标版本 | 升级方式 | 备注 |
|---------|---------|---------|---------|------|
| `dm-database-parser-sqllog` | 1.1.0 | 2.0.0 | 手动修改 Cargo.toml | major 升级，但公共 API 兼容 [VERIFIED: CHANGELOG] |

### 其他依赖（cargo update 自动升级）

| Library | 当前版本 | 可升级版本 | 说明 |
|---------|---------|---------|------|
| `autocfg` | 1.5.0 | 1.5.1 | build dep，透明升级 [VERIFIED: cargo update --dry-run] |
| `bumpalo` | 3.20.2 | 3.20.3 | 间接依赖 [VERIFIED] |
| `either` | 1.15.0 | 1.16.0 | rayon 的依赖 [VERIFIED] |
| `js-sys` | 0.3.98 | 0.3.99 | 间接依赖 [VERIFIED] |
| `serde_json` | 1.0.149 | 1.0.150 | 间接依赖 [VERIFIED] |
| `wasm-bindgen` 系列 | 0.2.121 | 0.2.122 | 多个间接依赖 [VERIFIED] |
| `web-sys` | 0.3.98 | 0.3.99 | 间接依赖 [VERIFIED] |

### 无法自动升级（版本约束）

| Library | 当前版本 | 最新版本 | 原因 |
|---------|---------|---------|------|
| `criterion` | 0.7.0 | 0.8.2 | 0.8.x 需要 Rust 1.86；项目 rust-version = "1.85" [VERIFIED: cargo info criterion@0.8.2] |
| `wasip2` / `wasip3` | 锁定 | 更新版 | 需要 Rust 1.87 [VERIFIED: cargo update --dry-run verbose] |

> `criterion 0.7` 保持不变，不需要任何操作。Phase 42 再评估是否随 rust-version 一起升级。

**升级命令：**
```bash
# Step 1: 手动修改 Cargo.toml
# dm-database-parser-sqllog = "2.0.0"

# Step 2: 更新 Cargo.lock
cargo update

# Step 3: 验证编译
cargo build --release 2>&1 | grep -E "^error:|^warning:"

# Step 4: 运行测试
cargo test

# Step 5: 运行 clippy
cargo clippy --all-targets -- -D warnings
```

---

## Package Legitimacy Audit

> slopcheck 在此环境不可用，改用 `cargo search` + `cargo info` + crates.io 来源验证。

| Package | Registry | 来源 | cargo info 版本 | slopcheck | Disposition |
|---------|----------|------|----------------|-----------|-------------|
| `dm-database-parser-sqllog` | crates.io | github.com/guangl/dm-database-parser-sqllog | 2.0.0 ✓ | N/A (slopcheck 不可用) | Approved — 与本项目同一作者，已使用 1.1.0 [VERIFIED: cargo search] |

**Packages removed due to slopcheck [SLOP] verdict:** none

**Packages flagged as suspicious [SUS]:** none

*slopcheck 在此环境不可用。`dm-database-parser-sqllog` 是本项目作者自己发布的库（同一 github.com/guangl 命名空间），风险极低。*

---

## Architecture Patterns

### 当前 Parser API 使用模式

```
Cargo.toml: dm-database-parser-sqllog = "1.1.0"  →  2.0.0
                    ↓
LogParserBuilder::new(file_path)    (不变)
         .build()?                  (不变)
                    ↓
parser.iter()                       (不变，返回 LogIterator)
  每次产出 Result<Sqllog, ParseError>
                    ↓
record.exec_id / exectime / rowcount / sql / tag / trxid ...   (字段名不变)
```

### Parser 使用位置汇总

| 文件 | 用法 | 2.0.0 兼容性 |
|------|------|-------------|
| `src/cli/run/processor.rs:53` | `LogParserBuilder::new(file_path).build()` | ✓ 完全兼容 |
| `src/cli/run/processor.rs:65` | `parser.iter()` + 字段访问 | ✓ 完全兼容 |
| `src/cli/run/prescan.rs:16` | `LogParserBuilder::new(file_path).build()` | ✓ 完全兼容 |
| `src/cli/run/prescan.rs:25` | `parser.iter().filter_map(Result::ok).collect()` | ✓ 完全兼容 |
| `src/cli/run/parallel.rs` | 通过 `process_log_file` 间接使用 | ✓ 完全兼容 |
| `src/pipeline/mod.rs:332` | `LogParserBuilder::new().build()` + `parser.iter().flatten()` | ✓ 完全兼容 |
| `src/exporter/sqlite/tests.rs` | 多处 `LogParserBuilder::new().build()` + `parser.iter()` | ✓ 完全兼容 |
| `src/pipeline/mod.rs:8` | `use dm_database_parser_sqllog::Sqllog` | ✓ 完全兼容 |

> `parallel.rs` 中的 `par_iter()` 调用（line 127）是对 `log_files`（`&[PathBuf]`）的 rayon 并行，**不是**对 parser 的并行迭代，与 parser 版本无关。

### Recommended Project Structure

升级不涉及目录结构变化，现有结构保持不变：

```
src/
├── cli/run/
│   ├── processor.rs    # 主要 parser 使用点
│   ├── prescan.rs      # 预扫描 parser 使用点
│   └── parallel.rs     # 通过 processor 间接使用
├── pipeline/           # Sqllog 类型引用
└── exporter/           # Sqllog 类型引用
```

### Anti-Patterns to Avoid

- **不要升级 criterion 到 0.8**：需要 Rust 1.86，而项目声明 rust-version = "1.85"，会破坏 Rust MSRV 保证。
- **不要使用新的 FilterBuilder API**：Phase 43 的工作，Phase 41 只做最小化适配。
- **不要修改现有字段访问代码**：2.0.0 字段名与 1.1.0 完全相同，修改反而引入回归风险。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 依赖版本升级 | 手动分析所有 transitive deps | `cargo update` | Cargo 的 semver 解析完全处理兼容性 |
| deprecated API 检测 | 手动审查代码 | `cargo build 2>&1 \| grep deprecated` | 编译器会直接报告 deprecated 警告 |
| 编译错误定位 | 手动猜测受影响的文件 | `cargo build 2>&1 \| grep error` | 编译器错误精确指向行号 |

**Key insight:** major 版本号不意味着 breaking changes——在此案例中 2.0.0 是纯加法升级，公共 API 向后兼容 1.1.0。

---

## Common Pitfalls

### Pitfall 1: criterion 版本约束混淆
**What goes wrong:** 在 Cargo.toml 写 `criterion = "0.8"` 后 `cargo build` 报错 "package requires Rust 1.86"
**Why it happens:** 项目 `rust-version = "1.85"`，criterion 0.8.x 最低需要 Rust 1.86
**How to avoid:** criterion 保持 `"0.7"`，不在本阶段动它
**Warning signs:** `cargo update --dry-run` 输出 `Unchanged criterion v0.7.0 (available: v0.8.2, requires Rust 1.86)`

### Pitfall 2: 误以为 major 版本必有 breaking changes
**What goes wrong:** 计划了"API 适配"任务但实际没有任何代码需要改
**Why it happens:** SemVer 的 major bump 允许但不强制包含 breaking changes
**How to avoid:** 先读 CHANGELOG，确认实际变更范围，再制定任务
**Warning signs:** CHANGELOG 明确说 "public API remains unchanged"

### Pitfall 3: 遗漏测试文件中的 parser 使用
**What goes wrong:** 只改 processor.rs / prescan.rs，遗漏 `src/exporter/sqlite/tests.rs` 中的 `LogParserBuilder` 调用
**Why it happens:** 测试文件散落在各模块目录下
**How to avoid:** 用 `grep -rn "LogParserBuilder\|dm_database_parser_sqllog" src/` 全量扫描
**Warning signs:** `cargo test` 报错而 `cargo build` 通过

### Pitfall 4: cargo update 无法升级 major 版本
**What goes wrong:** 运行 `cargo update` 后 Cargo.lock 中 `dm-database-parser-sqllog` 仍为 1.1.0
**Why it happens:** `cargo update` 只升级 semver 兼容的版本（即 ^1.x 范围内），2.0.0 不在 ^1.1.0 范围内
**How to avoid:** 必须先手动修改 Cargo.toml 中的版本要求为 `"2.0.0"` 或 `"^2"`，再运行 `cargo update`（或直接 `cargo build`）
**Warning signs:** 修改完 Cargo.toml 但忘记更新 Cargo.lock

---

## Code Examples

### Pattern 1: 升级后 processor.rs 的 parser 初始化（无需修改）

```rust
// Source: 当前 src/cli/run/processor.rs:53 — 与 2.0.0 API 完全兼容
let parser = LogParserBuilder::new(file_path).build().map_err(|e| {
    crate::error::Error::Parser(crate::error::ParserError::InvalidPath {
        path: file_path.into(),
        reason: format!("{e}"),
        line_number: None,
    })
})?;

// 迭代 — 不变
for result in parser.iter() {
    // ...
}
```

### Pattern 2: prescan.rs 的注释可在升级后清理（可选，Phase 43）

```rust
// 当前（v1.1.0 时代的变通注释，2.0.0 行为相同，注释可删除但不必要）
// 收集到 Vec 再并行处理（v1.1.0 的 LogParser 不再实现 rayon 的 IntoParallelRefIterator）
let records: Vec<_> = parser.iter().filter_map(std::result::Result::ok).collect();
```

> Phase 41 不需要改动这段代码，注释清理留给 Phase 43。

### Pattern 3: Cargo.toml 修改（唯一必要变更）

```toml
# Before
dm-database-parser-sqllog = "1.1.0"

# After
dm-database-parser-sqllog = "2.0.0"
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `dm-database-parser-sqllog 1.1.0` — 无 FilterBuilder | 2.0.0 — 增加 FilterBuilder 56 个谓词方法 | 2026-05-23 (2.0.0) | Phase 43 可利用新 API 删除冗余代码 |
| `dm-database-parser-sqllog 1.x` — 有 rayon par_iter | 1.1.0 起已移除 rayon 并行 | 2026-05-21 (1.1.0 breaking) | prescan.rs 已用 Vec 收集后 par_iter 变通 |

**Deprecated/outdated:**
- prescan.rs 注释 `"v1.1.0 的 LogParser 不再实现 rayon 的 IntoParallelRefIterator"` — 2.0.0 仍然如此（不是回归），注释本身没有错，但在 Phase 43 整理注释时可以删除或更新为更简洁的说明。

---

## Assumptions Log

> 本研究中所有关键声明均已通过 docs.rs、cargo info、CHANGELOG 或 cargo update --dry-run 验证。

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `dm-database-parser-sqllog 2.0.0` 的 `Sqllog` 字段类型与 1.1.0 完全一致（14 个字段名和类型均相同） | Standard Stack | 若有字段类型变化，需额外修改字段访问代码 |

> A1 基于 docs.rs 文档对比，两个版本文档均显示相同的 14 个字段。风险极低。

---

## Open Questions

1. **`dm-database-parser-sqllog 2.0.0` 的新依赖是否带来额外的编译时间或包体积开销？**
   - What we know: 2.0.0 增加了 FilterBuilder（内部模块），`async` 是可选 feature，不启用则不拉入 tokio
   - What's unclear: 是否引入了新的非可选传递依赖
   - Recommendation: 升级后运行 `cargo tree -p dm-database-parser-sqllog` 确认依赖树变化；目前 1.1.0 的依赖为 `atoi`、`encoding`、`memchr`、`thiserror`

2. **criterion 0.7 在 Rust 1.94 下是否有 deprecated 警告？**
   - What we know: criterion 0.7 声明 rust-version = "1.80"，Rust 1.94 向后兼容
   - What's unclear: 是否有使用已 deprecated 的 Rust API 导致编译警告
   - Recommendation: 升级完成后运行 `cargo build` 检查是否有来自 criterion 的警告

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | 编译 | ✓ | rustc 1.94.0 | — |
| cargo | 依赖管理 | ✓ | 随 rustc 1.94.0 | — |
| crates.io 网络访问 | `cargo update` / `cargo build` | ✓ | — | 离线缓存 |

**Missing dependencies with no fallback:** none

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust 内置 `#[test]` + criterion 0.7 |
| Config file | `Cargo.toml` [[bench]] 段 |
| Quick run command | `cargo test` |
| Full suite command | `cargo test && cargo clippy --all-targets -- -D warnings` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PARSER-01 | `cargo build --release` 编译成功，无 deprecated 警告 | smoke | `cargo build --release 2>&1 \| grep -c "^warning:"` (期望 0) | ✅ 编译本身即测试 |
| PARSER-01 | `cargo test` 全部通过 | integration | `cargo test` | ✅ `tests/integration.rs` + 各模块测试 |
| REFACTOR-02 | `cargo update` 后 `cargo test` 通过 | smoke | `cargo test` | ✅ |
| REFACTOR-02 | clippy 无警告 | lint | `cargo clippy --all-targets -- -D warnings` | ✅ |

### Sampling Rate

- **Per task commit:** `cargo build && cargo test`
- **Per wave merge:** `cargo build --release && cargo test && cargo clippy --all-targets -- -D warnings`
- **Phase gate:** 全部命令绿色后再 `/gsd:verify-work`

### Wave 0 Gaps

None — 现有测试基础设施完全覆盖本阶段需求（编译 + 测试 + lint）。

---

## Security Domain

> 本阶段仅升级依赖版本，无新引入网络、认证、加密、用户输入处理逻辑，ASVS 类别均不适用。

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | no | 解析库已封装，本阶段不改变输入处理逻辑 |
| V6 Cryptography | no | — |

---

## Sources

### Primary (HIGH confidence)
- [docs.rs/dm-database-parser-sqllog/2.0.0](https://docs.rs/dm-database-parser-sqllog/2.0.0/dm_database_parser_sqllog/) — Sqllog 字段列表、LogParserBuilder 方法、LogIterator API
- [docs.rs/dm-database-parser-sqllog/1.1.0](https://docs.rs/dm-database-parser-sqllog/1.1.0/dm_database_parser_sqllog/) — 1.1.0 Sqllog 字段对比验证
- [github.com/guangl/dm-database-parser-sqllog CHANGELOG.md](https://raw.githubusercontent.com/guangl/dm-database-parser-sqllog/main/CHANGELOG.md) — 2.0.0 无 breaking changes 确认，1.1.0 breaking changes 历史
- `cargo update --dry-run --verbose` — 可升级的 patch/minor 版本列表（本地验证）
- `cargo info criterion@0.8.2` — rust-version = "1.86" 约束（本地验证）

### Secondary (MEDIUM confidence)
- `cargo search dm-database-parser-sqllog` — 版本 2.0.0 在 crates.io 存在的确认

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — docs.rs 直接验证字段和 API，CHANGELOG 明确说明无 breaking changes
- Architecture: HIGH — 全量 grep 扫描确认所有 parser 使用位置，都是相同的调用模式
- Pitfalls: HIGH — cargo update --dry-run 实测确认了 criterion 版本约束

**Research date:** 2026-05-24
**Valid until:** 2026-06-24（crates.io 上的包版本稳定，30 天有效）
