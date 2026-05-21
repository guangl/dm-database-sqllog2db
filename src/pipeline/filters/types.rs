use serde::{Deserialize, Deserializer};

use super::serde_helpers::{TrxidSet, vec_to_hashset, vec_to_i64_hashset};

/// 记录的元数据字段，传递给过滤器评估
#[derive(Debug)]
pub(crate) struct RecordMeta<'a> {
    pub(crate) trxid: &'a str,
    pub(crate) ip: &'a str,
    pub(crate) sess: &'a str,
    pub(crate) thrd: &'a str,
    pub(crate) user: &'a str,
    pub(crate) stmt: &'a str,
    pub(crate) app: &'a str,
    pub(crate) tag: Option<&'a str>,
}

/// 包含过滤器 (include 子表字段)
#[derive(Debug, Deserialize, Clone, Default)]
pub struct IncludeFilters {
    #[serde(default)]
    pub users: Option<Vec<String>>,
    #[serde(default)]
    pub ips: Option<Vec<String>>,
    #[serde(default)]
    pub sessions: Option<Vec<String>>,
    #[serde(default)]
    pub threads: Option<Vec<String>>,
    #[serde(default)]
    pub statements: Option<Vec<String>>,
    #[serde(default)]
    pub apps: Option<Vec<String>>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub start_ts: Option<String>,
    #[serde(default)]
    pub end_ts: Option<String>,
    #[serde(default, deserialize_with = "vec_to_hashset")]
    pub trxids: Option<TrxidSet>,
}

impl IncludeFilters {
    #[must_use]
    pub fn has_filters(&self) -> bool {
        self.users.as_ref().is_some_and(|v| !v.is_empty())
            || self.ips.as_ref().is_some_and(|v| !v.is_empty())
            || self.sessions.as_ref().is_some_and(|v| !v.is_empty())
            || self.threads.as_ref().is_some_and(|v| !v.is_empty())
            || self.statements.as_ref().is_some_and(|v| !v.is_empty())
            || self.apps.as_ref().is_some_and(|v| !v.is_empty())
            || self.tags.as_ref().is_some_and(|v| !v.is_empty())
            || self.start_ts.is_some()
            || self.end_ts.is_some()
            || self.trxids.as_ref().is_some_and(|s| !s.is_empty())
    }
}

/// 排除过滤器 (exclude 子表字段)
#[derive(Debug, Deserialize, Clone, Default)]
pub struct ExcludeFilters {
    #[serde(default)]
    pub users: Option<Vec<String>>,
    #[serde(default)]
    pub ips: Option<Vec<String>>,
    #[serde(default)]
    pub sessions: Option<Vec<String>>,
    #[serde(default)]
    pub threads: Option<Vec<String>>,
    #[serde(default)]
    pub statements: Option<Vec<String>>,
    #[serde(default)]
    pub apps: Option<Vec<String>>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

impl ExcludeFilters {
    #[must_use]
    pub fn has_filters(&self) -> bool {
        self.users.as_ref().is_some_and(|v| !v.is_empty())
            || self.ips.as_ref().is_some_and(|v| !v.is_empty())
            || self.sessions.as_ref().is_some_and(|v| !v.is_empty())
            || self.threads.as_ref().is_some_and(|v| !v.is_empty())
            || self.statements.as_ref().is_some_and(|v| !v.is_empty())
            || self.apps.as_ref().is_some_and(|v| !v.is_empty())
            || self.tags.as_ref().is_some_and(|v| !v.is_empty())
    }
}

/// 过滤器配置（手写 Deserialize，支持新格式嵌套子表和旧格式扁平字段向后兼容）
// pub: required by integration tests (directly constructs FiltersFeature and accesses cfg.filter)
#[derive(Debug, Clone, Default)]
pub struct FiltersFeature {
    /// 是否启用过滤器
    pub enable: bool,
    /// 包含过滤条件子表
    pub include: IncludeFilters,
    /// 排除过滤条件子表
    pub exclude: ExcludeFilters,
    /// 指标过滤器 (事务级: 命中即保留整笔事务 - 需要预扫描)
    pub indicators: IndicatorFilters,
    /// SQL 内容过滤器 (事务级: 预扫描阶段匹配 SQL，保留整笔事务)
    pub sql: SqlFilters,
    /// SQL 记录级过滤器 (记录级: 在主扫描阶段对每条 DML 记录的 SQL 独立判断)
    pub record_sql: SqlFilters,
}

/// 中间反序列化结构体（私有），同时接受新格式子表和旧格式扁平字段
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
    // 旧格式扁平字段（向后兼容）— include 类
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
    // 旧格式扁平字段（向后兼容）— exclude 类
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
        // 检测混合格式：新子表与旧扁平字段同时存在时发出警告
        let flat_include_present = raw.usernames.is_some()
            || raw.client_ips.is_some()
            || raw.sess_ids.is_some()
            || raw.thrd_ids.is_some()
            || raw.statements.is_some()
            || raw.appnames.is_some()
            || raw.tags.is_some()
            || raw.start_ts.is_some()
            || raw.end_ts.is_some()
            || raw.trxids.is_some();

        if raw.include.is_some() && flat_include_present {
            log::warn!(
                "[filter] contains both a `[filter.include]` sub-table \
                 and legacy flat fields (e.g. `usernames`). The sub-table takes priority; \
                 flat fields are ignored. Remove the legacy keys to suppress this warning."
            );
        }

        let flat_exclude_present = raw.exclude_usernames.is_some()
            || raw.exclude_client_ips.is_some()
            || raw.exclude_sess_ids.is_some()
            || raw.exclude_thrd_ids.is_some()
            || raw.exclude_statements.is_some()
            || raw.exclude_appnames.is_some()
            || raw.exclude_tags.is_some();

        if raw.exclude.is_some() && flat_exclude_present {
            log::warn!(
                "[filter] contains both a `[filter.exclude]` sub-table \
                 and legacy flat exclude fields (e.g. `exclude_usernames`). The sub-table takes priority; \
                 flat fields are ignored. Remove the legacy keys to suppress this warning."
            );
        }

        // 新格式优先：有 include 子表则用新格式，否则从旧扁平字段构造
        let include = raw.include.unwrap_or(IncludeFilters {
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
        let exclude = raw.exclude.unwrap_or(ExcludeFilters {
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

/// 指标过滤器 (Transaction-level)
#[derive(Debug, Deserialize, Clone, Default)]
pub struct IndicatorFilters {
    /// 使用 `HashSet<i64>` 代替 `Vec<i64>`，将 `matches()` 热路径中的
    /// `.contains()` 从 O(n) 降为 O(1)。
    #[serde(default, deserialize_with = "vec_to_i64_hashset")]
    pub exec_ids: Option<std::collections::HashSet<i64>>,
    pub min_runtime_ms: Option<u32>,
    pub min_row_count: Option<u32>,
}

/// SQL 过滤器（仅用于事务级预扫描阶段的 `sql` 字段）。
///
/// **注意：这里的 `includes` / `excludes` 使用字面量子串匹配（`str::contains`），
/// 不支持正则表达式。** 请勿在配置中填写正则语法
/// （如 `^SELECT`、`\bDROP\b`），否则会被当作字面字符串查找，导致静默的语义错误。
///
/// 如需正则匹配，请使用记录级过滤器 `record_sql`，它由 `CompiledSqlFilters` 处理，支持正则。
#[derive(Debug, Deserialize, Clone, Default)]
pub struct SqlFilters {
    /// 字面子串包含列表：SQL 必须包含其中之一才会被选中（未配置 = 全部通过）。
    /// 仅支持字面字符串，不支持正则表达式。
    /// 旧字段名 `include_patterns` 通过 alias 向后兼容。
    #[serde(default, alias = "include_patterns")]
    pub includes: Option<Vec<String>>,
    /// 字面子串排除列表：SQL 包含其中任意一个则被过滤掉。
    /// 仅支持字面字符串，不支持正则表达式。
    /// 旧字段名 `exclude_patterns` 通过 alias 向后兼容。
    #[serde(default, alias = "exclude_patterns")]
    pub excludes: Option<Vec<String>>,
}
