# Phase 18: 模板 & 图表配置嵌套化 - Pattern Map

**Mapped:** 2026-05-17
**Files analyzed:** 4 (修改文件)
**Analogs found:** 4 / 4

---

## File Classification

| 修改文件 | Role | Data Flow | Closest Analog | Match Quality |
|---------|------|-----------|----------------|---------------|
| `src/pipeline/mod.rs` | config / model | transform | `src/pipeline/filters.rs` (FiltersFeature) | exact |
| `src/config/mod.rs` | config / validator | request-response | `src/config/mod.rs` validate_pipeline_filters() | exact |
| `src/cli/run.rs` | orchestration | request-response | `src/cli/run.rs` 同文件 do_template / charts 读取段 | exact |
| `src/cli/init.rs` | config template | — | `src/cli/init.rs` 同文件 CONFIG_TEMPLATE_ZH/EN 常量 | exact |

---

## Pattern Assignments

### `src/pipeline/mod.rs` — TemplateAnalysisConfig / ChartsConfig / PipelineConfig 重构

**Analog:** `src/pipeline/filters.rs` FiltersFeature（当前文件已有模式）+ `src/pipeline/mod.rs` NormalizeConfig

#### 当前 struct 定义（待修改，lines 129-183）

```rust
// 当前：[pipeline.template_analysis]
#[derive(Debug, Deserialize, Clone, Default)]
pub struct TemplateAnalysisConfig {
    #[serde(default)]
    pub enabled: bool,
}

// 当前：[pipeline.charts]
#[derive(Debug, Deserialize, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct ChartsConfig {
    pub output_dir: String,
    #[serde(default = "default_top_n")]
    pub top_n: usize,
    // ... bool 字段
}

// 当前：PipelineConfig 聚合
pub struct PipelineConfig {
    pub filters: Option<FiltersFeature>,
    pub normalize: Option<NormalizeConfig>,
    pub fields: Option<Vec<String>>,
    pub template_analysis: Option<TemplateAnalysisConfig>,
    pub charts: Option<ChartsConfig>,
}
```

#### 迁移目标 struct 模式（仿照 NormalizeConfig，lines 79-99）

```rust
// NormalizeConfig 是最直接的模式：enable 字段名 + serde(default) + Default impl
#[derive(Debug, Deserialize, Clone)]
pub struct NormalizeConfig {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default)]
    pub placeholders: Vec<String>,
}

impl Default for NormalizeConfig {
    fn default() -> Self {
        Self { enable: true, placeholders: Vec::new() }
    }
}
```

#### Phase 18 新 struct 设计（直接照抄 NormalizeConfig 模式）

```rust
// [template] — 字段名从 `enabled` 改为 `enable`，增加 output_* 字段
#[derive(Debug, Deserialize, Clone, Default)]
pub struct TemplateConfig {
    #[serde(default)]
    pub enable: bool,
    /// 模板统计 CSV 输出路径；空字符串 = 不生成
    #[serde(default)]
    pub output_csv_path: String,
    /// 模板统计 SQLite 表名；空字符串 = 不生成
    #[serde(default)]
    pub output_sqlite_table: String,
}

// [charts] — 字段不变，struct 从 PipelineConfig 提升为顶层
// ChartsConfig 字段/Default 实现不变（参见现有 lines 137-170）

// Config 顶层新增字段（参见下方 Config struct 模式）
```

#### PipelineConfig 变化

- 移除 `template_analysis: Option<TemplateAnalysisConfig>`
- 移除 `charts: Option<ChartsConfig>`
- 移除 `normalize: Option<NormalizeConfig>`（`replace_parameters` 别名也迁出）
- Phase 18 完成后 `PipelineConfig` 只保留 `filters` 和 `fields`（或完全废弃，Phase 19 范围）

---

### `src/config/mod.rs` — Config struct 与 validate() 重构

**Analog:** `src/config/mod.rs` 自身（当前 validate_pipeline_charts / validate_pipeline_filters 模式）

#### Config struct 新增顶层字段（仿照现有 exporter/pipeline 字段模式，lines 17-29）

```rust
// 现有 Config struct 模式
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub sqllog: SqllogConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub pipeline: PipelineConfig,
    #[serde(default)]
    pub exporter: ExporterConfig,
    #[serde(default)]
    pub resume: ResumeConfig,
}

// Phase 18 新增字段（完全照抄 #[serde(default)] + Option<T> 模式）：
// pub template: Option<TemplateConfig>,
// pub charts: Option<ChartsConfig>,
// pub replace_parameters: Option<NormalizeConfig>,
// pub filter: Option<FiltersFeature>,
// pub output: Option<OutputConfig>,  // 包含 fields
```

#### validate_pipeline_charts 模式（直接 copy-modify，lines 319-349）

```rust
// 现有跨字段依赖检查模式（Phase 18 需更新引用路径）
fn validate_pipeline_charts(&self) -> Result<()> {
    if let Some(charts) = &self.pipeline.charts {
        let ta_enabled = self
            .pipeline
            .template_analysis
            .as_ref()
            .is_some_and(|ta| ta.enabled);
        if !ta_enabled {
            return Err(Error::Config(ConfigError::InvalidValue {
                field: "pipeline.charts".to_string(),
                value: String::new(),
                reason: "启用 [pipeline.charts] 需要先设置 [pipeline.template_analysis]\nenabled = true".to_string(),
            }));
        }
        // output_dir 非空校验、top_n > 0 校验…
    }
    Ok(())
}

// Phase 18 改为：
// fn validate_charts(&self) -> Result<()> {
//     if let Some(charts) = &self.charts {
//         let ta_enabled = self.template.as_ref().is_some_and(|t| t.enable);
//         if !ta_enabled { return Err(...) }
//         // output_dir / top_n 校验不变
//     }
//     Ok(())
// }
```

#### 旧路径检测模式（新增，参见 D-06）

```rust
// 捕获旧 [features] 表的方案（在 Config struct 中新增私有字段）：
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    // ... 已有字段 ...
    // 旧路径检测：捕获 [features] 表（若用户仍用旧格式）
    #[serde(rename = "features")]
    _features_deprecated: Option<toml::Value>,
}

// validate() 开头新增检测（在所有子校验之前）：
pub fn validate(&self) -> Result<()> {
    if self._features_deprecated.is_some() {
        return Err(Error::Config(ConfigError::InvalidValue {
            field: "[features]".to_string(),
            value: String::new(),
            reason: "配置格式已升级，请迁移以下字段：\n  [features.template_analysis] → [template]\n  [features.charts]            → [charts]\n  [features.replace_parameters] → [replace_parameters]\n  [features.filter.*]          → [filter.*]\n  [features.fields]            → [output.fields]".to_string(),
        }));
    }
    // 原有校验…
    self.logging.validate()?;
    // ...
}
```

#### apply_one 中的新路径模式（仿照现有 pipeline.* 路径，lines 130-283）

```rust
// 现有模式（作为模板）：
"pipeline.filters.enable" => {
    self.pipeline
        .filters
        .get_or_insert_with(Default::default)
        .enable = parse_bool(value)?;
}

// Phase 18 新路径（完全照抄，改字段路径）：
"template.enable" => {
    self.template
        .get_or_insert_with(Default::default)
        .enable = parse_bool(value)?;
}
"template.output_csv_path" => {
    self.template
        .get_or_insert_with(Default::default)
        .output_csv_path = value.to_string();
}
// charts.output_dir / charts.top_n 等同理
```

---

### `src/cli/run.rs` — 热路径读取路径更新

**Analog:** 同文件 lines 752-763（当前 do_template / do_normalize 读取段）

#### 当前热路径模式（lines 752-768）

```rust
// do_normalize 读取路径（当前：pipeline.normalize）
let do_normalize = field_mask.includes_normalized_sql()
    && final_cfg
        .pipeline
        .normalize
        .as_ref()
        .is_none_or(|r| r.enable);

// do_template 读取路径（当前：pipeline.template_analysis.enabled）
let do_template = final_cfg
    .pipeline
    .template_analysis
    .as_ref()
    .is_some_and(|t| t.enabled);

// placeholder_override 读取路径（当前：pipeline.normalize）
let placeholder_override = final_cfg
    .pipeline
    .normalize
    .as_ref()
    .and_then(crate::pipeline::NormalizeConfig::placeholder_override);
```

#### Phase 18 目标读取路径（结构完全相同，仅更换字段路径）

```rust
// do_normalize → 读取 config.replace_parameters（顶层）
let do_normalize = field_mask.includes_normalized_sql()
    && final_cfg
        .replace_parameters
        .as_ref()
        .is_none_or(|r| r.enable);

// do_template → 读取 config.template.enable（字段名从 enabled 改为 enable）
let do_template = final_cfg
    .template
    .as_ref()
    .is_some_and(|t| t.enable);

// placeholder_override → 读取 config.replace_parameters（顶层）
let placeholder_override = final_cfg
    .replace_parameters
    .as_ref()
    .and_then(crate::pipeline::NormalizeConfig::placeholder_override);
```

#### charts 引用更新（当前 lines 813-814 / 920-921）

```rust
// 当前（两处相同）：
if let Some(charts_cfg) = final_cfg.pipeline.charts.as_ref() {
    crate::charts::generate_charts(agg, charts_cfg)?;
}

// Phase 18 目标（直接照抄，改字段路径）：
if let Some(charts_cfg) = final_cfg.charts.as_ref() {
    crate::charts::generate_charts(agg, charts_cfg)?;
}
```

#### write_template_stats 调用更新（当前 line 934）

```rust
// 当前（None = 自动推导伴随路径）：
exporter_manager.write_template_stats(stats, None)?;

// Phase 18 目标（从 config.template 传入显式路径）：
// 需查看 write_template_stats 签名后决定传参方式
// 参考 D-04：output_csv_path / output_sqlite_table 均为空字符串时不生成
let csv_out = final_cfg.template.as_ref()
    .and_then(|t| if t.output_csv_path.is_empty() { None } else { Some(t.output_csv_path.as_str()) });
let sqlite_table = final_cfg.template.as_ref()
    .and_then(|t| if t.output_sqlite_table.is_empty() { None } else { Some(t.output_sqlite_table.as_str()) });
exporter_manager.write_template_stats(stats, csv_out, sqlite_table)?;
```

---

### `src/cli/init.rs` — CONFIG_TEMPLATE_ZH / CONFIG_TEMPLATE_EN 更新

**Analog:** `src/cli/init.rs` 同文件（当前 CONFIG_TEMPLATE_ZH lines 65-162）

#### 当前模板格式（旧格式，lines 79-161 的关键部分）

```toml
[pipeline.normalize]
enable = true

[pipeline.template_analysis]
enabled = false

[pipeline.filters]
enable = false

[pipeline.filters.include]
# users = ["SYSDBA"]
```

#### Phase 18 目标模板格式（直接替换 `[pipeline.*]` 段）

```toml
[replace_parameters]
# 是否在导出结果中写入 normalized_sql 列（默认 true）
enable = true

[template]
# SQL 模板归一化（v1.4 新增顶层配置）
# 启用后对 sql_text 执行注释去除、IN 列表折叠、关键字大写、空白折叠
# 默认 false
enable = false
# output_csv_path = "outputs/templates.csv"   # 不填则不生成
# output_sqlite_table = "sql_templates"        # 不填则不生成

[filter]
# 是否启用过滤器
enable = false

[filter.include]
# users = ["SYSDBA"]
# ips = ["127.0.0.1", "192\\.168"]
# ...（其余注释字段不变，仅路径前缀从 pipeline.filters 改为 filter）

[filter.exclude]
# users = ["guest", "^anon"]

[filter.indicators]
# min_runtime_ms = 1000

[filter.sql]
# includes = ["FROM USER_TABLES"]

[charts]
# output_dir = "charts/"
# top_n = 10
```

**注意：** `handle_init` 函数体（lines 1-61）无需改动，只需更新两个字符串常量。

---

## Shared Patterns

### `#[serde(default)]` + `Option<T>` 字段模式
**Source:** `src/pipeline/mod.rs` lines 173-183 (`PipelineConfig`)，`src/config/mod.rs` lines 17-29 (`Config`)
**Apply to:** `Config` struct 中所有新增的顶层字段 (`template`, `charts`, `replace_parameters`, `filter`, `output`)

```rust
// 固定模式：顶层 Option<T> + serde(default)
#[serde(default)]
pub template: Option<TemplateConfig>,
#[serde(default)]
pub charts: Option<ChartsConfig>,
```

### `get_or_insert_with(Default::default)` 链式写入
**Source:** `src/config/mod.rs` lines 204-215 (`apply_one`)
**Apply to:** `apply_one()` 中所有新 `template.*` / `charts.*` / `replace_parameters.*` / `filter.*` 路径

```rust
// 固定模式：新路径 key 匹配 + get_or_insert + 字段赋值
"template.enable" => {
    self.template
        .get_or_insert_with(Default::default)
        .enable = parse_bool(value)?;
}
```

### `validate_pipeline_*` 私有方法模式
**Source:** `src/config/mod.rs` lines 286-349 (validate_pipeline_filters / validate_pipeline_charts)
**Apply to:** Phase 18 新增的 `validate_template()` / `validate_charts()`

```rust
// 固定模式：私有方法，返回 Result<()>，if let Some 解构后校验
fn validate_charts(&self) -> Result<()> {
    if let Some(charts) = &self.charts {
        // 跨字段依赖：charts 依赖 template.enable = true
        // output_dir 非空校验
        // top_n > 0 校验
    }
    Ok(())
}
```

### ConfigError::InvalidValue 错误构造
**Source:** `src/error.rs` lines 54-59，`src/config/mod.rs` lines 322-330
**Apply to:** 旧路径检测错误消息，以及所有 validate() 错误

```rust
// 固定模式
return Err(Error::Config(ConfigError::InvalidValue {
    field: "field.path".to_string(),
    value: String::new(),
    reason: "具体说明".to_string(),
}));
```

---

## No Analog Found

无。所有待修改文件在当前代码库中均有直接的同文件或跨文件 analog。

---

## Metadata

**Analog search scope:** `src/pipeline/`, `src/config/`, `src/cli/`, `src/error.rs`
**Files scanned:** 6 (`pipeline/mod.rs`, `pipeline/filters.rs`, `config/mod.rs`, `cli/run.rs`, `cli/init.rs`, `error.rs`)
**Pattern extraction date:** 2026-05-17

---

## 关键约束备忘（给 Planner）

1. **`TemplateAnalysisConfig` 字段名变更**：`enabled` → `enable`（对齐 NormalizeConfig/FiltersFeature）；同时 struct 重命名为 `TemplateConfig`，路径从 `pipeline.template_analysis` 提升为顶层 `template`。

2. **`FiltersFeature` 路径迁移**：`pipeline.filters` → `filter`（顶层）。Phase 17 的手写 `Deserialize` 实现不变，仅 `PipelineConfig` 中移除该字段，`Config` 顶层新增 `pub filter: Option<FiltersFeature>`。

3. **`NormalizeConfig` 路径迁移**：`pipeline.normalize`（alias `replace_parameters`）→ 顶层 `replace_parameters`。

4. **`ChartsConfig` 路径迁移**：`pipeline.charts` → 顶层 `charts`。struct 字段/Default 不变。

5. **`output.fields` 字段**：原 `pipeline.fields` → `output.fields`，需新增 `OutputConfig { fields: Option<Vec<String>> }` struct，并将 `PipelineConfig::field_mask()` / `ordered_field_indices()` 方法迁移或 delegate。

6. **破坏性升级无 alias 兼容**：`validate()` 开头通过 `_features_deprecated: Option<toml::Value>` 捕获旧 `[features]` 并返回迁移错误，不做 serde alias 透传。

7. **`write_template_stats` 签名可能需要变更**：当前签名 `(stats, None)` 的第二参数含义需要确认，Phase 18 要求显式传入 `output_csv_path` 和 `output_sqlite_table`（两个独立参数或新的结构体）。
