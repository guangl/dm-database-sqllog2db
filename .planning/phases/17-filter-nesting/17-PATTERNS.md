# Phase 17: 过滤器配置嵌套化 - Pattern Map

**Mapped:** 2026-05-17
**Files analyzed:** 4 (需修改的文件)
**Analogs found:** 4 / 4

---

## File Classification

| 新增/修改文件 | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/features/filters.rs` | model / config-struct | transform (deserialize → compile) | 自身（重构） | self-refactor |
| `src/config.rs` | config / validation | request-response (validate → compile) | 自身（小改） | self-refactor |
| `src/cli/run.rs` | orchestration | request-response (build pipeline) | 自身（字段路径更新） | self-refactor |
| `src/cli/init.rs` | utility / template | static-string output | 自身（模板替换） | self-refactor |

> 本 Phase 全部是现有文件的内部重构，无新建文件。所有 analog 均为文件自身。

---

## Pattern Assignments

### `src/features/filters.rs` — 主要重构目标

**重构范围：** 新增 `IncludeFilters` / `ExcludeFilters` struct；移除 `MetaFilters` 的 `#[serde(flatten)]`；为 `FiltersFeature` 手写 `Deserialize` impl（通过 `RawFiltersFeature` 中间结构）；`SqlFilters` 字段改名并加 alias；更新所有方法体中的字段路径；更新测试。

---

#### 模式 1：Deserialize helper 函数（保持不变，可直接复用）

**来源：** `src/features/filters.rs` 第 23–39 行

```rust
fn vec_to_hashset<'de, D>(deserializer: D) -> Result<Option<TrxidSet>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<Vec<String>> = Option::deserialize(deserializer)?;
    Ok(v.map(|items| items.into_iter().map(CompactString::from).collect()))
}

fn vec_to_i64_hashset<'de, D>(deserializer: D) -> Result<Option<AHashSet<i64>>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<Vec<i64>> = Option::deserialize(deserializer)?;
    Ok(v.map(|items| items.into_iter().collect()))
}
```

**注意：** `RawFiltersFeature` 中的 `trxids` 字段必须保留 `#[serde(default, deserialize_with = "vec_to_hashset")]`，因为旧格式的 `trxids = [...]` 依赖此 helper 解析。两个 helper 在同一 module 内，`RawFiltersFeature` 可直接引用。

---

#### 模式 2：现有 struct 上的 serde 属性风格（作为新 struct 的模板）

**来源：** `src/features/filters.rs` 第 83–109 行（`IndicatorFilters` 和 `SqlFilters`）

```rust
// IndicatorFilters 展示了 Option 字段 + 自定义 deserialize_with 的标准写法
#[derive(Debug, Deserialize, Clone, Default)]
pub struct IndicatorFilters {
    #[serde(default, deserialize_with = "vec_to_i64_hashset")]
    pub exec_ids: Option<AHashSet<i64>>,
    pub min_runtime_ms: Option<u32>,
    pub min_row_count: Option<u32>,
}

// SqlFilters 展示了独立子表（非 flatten）上 serde alias 的标准写法
// Phase 17 中 include_patterns → includes，exclude_patterns → excludes
#[derive(Debug, Deserialize, Clone, Default)]
pub struct SqlFilters {
    pub include_patterns: Option<Vec<String>>,  // 重构后：includes + alias
    pub exclude_patterns: Option<Vec<String>>,  // 重构后：excludes + alias
}
```

**新的 `SqlFilters` 写法（直接复制此结构，修改字段名并加 alias）：**

```rust
#[derive(Debug, Deserialize, Clone, Default)]
pub struct SqlFilters {
    #[serde(default, alias = "include_patterns")]
    pub includes: Option<Vec<String>>,
    #[serde(default, alias = "exclude_patterns")]
    pub excludes: Option<Vec<String>>,
}
```

**说明：** `SqlFilters` 是独立子表（不用 `flatten`），alias 在此可正常工作（无 serde#2341 限制）。

---

#### 模式 3：`RawFiltersFeature` 中间结构 + 手写 Deserialize（核心新增模式）

**来源：** `src/features/filters.rs` 第 42–58 行（现有 `FiltersFeature` 和 `MetaFilters`，作为字段参考）

现有 `FiltersFeature` 的字段定义方式：

```rust
#[derive(Debug, Deserialize, Clone, Default)]
pub struct FiltersFeature {
    pub enable: bool,
    #[serde(flatten)]           // Phase 17 需要移除此 flatten
    pub meta: MetaFilters,
    #[serde(default)]
    pub indicators: IndicatorFilters,
    #[serde(default)]
    pub sql: SqlFilters,
    #[serde(default)]
    pub record_sql: SqlFilters,
}
```

现有 `MetaFilters` 中所有旧字段名（`RawFiltersFeature` 需原样保留这些字段用于向后兼容）：

```rust
// src/features/filters.rs 第 61–81 行
pub struct MetaFilters {
    pub start_ts: Option<String>,
    pub end_ts: Option<String>,
    pub sess_ids: Option<Vec<String>>,
    pub thrd_ids: Option<Vec<String>>,
    pub usernames: Option<Vec<String>>,
    #[serde(default, deserialize_with = "vec_to_hashset")]
    pub trxids: Option<TrxidSet>,
    pub statements: Option<Vec<String>>,
    pub appnames: Option<Vec<String>>,
    pub client_ips: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub exclude_usernames: Option<Vec<String>>,
    pub exclude_client_ips: Option<Vec<String>>,
    pub exclude_sess_ids: Option<Vec<String>>,
    pub exclude_thrd_ids: Option<Vec<String>>,
    pub exclude_statements: Option<Vec<String>>,
    pub exclude_appnames: Option<Vec<String>>,
    pub exclude_tags: Option<Vec<String>>,
}
```

**新增的 `RawFiltersFeature` 应按照 `IndicatorFilters` 的属性风格编写，所有字段加 `#[serde(default)]`，旧字段名保持原样，新格式子表用 `Option<IncludeFilters>` / `Option<ExcludeFilters>`：**

```rust
// 参照 RESEARCH.md 第 127–180 行的完整示例
#[derive(Debug, Deserialize)]
struct RawFiltersFeature {
    #[serde(default)]
    enable: bool,
    // 新格式子表（优先）
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
    // 旧格式扁平字段（向后兼容）— 字段名与旧 MetaFilters 完全一致
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
    #[serde(default, deserialize_with = "vec_to_hashset")]  // 必须保留！
    trxids: Option<TrxidSet>,
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
```

---

#### 模式 4：`has_filters` 方法体（字段路径需更新）

**来源：** `src/features/filters.rs` 第 113–134 行

```rust
impl FiltersFeature {
    #[must_use]
    pub fn has_filters(&self) -> bool {
        if !self.enable {
            return false;
        }
        self.meta.start_ts.is_some()       // 重构后：self.include.start_ts.is_some()
            || self.meta.end_ts.is_some()  // 重构后：self.include.end_ts.is_some()
            || self.meta.has_filters()     // 重构后：self.include.has_filters() || self.exclude.has_filters()
            || self.indicators.has_filters()
            || self.sql.has_filters()
            || self.record_sql.has_filters()
    }
    // ...
}
```

---

#### 模式 5：`merge_found_trxids` 方法（字段路径需更新）

**来源：** `src/features/filters.rs` 第 171–179 行

```rust
pub fn merge_found_trxids(&mut self, trxids: Vec<CompactString>) {
    if !self.enable || trxids.is_empty() {
        return;
    }
    self.meta          // 重构后：self.include
        .trxids
        .get_or_insert_with(TrxidSet::default)
        .extend(trxids);
}
```

---

#### 模式 6：`CompiledMetaFilters::try_from_meta` → 改名为 `try_from_include_exclude`

**来源：** `src/features/filters.rs` 第 312–359 行

现有签名和 `compile_patterns` 调用风格（字段路径字符串作为错误上下文）：

```rust
pub fn try_from_meta(meta: &MetaFilters) -> crate::error::Result<Self> {
    Ok(Self {
        usernames: compile_patterns("features.filters.usernames", meta.usernames.as_deref())?,
        client_ips: compile_patterns("features.filters.client_ips", meta.client_ips.as_deref())?,
        // ... 其余字段
    })
}
```

重构后签名（保持 `compile_patterns` 调用风格不变，更新字段路径字符串和入参）：

```rust
pub fn try_from_include_exclude(
    include: &IncludeFilters,
    exclude: &ExcludeFilters,
) -> crate::error::Result<Self> {
    Ok(Self {
        usernames: compile_patterns(
            "features.filters.include.users",  // 路径字符串随字段改名更新
            include.users.as_deref(),
        )?,
        // ...
        trxids: include.trxids.clone(),        // 原来是 meta.trxids.clone()
        exclude_usernames: compile_patterns(
            "features.filters.exclude.users",
            exclude.users.as_deref(),
        )?,
        // ...
    })
}
```

---

#### 模式 7：`SqlFilters::has_filters` 和 `matches` 方法体（字段名需更新）

**来源：** `src/features/filters.rs` 第 568–610 行

```rust
impl SqlFilters {
    #[must_use]
    pub fn has_filters(&self) -> bool {
        self.include_patterns      // 重构后：self.includes
            .as_ref()
            .is_some_and(|v| !v.is_empty())
            || self
                .exclude_patterns  // 重构后：self.excludes
                .as_ref()
                .is_some_and(|v| !v.is_empty())
    }

    #[must_use]
    pub fn matches(&self, sql: &str) -> bool {
        // ...
        if let Some(patterns) = &self.include_patterns { // 重构后：self.includes
            // ...
        }
        if let Some(patterns) = &self.exclude_patterns { // 重构后：self.excludes
            // ...
        }
        true
    }
}
```

---

#### 模式 8：`CompiledSqlFilters::try_from_sql_filters`（字段名需更新）

**来源：** `src/features/filters.rs` 第 491–504 行

```rust
pub fn try_from_sql_filters(sf: &SqlFilters) -> crate::error::Result<Self> {
    Ok(Self {
        include_patterns: compile_patterns(
            "features.filters.record_sql.include_patterns",  // 路径字符串随字段改名更新
            sf.include_patterns.as_deref(),                  // 重构后：sf.includes.as_deref()
        )?,
        exclude_patterns: compile_patterns(
            "features.filters.record_sql.exclude_patterns",  // 路径字符串随字段改名更新
            sf.exclude_patterns.as_deref(),                  // 重构后：sf.excludes.as_deref()
        )?,
    })
}
```

---

#### 模式 9：测试中的 `make_feature` 工厂函数（需更新字段初始化）

**来源：** `src/features/filters.rs` 第 617–625 行

```rust
fn make_feature(enable: bool) -> FiltersFeature {
    FiltersFeature {
        enable,
        meta: MetaFilters::default(),      // 重构后：include: IncludeFilters::default(),
                                           //          exclude: ExcludeFilters::default(),
        indicators: IndicatorFilters::default(),
        sql: SqlFilters::default(),
        record_sql: SqlFilters::default(),
    }
}
```

所有直接访问 `f.meta.xxx` 的测试行均需更新为 `f.include.xxx` 或 `f.exclude.xxx`（详见 RESEARCH.md Wave 0 Gaps 章节）。

---

### `src/config.rs` — 小改：更新两处调用点

**重构范围：** `validate()` 和 `validate_and_compile()` 中的 `try_from_meta` 调用改为 `try_from_include_exclude`。

**来源（当前代码）：** `src/config.rs` 第 58–65 行 和 第 130–135 行

`validate()` 中（第 58–65 行）：

```rust
if let Some(filters) = &self.features.filters {
    if filters.enable {
        crate::features::filters::CompiledMetaFilters::try_from_meta(&filters.meta)?;
        // 重构后：
        // crate::features::filters::CompiledMetaFilters::try_from_include_exclude(
        //     &filters.include,
        //     &filters.exclude,
        // )?;
        crate::features::filters::CompiledSqlFilters::try_from_sql_filters(
            &filters.record_sql,
        )?;
        // record_sql 字段名不变，保持不变
    }
}
```

`validate_and_compile()` 中（第 130–135 行）：

```rust
if filters.enable {
    let meta = crate::features::CompiledMetaFilters::try_from_meta(&filters.meta)?;
    // 重构后：
    // let meta = crate::features::CompiledMetaFilters::try_from_include_exclude(
    //     &filters.include,
    //     &filters.exclude,
    // )?;
    let sql =
        crate::features::CompiledSqlFilters::try_from_sql_filters(&filters.record_sql)?;
    Some((meta, sql))
}
```

**serde alias 在 config.rs 中已有成功先例** — 参见 `SqllogConfig.path` 字段（第 383–388 行）：

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct SqllogConfig {
    #[serde(alias = "directory")]
    pub path: String,
}
```

这证明项目已使用 `#[serde(alias)]` 处理字段重命名向后兼容，且该 struct 不使用 `flatten`，与 `SqlFilters` 的情形完全相同（alias 在非 flatten struct 上正常工作）。

---

### `src/cli/run.rs` — 小改：两处字段路径更新

**重构范围：** `FilterProcessor::new` 中 `filter.meta.start_ts` / `filter.meta.end_ts` 改为 `filter.include.start_ts` / `filter.include.end_ts`；`recompile_meta_if_needed` 中 `try_from_meta(&filters.meta)` 改为 `try_from_include_exclude(&filters.include, &filters.exclude)`。

**来源（当前代码）：** `src/cli/run.rs` 第 54–62 行

```rust
fn new(compiled_meta: CompiledMetaFilters, filter: &crate::features::FiltersFeature) -> Self {
    let has_meta_filters = compiled_meta.has_any_filters();
    Self {
        compiled_meta,
        start_ts: filter.meta.start_ts.clone(),  // 重构后：filter.include.start_ts.clone()
        end_ts: filter.meta.end_ts.clone(),       // 重构后：filter.include.end_ts.clone()
        has_meta_filters,
    }
}
```

**来源：** `src/cli/run.rs` 第 669–679 行

```rust
fn recompile_meta_if_needed(
    final_cfg: &Config,
    original: Option<CompiledMetaFilters>,
) -> Result<Option<CompiledMetaFilters>> {
    let filters = match &final_cfg.features.filters {
        Some(f) if f.enable => f,
        _ => return Ok(original),
    };
    let recompiled = crate::features::CompiledMetaFilters::try_from_meta(&filters.meta)?;
    // 重构后：
    // let recompiled = crate::features::CompiledMetaFilters::try_from_include_exclude(
    //     &filters.include,
    //     &filters.exclude,
    // )?;
    Ok(Some(recompiled))
}
```

**热路径逻辑（第 83–107 行）不需要修改** — `FilterProcessor::process_with_meta` 只访问 `self.start_ts` / `self.end_ts`（已拷贝的 `String`），不直接引用 `FiltersFeature` 字段。

---

### `src/cli/init.rs` — 模板字符串替换

**重构范围：** `CONFIG_TEMPLATE_ZH` 和 `CONFIG_TEMPLATE_EN` 中 `[features.filters]` 区块替换为新嵌套格式示例。

**来源（当前模板结构）：** `src/cli/init.rs` 第 90–151 行（ZH）和 第 200–261 行（EN）

现有模板结构模式：

```toml
# 现有（旧格式，需要替换的区块）
[features.filters]
enable = false
# trxids = ["257809109", "257809110"]
# client_ips = ["127.0.0.1", "192\\.168"]
# exclude_client_ips = ["^10\\.0", "^172\\.16"]
# ... 其余扁平字段注释 ...

[features.filters.indicators]
# exec_ids = [...]
# min_runtime_ms = 1000

[features.filters.sql]
# include_patterns = ["FROM USER_TABLES"]
# exclude_patterns = ["SELECT 1", "DUAL"]
```

目标格式（参考 CONTEXT.md `<specifics>` 章节）：

```toml
# 新格式（Phase 17 完成后 init 应生成此结构）
[features.filters]
enable = false

[features.filters.include]
# users = ["SYSDBA"]
# ips = ["127.0.0.1", "192\\.168"]
# sessions = ["0x7f41435437a8"]
# threads = ["2188515"]
# statements = ["INS", "UPD", "DEL"]
# apps = ["DMSQL"]
# tags = ["\\[SEL\\]"]
# start_ts = "2023-01-01 00:00:00"
# end_ts   = "2023-01-01 23:59:59"

[features.filters.exclude]
# users = ["guest", "^anon"]
# ips = ["^10\\.0", "^172\\.16"]

[features.filters.indicators]
# exec_ids = [257809109, 257809110]
# min_runtime_ms = 1000
# min_row_count = 100

[features.filters.sql]
# includes = ["FROM USER_TABLES", "DELETE FROM"]
# excludes = ["SELECT 1", "DUAL"]
```

**注意：** `record_sql` 子表（正则匹配）不在 `init` 模板中默认生成注释（参照现有模板，`record_sql` 区块未出现在当前模板中）。

---

## Shared Patterns（跨文件共用）

### 错误处理：`crate::error::Error::Config(ConfigError::InvalidValue { ... })`

**来源：** `src/features/filters.rs` 第 268–272 行（`compile_patterns` 内部），`src/config.rs` 第 67–75 行

```rust
// compile_patterns 中的错误格式（字段路径字符串是错误消息的核心，重构后需更新路径）
Regex::new(p).map_err(|e| {
    crate::error::Error::Config(crate::error::ConfigError::InvalidValue {
        field: field.to_string(),   // "features.filters.include.users" 等新路径
        value: p.clone(),
        reason: format!("invalid regex: {e}"),
    })
})
```

**适用于：** `src/features/filters.rs` 中所有 `compile_patterns` 调用点（需更新 field 字符串）。

### `#[serde(default)]` 属性约定

**来源：** `src/features/filters.rs`（`IndicatorFilters`、`SqlFilters`）；`src/config.rs`（`Config` struct 各字段）

所有 Option 类型字段均加 `#[serde(default)]`，确保 TOML 中缺省时不报错（与 None 语义一致）。新增的 `IncludeFilters` / `ExcludeFilters` / `RawFiltersFeature` 的所有字段必须遵循此约定。

### `is_some_and(|v| !v.is_empty())` 检查模式

**来源：** `src/features/filters.rs` 第 183–217 行（`MetaFilters::has_filters`）

```rust
// Option<Vec<T>> 的非空判断约定（整个文件一致使用）
self.client_ips.as_ref().is_some_and(|v| !v.is_empty())
```

新的 `IncludeFilters::has_filters` 和 `ExcludeFilters::has_filters` 方法应复制此模式。

---

## 关键字段路径映射（重构前 → 重构后）

| 重构前路径 | 重构后路径 | 出现位置 |
|---|---|---|
| `filter.meta.start_ts` | `filter.include.start_ts` | `run.rs:FilterProcessor::new` (第 58 行) |
| `filter.meta.end_ts` | `filter.include.end_ts` | `run.rs:FilterProcessor::new` (第 59 行) |
| `filters.meta` (as arg) | `filters.include`, `filters.exclude` (两个 arg) | `config.rs` validate/validate_and_compile (第 60, 132 行) |
| `&filters.meta` (recompile) | `&filters.include, &filters.exclude` | `run.rs:recompile_meta_if_needed` (第 678 行) |
| `self.meta.trxids` | `self.include.trxids` | `filters.rs:merge_found_trxids` (第 175 行) |
| `self.meta.start_ts` | `self.include.start_ts` | `filters.rs:has_filters` (第 118 行) |
| `self.meta.end_ts` | `self.include.end_ts` | `filters.rs:has_filters` (第 119 行) |
| `self.meta.has_filters()` | `self.include.has_filters() \|\| self.exclude.has_filters()` | `filters.rs:has_filters` (第 120 行) |
| `sf.include_patterns` | `sf.includes` | `filters.rs:CompiledSqlFilters::try_from_sql_filters` (第 496 行) |
| `sf.exclude_patterns` | `sf.excludes` | `filters.rs:CompiledSqlFilters::try_from_sql_filters` (第 500 行) |
| `self.include_patterns` | `self.includes` | `filters.rs:SqlFilters::has_filters/matches` (第 571, 587 行) |
| `self.exclude_patterns` | `self.excludes` | `filters.rs:SqlFilters::has_filters/matches` (第 574, 601 行) |
| `f.meta.trxids` (tests) | `f.include.trxids` | `filters.rs` tests 第 775, 784 行 |
| `f.meta.usernames` (tests) | `f.include.users` | `filters.rs` tests 第 631, 632, 737 等行 |

---

## No Analog Found

无。本 Phase 所有文件均为自身重构，pattern 均来自文件自身。

---

## Metadata

**Analog search scope:** `src/features/filters.rs`, `src/config.rs`, `src/cli/run.rs`, `src/cli/init.rs`
**Files scanned:** 4
**Pattern extraction date:** 2026-05-17
