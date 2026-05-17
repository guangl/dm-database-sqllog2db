# Phase 17: 过滤器配置嵌套化 - Research

**Researched:** 2026-05-17
**Domain:** Rust / serde / toml — 配置结构重构 + 向后兼容反序列化
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** 新格式 include/exclude 子表使用语义化短名：
  - `users`（旧：`usernames`）
  - `ips`（旧：`client_ips`）
  - `sessions`（旧：`sess_ids`）
  - `threads`（旧：`thrd_ids`）
  - `statements`（不变）
  - `apps`（旧：`appnames`）
  - `tags`（不变）
- **D-02:** `start_ts`, `end_ts`, `trxids` 放入 `[features.filter.include]`
- **D-03:** sql/record_sql 子表内字段名：`include_patterns` → `includes`，`exclude_patterns` → `excludes`
- **D-04:** 全部嵌套化（include / exclude / indicators / sql / record_sql 均成为子表），`[features.filter]` 层只保留 `enable`
- **D-05:** Phase 17 不涉及 Phase 18（template/charts）或 Phase 19（代码结构拆分）
- **D-06:** `cargo run -- init` 生成新嵌套格式

### Claude's Discretion

- **向后兼容实现方式**：serde alias vs 手写 `Deserialize` impl。由规划/实现阶段根据 toml crate 对 flatten+alias 的实际支持情况决定；若 flatten+alias 有限制，可改用自定义 Visitor 或中间 raw 结构体。
- **`indicators` / `sql` / `record_sql` 旧格式兼容**：这些字段目前已作为子表出现（无 flatten），旧格式 key 名不变；主要工作量在 meta 字段兼容。

### Deferred Ideas (OUT OF SCOPE)

- `[template]` / `[charts]` 配置嵌套化 — Phase 18
- 代码结构拆分 — Phase 19
- 调研 dm-database-parser-sqllog 1.0.0 新特性 — 已关闭

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CONFIG-01 | 用户可在 `[filter.include]` 嵌套子表中配置所有包含过滤条件 | 新增 `IncludeFilters` struct，作为 `FiltersFeature.include` 显式字段 |
| CONFIG-02 | 用户可在 `[filter.exclude]` 嵌套子表中配置所有排除过滤条件 | 新增 `ExcludeFilters` struct，作为 `FiltersFeature.exclude` 显式字段 |
| CONFIG-05 | 旧版扁平格式仍可被正确解析 | 通过中间 raw struct + 手写 `Deserialize` impl 实现（见架构模式章节） |

</phase_requirements>

---

## Summary

Phase 17 的核心工作是将 `FiltersFeature` 中原本通过 `#[serde(flatten)]` 合并的扁平 `MetaFilters` 字段，重组为显式的 `include` / `exclude` / `indicators` / `sql` / `record_sql` 五个子表。目标是让新格式用 TOML 子表表达更清晰，同时保证旧版扁平字段（`usernames`, `exclude_usernames`, `client_ips` 等）无需修改即可继续 parse。

**关键技术发现：serde 的 `flatten` + `alias` 组合在当前已知稳定版本中存在功能限制（多个 open/closed issue，包括 serde#1504、serde#2341、serde#1976）。** 虽然 PR #2387 声称修复了 #1504，但 #2341 与 #1976 的状态表明实际场景中 alias 在 flatten 场景下仍不可靠，尤其是 `toml` crate（使用 `serde` 的 `MapAccess` 实现，非 `serde_json`）。项目当前 `toml = "1.1.2"` + `serde = "1.0.228"`。

**主要推荐方案**：使用"中间 raw 结构体 + 手写 `Deserialize` impl"实现向后兼容，而非依赖 `flatten + alias`（可靠性存疑，且迁移后 flatten 本身需要移除）。该方案在 serde 社区有成熟模式，完全可控。

**Primary recommendation:** 为 `FiltersFeature` 手写 `Deserialize` impl，内部反序列化到 `RawFiltersFeature`（同时接受新格式子表和旧格式扁平字段），再转换为最终 struct。

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| TOML 配置解析（新/旧格式） | `src/features/filters.rs` — FiltersFeature Deserialize | `src/config.rs` — Config::from_file | filters.rs 拥有字段语义，deserialize 逻辑应在 struct 定义处 |
| 配置验证（regex 编译） | `src/config.rs` — validate_and_compile | — | 跨 struct 的语义校验属于 config 层 |
| 热路径过滤执行 | `src/cli/run.rs` — FilterProcessor | `src/features/filters.rs` — CompiledMetaFilters | run.rs 调度，filters.rs 执行 |
| init 模板生成 | `src/cli/init.rs` — CONFIG_TEMPLATE_ZH/EN | — | 静态字符串常量，直接替换 |
| 字段路径错误消息 | `src/features/filters.rs` — compile_patterns | — | 错误消息中的字段路径需随重构更新 |

---

## Architecture Patterns

### System Architecture Diagram

```
TOML 配置文件
    ↓ toml::from_str
FiltersFeature::deserialize (手写 impl)
    ↓ 反序列化到 RawFiltersFeature（接受新旧两种格式）
    ↓ From<RawFiltersFeature> for FiltersFeature
FiltersFeature {
    enable: bool,
    include: IncludeFilters,   ← 新结构（含 users/ips/sessions/threads/statements/apps/tags/start_ts/end_ts/trxids）
    exclude: ExcludeFilters,   ← 新结构（含 users/ips/sessions/threads/statements/apps/tags）
    indicators: IndicatorFilters,
    sql: SqlFilters,
    record_sql: SqlFilters,
}
    ↓
Config::validate_and_compile
    ↓ CompiledMetaFilters::try_from_include_exclude(&filters.include, &filters.exclude)
    ↓ CompiledSqlFilters::try_from_sql_filters(&filters.record_sql)
    ↓ build_pipeline (run.rs) → FilterProcessor
    ↓ 热路径：pipeline.is_empty() 快速退出 / run_with_meta
```

### Recommended Project Structure

新增/修改的 struct 均在 `src/features/filters.rs`，其余文件只需更新字段引用路径：

```
src/
├── features/
│   └── filters.rs     ← 主要变更：新 struct + 手写 Deserialize
├── config.rs           ← 更新字段路径（meta → include/exclude），SqlFilters 字段名
└── cli/
    └── init.rs         ← 替换 CONFIG_TEMPLATE_ZH/EN 中 filter 区块
```

### 中间 raw struct 模式（向后兼容）

**What:** 定义一个 `RawFiltersFeature` 接受旧版所有扁平字段 + 新版子表字段，再在 `impl From<RawFiltersFeature>` 中合并到新结构体。

**When to use:** 当需要同时接受新旧两种格式，且新旧格式存在 key 名映射时。

**Example（[ASSUMED] 基于项目已有 serde 模式推导）:**
```rust
// 仅用于反序列化的中间结构（不对外暴露）
#[derive(Debug, Deserialize)]
struct RawFiltersFeature {
    #[serde(default)]
    enable: bool,

    // 新格式子表
    #[serde(default)]
    include: Option<IncludeFilters>,
    #[serde(default)]
    exclude: Option<ExcludeFilters>,
    #[serde(default)]
    indicators: IndicatorFilters,
    #[serde(default)]
    sql: SqlFilters,
    #[serde(default)]
    record_sql: SqlFilters,

    // 旧格式扁平字段（向后兼容）
    // include 类
    #[serde(default)]
    usernames: Option<Vec<String>>,
    #[serde(default)]
    client_ips: Option<Vec<String>>,
    #[serde(default)]
    sess_ids: Option<Vec<String>>,
    #[serde(default)]
    thrd_ids: Option<Vec<String>>,
    #[serde(default)]
    statements: Option<Vec<String>>,
    #[serde(default)]
    appnames: Option<Vec<String>>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    start_ts: Option<String>,
    #[serde(default)]
    end_ts: Option<String>,
    #[serde(default, deserialize_with = "vec_to_hashset")]
    trxids: Option<TrxidSet>,
    // exclude 类
    #[serde(default)]
    exclude_usernames: Option<Vec<String>>,
    #[serde(default)]
    exclude_client_ips: Option<Vec<String>>,
    #[serde(default)]
    exclude_sess_ids: Option<Vec<String>>,
    #[serde(default)]
    exclude_thrd_ids: Option<Vec<String>>,
    #[serde(default)]
    exclude_statements: Option<Vec<String>>,
    #[serde(default)]
    exclude_appnames: Option<Vec<String>>,
    #[serde(default)]
    exclude_tags: Option<Vec<String>>,
}

impl<'de> Deserialize<'de> for FiltersFeature {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = RawFiltersFeature::deserialize(d)?;
        Ok(FiltersFeature::from(raw))
    }
}

impl From<RawFiltersFeature> for FiltersFeature {
    fn from(raw: RawFiltersFeature) -> Self {
        // 新格式优先；有 include 子表则用新格式，否则从旧扁平字段构造
        let include = raw.include.unwrap_or_else(|| IncludeFilters {
            users: raw.usernames,
            ips: raw.client_ips,
            sessions: raw.sess_ids,
            threads: raw.thrd_ids,
            statements: raw.statements,
            apps: raw.appnames,
            tags: raw.tags,
            start_ts: raw.start_ts,
            end_ts: raw.end_ts,
            trxids: raw.trxids,
        });
        let exclude = raw.exclude.unwrap_or_else(|| ExcludeFilters {
            users: raw.exclude_usernames,
            ips: raw.exclude_client_ips,
            sessions: raw.exclude_sess_ids,
            threads: raw.exclude_thrd_ids,
            statements: raw.exclude_statements,
            apps: raw.exclude_appnames,
            tags: raw.exclude_tags,
        });
        FiltersFeature {
            enable: raw.enable,
            include,
            exclude,
            indicators: raw.indicators,
            sql: raw.sql,
            record_sql: raw.record_sql,
        }
    }
}
```

### SqlFilters 字段名重命名

`SqlFilters` 目前有 `include_patterns`/`exclude_patterns` 字段（决策 D-03 改为 `includes`/`excludes`）。同样需要 alias 保持向后兼容：

```rust
#[derive(Debug, Deserialize, Clone, Default)]
pub struct SqlFilters {
    #[serde(default, alias = "include_patterns")]
    pub includes: Option<Vec<String>>,
    #[serde(default, alias = "exclude_patterns")]
    pub excludes: Option<Vec<String>>,
}
```

**注意：`SqlFilters` 不使用 `flatten`，是独立子表，所以 `alias` 可以正常工作，无 flatten+alias 限制。** [VERIFIED: serde docs — alias 限制只在 flatten 场景下出现]

### FiltersFeature 新结构

```rust
#[derive(Debug, Clone, Default)]
pub struct FiltersFeature {
    pub enable: bool,
    pub include: IncludeFilters,
    pub exclude: ExcludeFilters,
    pub indicators: IndicatorFilters,
    pub sql: SqlFilters,           // 字段名不变，SqlFilters 内部字段改名
    pub record_sql: SqlFilters,    // 字段名不变
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct IncludeFilters {
    pub users: Option<Vec<String>>,
    pub ips: Option<Vec<String>>,
    pub sessions: Option<Vec<String>>,
    pub threads: Option<Vec<String>>,
    pub statements: Option<Vec<String>>,
    pub apps: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub start_ts: Option<String>,
    pub end_ts: Option<String>,
    #[serde(default, deserialize_with = "vec_to_hashset")]
    pub trxids: Option<TrxidSet>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ExcludeFilters {
    pub users: Option<Vec<String>>,
    pub ips: Option<Vec<String>>,
    pub sessions: Option<Vec<String>>,
    pub threads: Option<Vec<String>>,
    pub statements: Option<Vec<String>>,
    pub apps: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}
```

### Anti-Patterns to Avoid

- **直接在 FiltersFeature 新字段上用 `#[serde(alias)]` + 移除 flatten**：serde alias 在非 flatten 字段上理论可行，但当旧格式和新格式共享同一 struct 时，旧扁平字段和新子表字段会混在一起。TOML 解析器会因为找不到顶层 `users` 字段（旧格式在 `[features.filters]` 下有 `usernames`，新字段名是 `users`）而静默忽略或报错。中间 raw struct 方案更健壮。

- **依赖 `#[serde(flatten)]` + `#[serde(alias)]` 组合**：已知 serde flatten 与 alias 存在长期未彻底解决的兼容问题（serde#1504、serde#2341），不应在旧代码维护路径中引入。

- **破坏 `merge_found_trxids` 路径**：该方法写入 `self.meta.trxids`，重构后需更新为 `self.include.trxids`，漏改会导致事务级预扫描结果无法生效（静默错误）。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TOML 向后兼容字段映射 | 手写自定义 Visitor | `RawFiltersFeature` 中间 struct + `From` impl | 中间 struct 可完整利用 derive(Deserialize)，无需实现 Visitor 状态机 |
| SqlFilters 字段别名 | 手写 Deserialize | `#[serde(alias = "include_patterns")]` | SqlFilters 是独立子表，alias 可正常工作 |
| 正则编译 | 手写 regex 缓存 | 现有 `compile_patterns` 函数 | 已有函数可直接复用，不需要改动 |

---

## Common Pitfalls

### Pitfall 1: flatten + alias 不可靠
**What goes wrong:** 在 `FiltersFeature` 上保留 `#[serde(flatten)]` 同时用 `#[serde(alias)]` 支持旧字段，TOML 下 alias 可能被忽略，旧格式 config 静默 parse 为空值。
**Why it happens:** serde 的 flatten 实现将字段收集后二次 deserialize，alias 信息在此过程中丢失（serde#2341 仍为 open/closed without fix）。
**How to avoid:** 移除 flatten，改用手写 `Deserialize` + 中间 raw struct。
**Warning signs:** 旧格式 config 的 `usernames`、`exclude_usernames` 等字段解析后为 `None`，但 parse 不报错。

### Pitfall 2: merge_found_trxids 字段路径遗漏
**What goes wrong:** `merge_found_trxids` 方法引用 `self.meta.trxids`，重构后变为 `self.include.trxids`，若未更新则事务预扫描结果（trxid 集合）不会被合并，所有事务级过滤失效。
**Why it happens:** 代码审阅时容易遗漏 `cli/run.rs` 中通过 `if let Some(f) = &mut tmp.features.filters { f.merge_found_trxids(...) }` 调用的链路。
**How to avoid:** 搜索全代码库 `merge_found_trxids` 和 `filter.meta.` 引用，统一更新到新字段路径。
**Warning signs:** `cargo test` 中 `test_merge_found_trxids_adds_to_set` 等测试直接访问 `.meta.trxids` 会编译失败，能快速暴露。

### Pitfall 3: CompiledMetaFilters::try_from_meta 入参签名变化
**What goes wrong:** `config.rs:validate_and_compile` 调用 `CompiledMetaFilters::try_from_meta(&filters.meta)`，重构后 `MetaFilters` 被拆为 `IncludeFilters` + `ExcludeFilters`，签名需改为接受两个参数（或新增 `try_from_include_exclude`）。
**Why it happens:** `config.rs` 有两处调用（`validate` 和 `validate_and_compile`），需同步更新。
**How to avoid:** 重命名为 `try_from_include_exclude(include: &IncludeFilters, exclude: &ExcludeFilters)`，更新所有调用方。
**Warning signs:** 编译错误 `no field meta`。

### Pitfall 4: FilterProcessor 中 start_ts / end_ts 字段路径
**What goes wrong:** `FilterProcessor::new` 从 `filter.meta.start_ts` / `filter.meta.end_ts` 读取时间范围，重构后需改为 `filter.include.start_ts` / `filter.include.end_ts`。
**Why it happens:** `cli/run.rs:FilterProcessor::new` 直接解构 `FiltersFeature` 字段。
**How to avoid:** 重构时 grep `filter.meta.` 确保全量覆盖。
**Warning signs:** 时间范围过滤器在新格式下不生效（静默，无 panic）。

### Pitfall 5: SqlFilters has_filters / matches 方法字段名更新
**What goes wrong:** `SqlFilters::has_filters` 和 `matches` 引用 `self.include_patterns`/`self.exclude_patterns`，重构为 `includes`/`excludes` 后需同步更新。
**Why it happens:** 字段名重命名后方法体未同步。
**How to avoid:** 编译时会报 `no field include_patterns`，可快速发现。

### Pitfall 6: 旧格式 trxids 反序列化
**What goes wrong:** `trxids` 字段使用自定义 `deserialize_with = "vec_to_hashset"`，在 `RawFiltersFeature` 中必须保留该属性，否则旧格式 `trxids = ["..."]` 会 parse 失败。
**Why it happens:** `vec_to_hashset` 是局部函数，在 raw struct 中引用时需确保在相同 module 可见。
**How to avoid:** `RawFiltersFeature` 定义在同一 `filters.rs` 文件中，可直接引用。

---

## Code Examples

### 新格式配置（目标）[CITED: 17-CONTEXT.md specifics]

```toml
[features.filter]
enable = true

[features.filter.include]
users = ["user1"]
ips = ["192.168.1.1"]
sessions = ["s001"]
threads = ["t001"]
statements = ["SELECT", "INSERT"]
apps = ["myapp"]
tags = ["audit"]
start_ts = "2024-01-01T00:00:00"
end_ts = "2024-12-31T23:59:59"

[features.filter.exclude]
users = ["admin"]
ips = ["10.0.0.1"]

[features.filter.indicators]
min_runtime_ms = 100
min_row_count = 10

[features.filter.sql]
includes = ["SELECT"]
excludes = ["DROP"]

[features.filter.record_sql]
includes = ["^SELECT\\s+\\*"]
excludes = ["^DROP"]
```

### 旧格式配置（向后兼容必须 parse）[CITED: 17-CONTEXT.md specifics]

```toml
[features.filter]
enable = true
usernames = ["user1"]
exclude_usernames = ["admin"]
client_ips = ["192.168.1.1"]
exec_ids = [1, 2]
min_runtime_ms = 100
```

### CompiledMetaFilters 更新后的 try_from 入参 [ASSUMED]

```rust
// 推荐签名（重构后）
impl CompiledMetaFilters {
    pub fn try_from_include_exclude(
        include: &IncludeFilters,
        exclude: &ExcludeFilters,
    ) -> crate::error::Result<Self> {
        Ok(Self {
            usernames: compile_patterns("features.filters.include.users", include.users.as_deref())?,
            client_ips: compile_patterns("features.filters.include.ips", include.ips.as_deref())?,
            // ... 其余 include 字段
            exclude_usernames: compile_patterns("features.filters.exclude.users", exclude.users.as_deref())?,
            // ... 其余 exclude 字段
            trxids: include.trxids.clone(),
        })
    }
}
```

---

## Runtime State Inventory

> 本 Phase 是纯配置 struct 重构，不涉及数据库、OS 注册状态或运行时服务。

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| Stored data | None — 无持久化存储结构涉及过滤器字段名 | none |
| Live service config | None — 无外部服务引用过滤器字段名 | none |
| OS-registered state | None | none |
| Secrets/env vars | None | none |
| Build artifacts | None — 无 egg-info / 已安装包 | none |

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test + cargo test |
| Config file | Cargo.toml（[dev-dependencies] tempfile = "3.27.0"） |
| Quick run command | `cargo test --lib features::filters` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CONFIG-01 | 新格式 `[features.filter.include]` 子表可被 parse | unit | `cargo test --lib features::filters::tests` | ✅（需新增对应测试） |
| CONFIG-02 | 新格式 `[features.filter.exclude]` 子表可被 parse | unit | `cargo test --lib features::filters::tests` | ✅（需新增对应测试） |
| CONFIG-05 | 旧版扁平字段（usernames/exclude_usernames/client_ips/exec_ids 等）parse 结果与旧格式一致 | unit | `cargo test --lib features::filters::tests::test_backward_compat_flat_format` | ❌ Wave 0 |
| CONFIG-05 | `cargo run -- validate` 对旧格式 config.toml 通过验证 | integration | `cargo test --lib config::tests` | ✅（需验证旧格式 toml 测试） |

### Sampling Rate

- **Per task commit:** `cargo clippy --all-targets -- -D warnings && cargo test --lib`
- **Per wave merge:** `cargo test`
- **Phase gate:** `cargo test` 全绿 + `cargo run -- validate -c config.toml` 通过

### Wave 0 Gaps

- [ ] `src/features/filters.rs` — 新增 `test_backward_compat_flat_format`：旧扁平格式 TOML → parse → FiltersFeature，验证字段值映射到 include/exclude 正确
- [ ] `src/features/filters.rs` — 新增 `test_new_nested_format_include`：新格式 include 子表 TOML → parse → IncludeFilters 各字段正确
- [ ] `src/features/filters.rs` — 新增 `test_new_nested_format_exclude`：新格式 exclude 子表 TOML → parse → ExcludeFilters 各字段正确
- [ ] `src/features/filters.rs` — 新增 `test_sql_filters_alias_backward_compat`：旧 `include_patterns` / `exclude_patterns` 字段名仍可 parse
- [ ] 现有测试 `test_filters_toml_deserialization_with_trxids_and_exec_ids`：依赖旧 `filters.meta.trxids` 字段访问，需更新为 `filters.include.trxids`

---

## Security Domain

不适用。本 Phase 为配置结构重构，无新增 I/O、网络、认证或加密操作，无 ASVS 相关变更。

---

## Impact Assessment

### 需要修改的文件清单

| 文件 | 变更类型 | 具体内容 |
|------|----------|----------|
| `src/features/filters.rs` | 主要重构 | 1) 新增 `IncludeFilters`/`ExcludeFilters` struct；2) `FiltersFeature` 去掉 `meta: MetaFilters`，改为 `include`/`exclude`；3) `MetaFilters` 整体移除或重命名为 `RawFiltersFeature`（私有）；4) `SqlFilters` 字段重命名并加 alias；5) `FiltersFeature::has_filters`/`has_transaction_filters`/`merge_found_trxids` 更新字段路径；6) `CompiledMetaFilters::try_from_meta` 更名并改入参；7) 错误消息中的字段路径字符串更新 |
| `src/config.rs` | 小改 | `validate` 和 `validate_and_compile` 中 `try_from_meta(&filters.meta)` 改为 `try_from_include_exclude(&filters.include, &filters.exclude)`；`CompiledSqlFilters::try_from_sql_filters(&filters.record_sql)` 保持不变（字段名不变） |
| `src/cli/run.rs` | 小改 | `FilterProcessor::new` 中 `filter.meta.start_ts` → `filter.include.start_ts`，同理 `end_ts`；`scan_log_file_for_matches` 中 `filters.indicators`/`filters.sql` 字段名不变（子表名未变） |
| `src/cli/init.rs` | 模板替换 | `CONFIG_TEMPLATE_ZH`/`CONFIG_TEMPLATE_EN` 中 filter 区块替换为新嵌套格式；旧扁平注释去掉，改为 include/exclude 子表示例 |
| `src/features/filters.rs` tests | 更新 | 测试中直接访问 `f.meta.usernames` 等字段的全部改为 `f.include.users`；`f.meta.trxids` → `f.include.trxids`；`f.meta.exclude_usernames` → `f.exclude.users` |

### 不需要修改的文件

- `src/exporter/` — 完全不引用 filter struct 字段
- `src/parser.rs` — 不引用 filter struct
- `src/features/mod.rs` — `FeaturesConfig.filters: Option<FiltersFeature>` 字段声明不变；re-export 不变

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `toml = "1.1.2"` + `serde = "1.0.228"` 下 `alias` 在非 flatten 字段（如 `SqlFilters::includes` alias `include_patterns`）可正常工作 | Architecture Patterns | 低风险：alias 在非 flatten struct 是 serde 基础功能，有大量成熟用例 |
| A2 | 中间 raw struct 方案（`RawFiltersFeature`）在 toml crate 下能正确接受新格式子表字段（`include`/`exclude` 为 `Option<IncludeFilters>`）和旧格式扁平字段 | Architecture Patterns | 中风险：TOML 不允许同一 key 出现两次，但新旧格式用的是不同 key，不存在冲突；若出现 "extra field" 错误需增加 `#[serde(deny_unknown_fields)]` 或确保没有多余字段 |
| A3 | `FiltersFeature::has_filters` 的语义（`has_any_filters` 包含 exclude）在重构后保持不变，热路径快速退出逻辑不受影响 | Architecture Patterns | 低风险：语义不变，仅字段路径更改 |

---

## Open Questions (RESOLVED)

1. **新旧格式并存时的优先级**
   - What we know: `RawFiltersFeature` 中新格式子表和旧格式字段同时出现时需要定义合并规则
   - What's unclear: 用户是否可能在同一 config 中同时写 `[features.filters.include]` 和 `usernames = [...]`？
   - RESOLVED: 在 `From<RawFiltersFeature>` 中**新格式优先**——有 `include` 子表则用新格式，旧字段忽略；CONTEXT.md 决策已明确用户需求，旧格式只是向后兼容，不是并存语义

2. **`recompile_meta_if_needed` 中的字段路径**
   - What we know: `run.rs:recompile_meta_if_needed` 调用 `CompiledMetaFilters::try_from_meta(&filters.meta)`
   - What's unclear: 该函数签名更新后入参是两个独立 struct，调用点逻辑需相应修改
   - RESOLVED: 直接更新为 `try_from_include_exclude(&filters.include, &filters.exclude)`，没有额外复杂度

---

## Environment Availability

Step 2.6: SKIPPED（纯 Rust 代码/配置重构，无外部工具依赖，`cargo`/`rustc` 已在项目标准环境中）

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `#[serde(flatten)]` 合并 meta 字段到 FiltersFeature | 显式 include/exclude 子表 + 手写 Deserialize | Phase 17 | 配置层次更清晰；旧格式向后兼容无需用户改动 |
| `include_patterns`/`exclude_patterns` 字段名 | `includes`/`excludes` + alias 向后兼容 | Phase 17 | 语义化字段名，旧格式 config 不需要改动 |

**Deprecated/outdated（Phase 17 完成后）:**
- `MetaFilters` struct：被 `IncludeFilters` + `ExcludeFilters` 替代，代码中不再公开暴露
- `FiltersFeature.meta` 字段：移除，替换为 `FiltersFeature.include` + `FiltersFeature.exclude`

---

## Sources

### Primary (HIGH confidence)
- 代码直读：`src/features/filters.rs`（全文）— 当前 struct 定义、flatten 使用、CompiledMetaFilters 字段
- 代码直读：`src/config.rs`（全文）— validate/validate_and_compile 调用链
- 代码直读：`src/cli/run.rs`（FilterProcessor、recompile_meta_if_needed）— 热路径逻辑
- 代码直读：`src/cli/init.rs` — 模板内容
- 代码直读：`config.toml`（项目根）— 旧格式基准
- `.planning/phases/17-filter-nesting/17-CONTEXT.md` — 用户决策

### Secondary (MEDIUM confidence)
- [serde field-attrs — alias](https://serde.rs/field-attrs.html)：alias 在非 flatten struct 上的标准用法
- [serde attr-flatten](https://serde.rs/attr-flatten.html)：flatten 限制文档

### Tertiary (LOW confidence)
- [serde#1504: Field aliases do not work in combination with flatten](https://github.com/serde-rs/serde/issues/1504) — 确认 flatten+alias 有历史缺陷
- [serde#2341: alias and flatten don't work together](https://github.com/serde-rs/serde/issues/2341) — 近期同类问题
- [serde#1976: alias and flatten don't work (serde_json)](https://github.com/serde-rs/serde/issues/1976) — 跨格式确认

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — 仅用项目已有依赖（toml = 1.1.2, serde = 1.0.228），无新增包
- Architecture: HIGH — 中间 raw struct + From impl 是成熟的 serde 兼容模式，代码库中有足够上下文支撑
- Pitfalls: HIGH — 通过全代码读取识别所有字段引用点，风险已量化

**Research date:** 2026-05-17
**Valid until:** 2026-08-17（serde/toml 版本锁定，无需更新）
