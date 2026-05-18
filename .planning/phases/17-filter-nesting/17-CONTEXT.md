# Phase 17: 过滤器配置嵌套化 - Context

**Gathered:** 2026-05-17
**Status:** Ready for planning

<domain>
## Phase Boundary

将 `[features.filter]` 下的所有过滤条件重组为嵌套子表：
- `[features.filter.include]` — 包含条件（include meta 字段）
- `[features.filter.exclude]` — 排除条件（exclude meta 字段）
- `[features.filter.indicators]` — 指标条件（exec_ids / min_runtime_ms / min_row_count）
- `[features.filter.sql]` — 事务级 SQL 字面匹配
- `[features.filter.record_sql]` — 记录级 SQL 正则匹配

旧版扁平字段配置（usernames / exclude_usernames / client_ips 等）通过 serde alias 向后兼容，无需用户改动。`pipeline.is_empty()` 热路径逻辑不变。

</domain>

<decisions>
## Implementation Decisions

### 字段命名（新格式）

- **D-01:** `[features.filter.include]` 和 `[features.filter.exclude]` 子表使用语义化短名，而非旧有字段名：
  - `users`（旧：`usernames`）
  - `ips`（旧：`client_ips`）
  - `sessions`（旧：`sess_ids`）
  - `threads`（旧：`thrd_ids`）
  - `statements`（不变）
  - `apps`（旧：`appnames`）
  - `tags`（不变）

- **D-02:** `start_ts`, `end_ts`, `trxids` 放入 `[features.filter.include]`。时间范围和事务 ID 属于"包含条件"语义，不在 `[features.filter.exclude]` 中出现。

- **D-03:** `sql` 和 `record_sql` 子表内的字段名语义化：
  - `include_patterns` → `includes`
  - `exclude_patterns` → `excludes`

### Phase 17 范围

- **D-04:** 全部嵌套化 —— include / exclude / indicators / sql / record_sql 均成为 `[features.filter]` 的子表。`[features.filter]` 层只保留 `enable` 字段。

- **D-05:** Phase 17 不涉及 Phase 18 的 `[template]` / `[charts]` 重构，也不涉及 Phase 19 的代码结构拆分。

### init 命令

- **D-06:** `cargo run -- init -o config.toml` 生成**新嵌套格式**，新用户直接得到最新用法。

### Claude's Discretion

- **向后兼容实现方式**：serde alias 方案（`#[serde(alias = "...")]` 加到新字段上）vs 手写 `Deserialize` impl。由规划/实现阶段根据 toml crate 对 flatten+alias 的实际支持情况决定；若 flatten+alias 有限制，可改用自定义 Visitor 或中间 raw 结构体。
- **`indicators` / `sql` / `record_sql` 旧格式兼容**：这些字段目前已作为子表出现（无 flatten），旧格式 key 名不变；主要工作量在 meta 字段兼容。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 过滤器核心结构
- `src/features/filters.rs:43` — `FiltersFeature` struct（当前顶层过滤器配置）
- `src/features/filters.rs:62` — `MetaFilters` struct（当前扁平 include+exclude 字段，Phase 17 主要重构对象）
- `src/features/filters.rs:85` — `IndicatorFilters` struct（exec_ids / min_runtime_ms / min_row_count）
- `src/features/filters.rs:102` — `SqlFilters` struct（include_patterns / exclude_patterns，需改名）

### 配置集成点
- `src/config.rs` — `Config` struct + `validate()` 函数，调用 `CompiledMetaFilters::try_from_meta` / `CompiledSqlFilters::try_from_sql_filters`
- `src/features/mod.rs:176` — `FeaturesConfig.filters: Option<FiltersFeature>` 字段声明

### 热路径（不能破坏）
- `src/cli/run.rs` — `pipeline.is_empty()` 快速退出逻辑；过滤器预扫描（`scan_log_file_for_matches`）和主扫描路径

### 需求规范
- `.planning/REQUIREMENTS.md` — CONFIG-01（include 子表）、CONFIG-02（exclude 子表）、CONFIG-05（旧格式向后兼容）

### 现有配置示例（兼容性验证基准）
- `config.toml` — 项目根目录的现有配置，旧格式；Phase 17 完成后必须仍可正确 parse

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `CompiledMetaFilters::try_from_meta(&filters.meta)` — 编译过滤器的入口；重构后入参结构体变化，但编译逻辑可复用
- `CompiledSqlFilters::try_from_sql_filters(&filters.record_sql)` — 同上，record_sql 字段名改后需更新调用点
- `vec_to_hashset` / `vec_to_i64_hashset` — serde deserialize helpers，可继续复用

### Established Patterns
- `#[serde(default)]` — 所有 sub-struct 字段均用 default，确保 TOML 中缺省时不报错
- `#[serde(flatten)]` — 目前 `MetaFilters` 通过 flatten 合并到 `FiltersFeature`；Phase 17 需**移除** flatten，改为显式 `include`/`exclude` 字段
- `Option<Vec<String>>` — meta 过滤字段全部 Option，None 表示"不过滤该维度"

### Integration Points
- `config.rs:validate()` — 需更新字段路径引用（`filters.meta` 拆分后字段路径变化）
- `cli/run.rs` — 过滤器构建和 pre-scan 逻辑，引用 `filters.indicators` / `filters.sql` / `filters.record_sql`；字段名改变后需同步更新
- `cargo run -- init` 的配置模板生成代码 — 需输出新格式

</code_context>

<specifics>
## Specific Ideas

**新格式示例（目标配置）：**

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

**旧格式（向后兼容，必须继续 parse）：**

```toml
[features.filter]
enable = true
usernames = ["user1"]
exclude_usernames = ["admin"]
client_ips = ["192.168.1.1"]
exec_ids = [1, 2]
min_runtime_ms = 100
```

</specifics>

<deferred>
## Deferred Ideas

- TODO "调研 dm-database-parser-sqllog 1.0.0 新特性" — 已在 Phase 6 关闭（PERF-07），与 Phase 17 无关
- `[template]` / `[charts]` 配置嵌套化 — Phase 18 范围，不在 Phase 17 处理
- 代码结构拆分（filters.rs 超 300 行） — Phase 19 范围

</deferred>

---

*Phase: 17-过滤器配置嵌套化*
*Context gathered: 2026-05-17*
