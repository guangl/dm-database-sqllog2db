use ahash::HashSet as AHashSet;
use compact_str::CompactString;
use regex::Regex;
use serde::{Deserialize, Deserializer};

/// `trxid` 过滤集合类型：使用 `ahash`（non-cryptographic SIMD 哈希），
/// 比标准 `SipHash` 快 2-3×，适合大量短字符串的热路径 `contains` 查询。
type TrxidSet = AHashSet<CompactString>;

/// 记录的元数据字段，传递给过滤器评估
#[derive(Debug)]
pub struct RecordMeta<'a> {
    pub trxid: &'a str,
    pub ip: &'a str,
    pub sess: &'a str,
    pub thrd: &'a str,
    pub user: &'a str,
    pub stmt: &'a str,
    pub app: &'a str,
    pub tag: Option<&'a str>,
}

fn vec_to_hashset<'de, D>(deserializer: D) -> Result<Option<TrxidSet>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<Vec<String>> = Option::deserialize(deserializer)?;
    // CompactString 对 ≤23 字节的字符串（trxid 通常是数字字符串）直接内联存储，
    // 消除堆分配，提升 HashSet 的 cache locality（bucket 内直接包含字符串数据）。
    Ok(v.map(|items| items.into_iter().map(CompactString::from).collect()))
}

fn vec_to_i64_hashset<'de, D>(deserializer: D) -> Result<Option<AHashSet<i64>>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<Vec<i64>> = Option::deserialize(deserializer)?;
    Ok(v.map(|items| items.into_iter().collect()))
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
                "[features.filters] contains both a `[features.filters.include]` sub-table \
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
                "[features.filters] contains both a `[features.filters.exclude]` sub-table \
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
    /// 使用 `AHashSet<i64>` 代替 `Vec<i64>`，将 `matches()` 热路径中的
    /// `.contains()` 从 O(n) 降为 O(1)。
    #[serde(default, deserialize_with = "vec_to_i64_hashset")]
    pub exec_ids: Option<AHashSet<i64>>,
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

impl FiltersFeature {
    /// 检查是否配置了任何过滤器
    #[must_use]
    pub fn has_filters(&self) -> bool {
        if !self.enable {
            return false;
        }
        self.include.has_filters()
            || self.exclude.has_filters()
            || self.indicators.has_filters()
            || self.sql.has_filters()
            || self.record_sql.has_filters()
    }

    /// 检查是否提供了需要预扫描的过滤器 (Transaction-level)
    #[must_use]
    pub fn has_transaction_filters(&self) -> bool {
        // 如果未开启过滤器功能，则不执行预扫描
        if !self.enable {
            return false;
        }
        self.indicators.has_filters() || self.sql.has_filters()
    }

    /// 合并预扫描发现的事务 ID 到 `IncludeFilters` 中，以便在正式扫描时直接通过 trxid 匹配保留整笔事务
    pub fn merge_found_trxids(&mut self, trxids: Vec<CompactString>) {
        if !self.enable || trxids.is_empty() {
            return;
        }
        self.include
            .trxids
            .get_or_insert_with(TrxidSet::default)
            .extend(trxids);
    }
}

/// 将正则字符串列表编译为 `Vec<Regex>`。None 或空列表返回 `Ok(None)`（未配置）。
/// 遇到非法正则时返回 `ConfigError::InvalidValue`，field 参数用于错误消息。
fn compile_patterns(
    field: &str,
    patterns: Option<&[String]>,
) -> crate::error::Result<Option<Vec<Regex>>> {
    match patterns {
        None | Some([]) => Ok(None),
        Some(v) => {
            let compiled = v
                .iter()
                .map(|p| {
                    Regex::new(p).map_err(|e| {
                        crate::error::Error::Config(crate::error::ConfigError::InvalidValue {
                            field: field.to_string(),
                            value: p.clone(),
                            reason: format!("invalid regex: {e}"),
                        })
                    })
                })
                .collect::<crate::error::Result<Vec<_>>>()?;
            Ok(Some(compiled))
        }
    }
}

/// None 表示"未配置，直接通过"；Some(patterns) 表示"任意一个匹配即满足"。
#[inline]
fn match_any_regex(patterns: Option<&[Regex]>, val: &str) -> bool {
    match patterns {
        None | Some([]) => true,
        Some(p) => p.iter().any(|re| re.is_match(val)),
    }
}

/// 预编译后的元数据过滤器，在热路径中使用。由 `MetaFilters` 在启动时构造。
#[derive(Debug)]
pub struct CompiledMetaFilters {
    pub usernames: Option<Vec<Regex>>,
    pub client_ips: Option<Vec<Regex>>,
    pub sess_ids: Option<Vec<Regex>>,
    pub thrd_ids: Option<Vec<Regex>>,
    pub statements: Option<Vec<Regex>>,
    pub appnames: Option<Vec<Regex>>,
    pub tags: Option<Vec<Regex>>,
    pub trxids: Option<TrxidSet>,
    pub exclude_usernames: Option<Vec<Regex>>,
    pub exclude_client_ips: Option<Vec<Regex>>,
    pub exclude_sess_ids: Option<Vec<Regex>>,
    pub exclude_thrd_ids: Option<Vec<Regex>>,
    pub exclude_statements: Option<Vec<Regex>>,
    pub exclude_appnames: Option<Vec<Regex>>,
    pub exclude_tags: Option<Vec<Regex>>,
}

impl CompiledMetaFilters {
    /// 从 `IncludeFilters` 和 `ExcludeFilters` 编译所有正则，
    /// 遇到非法 pattern 返回 `ConfigError::InvalidValue`。
    pub fn try_from_include_exclude(
        include: &IncludeFilters,
        exclude: &ExcludeFilters,
    ) -> crate::error::Result<Self> {
        Ok(Self {
            usernames: compile_patterns(
                "features.filters.include.users",
                include.users.as_deref(),
            )?,
            client_ips: compile_patterns("features.filters.include.ips", include.ips.as_deref())?,
            sess_ids: compile_patterns(
                "features.filters.include.sessions",
                include.sessions.as_deref(),
            )?,
            thrd_ids: compile_patterns(
                "features.filters.include.threads",
                include.threads.as_deref(),
            )?,
            statements: compile_patterns(
                "features.filters.include.statements",
                include.statements.as_deref(),
            )?,
            appnames: compile_patterns("features.filters.include.apps", include.apps.as_deref())?,
            tags: compile_patterns("features.filters.include.tags", include.tags.as_deref())?,
            trxids: include.trxids.clone(),
            exclude_usernames: compile_patterns(
                "features.filters.exclude.users",
                exclude.users.as_deref(),
            )?,
            exclude_client_ips: compile_patterns(
                "features.filters.exclude.ips",
                exclude.ips.as_deref(),
            )?,
            exclude_sess_ids: compile_patterns(
                "features.filters.exclude.sessions",
                exclude.sessions.as_deref(),
            )?,
            exclude_thrd_ids: compile_patterns(
                "features.filters.exclude.threads",
                exclude.threads.as_deref(),
            )?,
            exclude_statements: compile_patterns(
                "features.filters.exclude.statements",
                exclude.statements.as_deref(),
            )?,
            exclude_appnames: compile_patterns(
                "features.filters.exclude.apps",
                exclude.apps.as_deref(),
            )?,
            exclude_tags: compile_patterns(
                "features.filters.exclude.tags",
                exclude.tags.as_deref(),
            )?,
        })
    }

    /// 是否有任何已编译的过滤条件（用于快路径跳过）。
    /// 只检查 include 字段，不含 exclude 字段。
    #[must_use]
    pub fn has_filters(&self) -> bool {
        self.usernames.is_some()
            || self.client_ips.is_some()
            || self.sess_ids.is_some()
            || self.thrd_ids.is_some()
            || self.statements.is_some()
            || self.appnames.is_some()
            || self.tags.is_some()
            || self.trxids.as_ref().is_some_and(|v| !v.is_empty())
    }

    /// 是否有任何过滤条件（include 或 exclude 任一非空）。
    /// 供 `FilterProcessor::new()` 预计算 `has_meta_filters`，
    /// 确保纯 exclude 配置也激活 meta 检查路径。
    #[must_use]
    pub fn has_any_filters(&self) -> bool {
        self.has_filters()
            || self.exclude_usernames.is_some()
            || self.exclude_client_ips.is_some()
            || self.exclude_sess_ids.is_some()
            || self.exclude_thrd_ids.is_some()
            || self.exclude_statements.is_some()
            || self.exclude_appnames.is_some()
            || self.exclude_tags.is_some()
    }

    /// AND 语义：所有已配置的字段都必须匹配记录才被保留（D-04）。
    /// 字段内 OR：同一字段列表中任意一个正则匹配即满足该字段（D-02）。
    /// Exclude OR-veto：任一 exclude 字段命中则直接丢弃，优先于 include 检查（D-04）。
    #[inline]
    #[must_use]
    pub fn should_keep(&self, meta: &RecordMeta) -> bool {
        // === 1. Exclude OR-veto（任一命中 → 丢弃，短路最快）===
        if self.exclude_veto(meta) {
            return false;
        }
        // === 2. Include AND 检查（现有逻辑，不变）===
        self.include_and(meta)
    }

    /// Exclude OR-veto：任一 exclude 字段命中则返回 true（应丢弃）。
    fn exclude_veto(&self, meta: &RecordMeta) -> bool {
        if self.exclude_usernames.is_some()
            && match_any_regex(self.exclude_usernames.as_deref(), meta.user)
        {
            return true;
        }
        if self.exclude_client_ips.is_some()
            && match_any_regex(self.exclude_client_ips.as_deref(), meta.ip)
        {
            return true;
        }
        if self.exclude_sess_ids.is_some()
            && match_any_regex(self.exclude_sess_ids.as_deref(), meta.sess)
        {
            return true;
        }
        if self.exclude_thrd_ids.is_some()
            && match_any_regex(self.exclude_thrd_ids.as_deref(), meta.thrd)
        {
            return true;
        }
        if self.exclude_statements.is_some()
            && match_any_regex(self.exclude_statements.as_deref(), meta.stmt)
        {
            return true;
        }
        if self.exclude_appnames.is_some()
            && match_any_regex(self.exclude_appnames.as_deref(), meta.app)
        {
            return true;
        }
        // exclude_tags：tag 为 Option<&str>，无 tag 值时不触发 exclude（保留该记录）
        if let (Some(excl_tags), Some(t)) = (&self.exclude_tags, meta.tag) {
            if excl_tags.iter().any(|re| re.is_match(t)) {
                return true;
            }
        }
        false
    }

    /// Include AND 检查：所有已配置字段必须匹配才返回 true。
    fn include_and(&self, meta: &RecordMeta) -> bool {
        if !match_any_regex(self.usernames.as_deref(), meta.user) {
            return false;
        }
        if !match_any_regex(self.client_ips.as_deref(), meta.ip) {
            return false;
        }
        if !match_any_regex(self.sess_ids.as_deref(), meta.sess) {
            return false;
        }
        if !match_any_regex(self.thrd_ids.as_deref(), meta.thrd) {
            return false;
        }
        if !match_any_regex(self.statements.as_deref(), meta.stmt) {
            return false;
        }
        if !match_any_regex(self.appnames.as_deref(), meta.app) {
            return false;
        }
        // trxids：精确匹配（不用正则），参与 AND
        if let Some(trxids) = &self.trxids {
            if !trxids.is_empty() && !trxids.contains(meta.trxid) {
                return false;
            }
        }
        // tags：meta.tag 可能为 None，需要特殊处理
        if let Some(tag_patterns) = &self.tags {
            match meta.tag {
                Some(t) if !tag_patterns.iter().any(|re| re.is_match(t)) => return false,
                None if !tag_patterns.is_empty() => return false,
                _ => {}
            }
        }
        true
    }
}

/// 预编译后的 SQL 记录级过滤器（D-03）。
/// 仅用于 `record_sql`，事务级 `sql`（预扫描）保持字符串包含匹配。
#[derive(Debug)]
pub struct CompiledSqlFilters {
    pub include_patterns: Option<Vec<Regex>>,
    pub exclude_patterns: Option<Vec<Regex>>,
}

impl CompiledSqlFilters {
    /// 从 `SqlFilters` 编译正则，遇到非法 pattern 返回 `ConfigError::InvalidValue`。
    pub fn try_from_sql_filters(sf: &SqlFilters) -> crate::error::Result<Self> {
        Ok(Self {
            include_patterns: compile_patterns(
                "features.filters.record_sql.includes",
                sf.includes.as_deref(),
            )?,
            exclude_patterns: compile_patterns(
                "features.filters.record_sql.excludes",
                sf.excludes.as_deref(),
            )?,
        })
    }

    /// 是否有任何已编译的过滤条件。
    #[must_use]
    #[allow(dead_code)]
    pub fn has_filters(&self) -> bool {
        self.include_patterns.is_some() || self.exclude_patterns.is_some()
    }

    /// 判断 SQL 是否通过过滤：
    /// - include：必须命中其中之一（未配置 = 通过）
    /// - exclude：不能命中任何一个
    #[inline]
    #[must_use]
    pub fn matches(&self, sql: &str) -> bool {
        let include_ok = self
            .include_patterns
            .as_deref()
            .is_none_or(|p| p.is_empty() || p.iter().any(|re| re.is_match(sql)));
        if !include_ok {
            return false;
        }
        if let Some(excl) = &self.exclude_patterns {
            if excl.iter().any(|re| re.is_match(sql)) {
                return false;
            }
        }
        true
    }
}

impl IndicatorFilters {
    #[must_use]
    pub fn has_filters(&self) -> bool {
        self.exec_ids.as_ref().is_some_and(|v| !v.is_empty())
            || self.min_runtime_ms.is_some()
            || self.min_row_count.is_some()
    }

    #[must_use]
    pub fn matches(&self, exec_id: i64, runtime_ms: f32, row_count: i64) -> bool {
        if !self.has_filters() {
            return false;
        }

        if let Some(ids) = &self.exec_ids {
            if ids.contains(&exec_id) {
                return true;
            }
        }
        if let Some(min_t) = self.min_runtime_ms {
            if f64::from(runtime_ms) >= f64::from(min_t) {
                return true;
            }
        }
        if let Some(min_r) = self.min_row_count {
            if row_count >= i64::from(min_r) {
                return true;
            }
        }
        false
    }
}

impl SqlFilters {
    #[must_use]
    pub fn has_filters(&self) -> bool {
        self.includes.as_ref().is_some_and(|v| !v.is_empty())
            || self.excludes.as_ref().is_some_and(|v| !v.is_empty())
    }

    #[must_use]
    pub fn matches(&self, sql: &str) -> bool {
        if !self.has_filters() {
            return false;
        }

        // 如果指定了包含模式，必须命中其中之一
        let include_match = if let Some(patterns) = &self.includes {
            if patterns.is_empty() {
                true
            } else {
                patterns.iter().any(|p| sql.contains(p))
            }
        } else {
            true
        };

        if !include_match {
            return false;
        }

        // 如果指定了排除模式，不能命中任何一个
        if let Some(patterns) = &self.excludes {
            if patterns.iter().any(|p| sql.contains(p)) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_feature(enable: bool) -> FiltersFeature {
        FiltersFeature {
            enable,
            include: IncludeFilters::default(),
            exclude: ExcludeFilters::default(),
            indicators: IndicatorFilters::default(),
            sql: SqlFilters::default(),
            record_sql: SqlFilters::default(),
        }
    }

    // ── has_filters ────────────────────────────────────────────
    #[test]
    fn test_has_filters_disabled_returns_false() {
        let mut f = make_feature(false);
        f.include.users = Some(vec!["USER".into()]);
        assert!(!f.has_filters());
    }

    #[test]
    fn test_has_filters_empty() {
        assert!(!make_feature(true).has_filters());
    }

    #[test]
    fn test_has_filters_with_username() {
        let mut f = make_feature(true);
        f.include.users = Some(vec!["USER".into()]);
        assert!(f.has_filters());
    }

    #[test]
    fn test_has_filters_with_start_ts() {
        let mut f = make_feature(true);
        f.include.start_ts = Some("2025-01-01".into());
        assert!(f.has_filters());
    }

    #[test]
    fn test_has_filters_with_indicator() {
        let mut f = make_feature(true);
        f.indicators.min_runtime_ms = Some(1000);
        assert!(f.has_filters());
    }

    // ── has_transaction_filters ────────────────────────────────
    #[test]
    fn test_has_transaction_filters_disabled() {
        let mut f = make_feature(false);
        f.indicators.min_runtime_ms = Some(1000);
        assert!(!f.has_transaction_filters());
    }

    #[test]
    fn test_has_transaction_filters_no_indicators() {
        let mut f = make_feature(true);
        f.include.users = Some(vec!["USER".into()]);
        assert!(!f.has_transaction_filters());
    }

    #[test]
    fn test_has_transaction_filters_with_min_runtime() {
        let mut f = make_feature(true);
        f.indicators.min_runtime_ms = Some(500);
        assert!(f.has_transaction_filters());
    }

    #[test]
    fn test_has_transaction_filters_with_exec_ids() {
        let mut f = make_feature(true);
        f.indicators.exec_ids = Some([1_i64, 2, 3].into_iter().collect());
        assert!(f.has_transaction_filters());
    }

    fn m<'a>(trxid: &'a str, ip: &'a str, user: &'a str, tag: Option<&'a str>) -> RecordMeta<'a> {
        RecordMeta {
            trxid,
            ip,
            sess: "s",
            thrd: "t",
            user,
            stmt: "st",
            app: "a",
            tag,
        }
    }

    // ── merge_found_trxids ─────────────────────────────────────
    #[test]
    fn test_merge_found_trxids_empty_list() {
        let mut f = make_feature(true);
        f.include.users = Some(vec!["USER".into()]);
        f.merge_found_trxids(vec![]);
        assert!(f.include.trxids.is_none());
    }

    #[test]
    fn test_merge_found_trxids_adds_to_set() {
        let mut f = make_feature(true);
        f.include.users = Some(vec!["USER".into()]);
        // "TX1".into() → CompactString (inline, no heap alloc)
        f.merge_found_trxids(vec!["TX1".into(), "TX2".into()]);
        let trxids = f.include.trxids.unwrap();
        // contains(&str) works via CompactString: Borrow<str>
        assert!(trxids.contains("TX1"));
        assert!(trxids.contains("TX2"));
    }

    // ── IndicatorFilters ───────────────────────────────────────
    #[test]
    fn test_indicator_has_filters_empty() {
        assert!(!IndicatorFilters::default().has_filters());
    }

    #[test]
    fn test_indicator_matches_exec_id() {
        let f = IndicatorFilters {
            exec_ids: Some([42_i64].into_iter().collect()),
            min_runtime_ms: None,
            min_row_count: None,
        };
        assert!(f.matches(42, 0.0_f32, 0));
        assert!(!f.matches(99, 0.0_f32, 0));
    }

    #[test]
    fn test_indicator_matches_min_runtime() {
        let f = IndicatorFilters {
            exec_ids: None,
            min_runtime_ms: Some(1000),
            min_row_count: None,
        };
        assert!(f.matches(0, 1000.0_f32, 0));
        assert!(f.matches(0, 2000.0_f32, 0));
        assert!(!f.matches(0, 999.0_f32, 0));
    }

    #[test]
    fn test_indicator_matches_min_row_count() {
        let f = IndicatorFilters {
            exec_ids: None,
            min_runtime_ms: None,
            min_row_count: Some(100),
        };
        assert!(f.matches(0, 0.0_f32, 100));
        assert!(!f.matches(0, 0.0_f32, 99));
    }

    #[test]
    fn test_indicator_no_filters_always_false() {
        assert!(!IndicatorFilters::default().matches(1, 9999.0_f32, 9999));
    }

    // ── SqlFilters ─────────────────────────────────────────────
    #[test]
    fn test_sql_filters_empty() {
        assert!(!SqlFilters::default().has_filters());
        assert!(!SqlFilters::default().matches("SELECT 1"));
    }

    #[test]
    fn test_sql_filters_include_pattern() {
        let f = SqlFilters {
            includes: Some(vec!["SELECT".into()]),
            excludes: None,
        };
        assert!(f.matches("SELECT * FROM t"));
        assert!(!f.matches("INSERT INTO t VALUES (1)"));
    }

    #[test]
    fn test_sql_filters_exclude_pattern() {
        let f = SqlFilters {
            includes: None,
            excludes: Some(vec!["DROP".into()]),
        };
        assert!(f.matches("SELECT * FROM t"));
        assert!(!f.matches("DROP TABLE t"));
    }

    #[test]
    fn test_sql_filters_include_and_exclude() {
        let f = SqlFilters {
            includes: Some(vec!["FROM t".into()]),
            excludes: Some(vec!["WHERE id=0".into()]),
        };
        assert!(f.matches("SELECT * FROM t WHERE id=1"));
        assert!(!f.matches("SELECT * FROM t WHERE id=0"));
        assert!(!f.matches("SELECT * FROM other"));
    }

    #[test]
    fn test_sql_filters_empty_include_patterns_with_exclude() {
        // includes is Some but empty → "true" branch
        // excludes is non-empty so has_filters() returns true
        let f = SqlFilters {
            includes: Some(vec![]),
            excludes: Some(vec!["DROP".into()]),
        };
        // SQL doesn't match exclude → passes
        assert!(f.matches("SELECT 1"));
        // SQL matches exclude → filtered
        assert!(!f.matches("DROP TABLE t"));
    }

    // ── compile_patterns ───────────────────────────────────────
    #[test]
    fn test_compile_patterns_none() {
        let result = compile_patterns("test.field", None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_compile_patterns_empty() {
        let result = compile_patterns("test.field", Some(&[]));
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_compile_patterns_valid() {
        let patterns = vec!["^admin.*".to_string()];
        let result = compile_patterns("test.field", Some(&patterns));
        assert!(result.is_ok());
        let compiled = result.unwrap();
        assert!(compiled.is_some());
        assert_eq!(compiled.unwrap().len(), 1);
    }

    #[test]
    fn test_compile_patterns_invalid() {
        let patterns = vec!["[invalid".to_string()];
        let result = compile_patterns("test.field", Some(&patterns));
        assert!(result.is_err());
    }

    // ── match_any_regex ────────────────────────────────────────
    #[test]
    fn test_match_any_regex_none_passes() {
        assert!(match_any_regex(None, "anything"));
    }

    #[test]
    fn test_match_any_regex_empty_passes() {
        assert!(match_any_regex(Some(&[]), "anything"));
    }

    #[test]
    fn test_match_any_regex_match() {
        use regex::Regex;
        let re = Regex::new("^admin").unwrap();
        assert!(match_any_regex(Some(&[re]), "admin_dba"));
    }

    #[test]
    fn test_match_any_regex_no_match() {
        use regex::Regex;
        let re = Regex::new("^admin").unwrap();
        assert!(!match_any_regex(Some(&[re]), "sys_admin"));
    }

    // ── CompiledMetaFilters ────────────────────────────────────
    fn make_compiled_meta(
        users: Option<Vec<String>>,
        ips: Option<Vec<String>>,
    ) -> CompiledMetaFilters {
        let include = IncludeFilters {
            users,
            ips,
            ..IncludeFilters::default()
        };
        CompiledMetaFilters::try_from_include_exclude(&include, &ExcludeFilters::default()).unwrap()
    }

    #[test]
    fn test_compiled_meta_unconfigured_passes() {
        let compiled = make_compiled_meta(None, None);
        assert!(compiled.should_keep(&m("tx", "1.2.3.4", "any_user", None)));
    }

    #[test]
    fn test_compiled_meta_and_semantics() {
        let compiled = make_compiled_meta(
            Some(vec!["^admin".to_string()]),
            Some(vec!["^192\\.168".to_string()]),
        );
        // 两者都匹配 → 保留
        assert!(compiled.should_keep(&m("tx", "192.168.1.1", "admin_dba", None)));
        // 只有 username 匹配 → 拒绝
        assert!(!compiled.should_keep(&m("tx", "10.0.0.1", "admin_dba", None)));
        // 只有 ip 匹配 → 拒绝
        assert!(!compiled.should_keep(&m("tx", "192.168.1.1", "sys_user", None)));
    }

    #[test]
    fn test_compiled_meta_single_field_or() {
        let include = IncludeFilters {
            users: Some(vec!["^admin".to_string(), ".*_dba$".to_string()]),
            ..IncludeFilters::default()
        };
        let compiled =
            CompiledMetaFilters::try_from_include_exclude(&include, &ExcludeFilters::default())
                .unwrap();
        assert!(compiled.should_keep(&m("tx", "ip", "admin_user", None)));
        assert!(compiled.should_keep(&m("tx", "ip", "sys_dba", None)));
        assert!(!compiled.should_keep(&m("tx", "ip", "regular_user", None)));
    }

    #[test]
    fn test_compiled_meta_tags_none_rejected() {
        let include = IncludeFilters {
            tags: Some(vec!["^SEL".to_string()]),
            ..IncludeFilters::default()
        };
        let compiled =
            CompiledMetaFilters::try_from_include_exclude(&include, &ExcludeFilters::default())
                .unwrap();
        // tag 为 None 时，有 tag 过滤条件，拒绝
        assert!(!compiled.should_keep(&m("tx", "ip", "user", None)));
        // tag 匹配时通过
        assert!(compiled.should_keep(&m("tx", "ip", "user", Some("SELECT"))));
        // tag 不匹配时拒绝
        assert!(!compiled.should_keep(&m("tx", "ip", "user", Some("INSERT"))));
    }

    #[test]
    fn test_compiled_meta_trxids_and() {
        use compact_str::CompactString;
        let mut trxid_set = TrxidSet::default();
        trxid_set.insert(CompactString::from("TX123"));
        let include = IncludeFilters {
            users: Some(vec!["^admin".to_string()]),
            trxids: Some(trxid_set),
            ..IncludeFilters::default()
        };
        let compiled =
            CompiledMetaFilters::try_from_include_exclude(&include, &ExcludeFilters::default())
                .unwrap();
        // 两者都满足 → 通过
        assert!(compiled.should_keep(&m("TX123", "ip", "admin_user", None)));
        // trxid 不匹配 → 拒绝（AND）
        assert!(!compiled.should_keep(&m("TX999", "ip", "admin_user", None)));
        // username 不匹配 → 拒绝（AND）
        assert!(!compiled.should_keep(&m("TX123", "ip", "other_user", None)));
    }

    // ── IncludeFilters / ExcludeFilters has_filters ────────────
    #[test]
    fn test_t1_meta_has_filters_with_exclude_usernames() {
        let exclude = ExcludeFilters {
            users: Some(vec!["guest".to_string()]),
            ..ExcludeFilters::default()
        };
        assert!(exclude.has_filters());
    }

    #[test]
    fn test_t1_meta_has_filters_all_none_is_false() {
        assert!(!IncludeFilters::default().has_filters());
        assert!(!ExcludeFilters::default().has_filters());
    }

    #[test]
    fn test_t1_compiled_from_meta_exclude_usernames() {
        let exclude = ExcludeFilters {
            users: Some(vec!["^guest".to_string()]),
            ..ExcludeFilters::default()
        };
        let compiled =
            CompiledMetaFilters::try_from_include_exclude(&IncludeFilters::default(), &exclude)
                .unwrap();
        assert!(compiled.exclude_usernames.is_some());
    }

    #[test]
    fn test_t1_compiled_from_meta_exclude_none() {
        let compiled = CompiledMetaFilters::try_from_include_exclude(
            &IncludeFilters::default(),
            &ExcludeFilters::default(),
        )
        .unwrap();
        assert!(compiled.exclude_usernames.is_none());
    }

    #[test]
    fn test_t1_has_any_filters_include_only() {
        let include = IncludeFilters {
            users: Some(vec!["admin".to_string()]),
            ..IncludeFilters::default()
        };
        let compiled =
            CompiledMetaFilters::try_from_include_exclude(&include, &ExcludeFilters::default())
                .unwrap();
        assert_eq!(compiled.has_any_filters(), compiled.has_filters());
    }

    #[test]
    fn test_t1_has_any_filters_exclude_only() {
        let exclude = ExcludeFilters {
            users: Some(vec!["guest".to_string()]),
            ..ExcludeFilters::default()
        };
        let compiled =
            CompiledMetaFilters::try_from_include_exclude(&IncludeFilters::default(), &exclude)
                .unwrap();
        assert!(compiled.has_any_filters());
        assert!(!compiled.has_filters()); // include-only check should be false
    }

    // ── exclude filters ────────────────────────────────────────
    fn make_compiled_with_exclude(
        exclude_users: Option<Vec<String>>,
        exclude_ips: Option<Vec<String>>,
    ) -> CompiledMetaFilters {
        let exclude = ExcludeFilters {
            users: exclude_users,
            ips: exclude_ips,
            ..ExcludeFilters::default()
        };
        CompiledMetaFilters::try_from_include_exclude(&IncludeFilters::default(), &exclude).unwrap()
    }

    #[test]
    fn test_exclude_username_drops_matching_record() {
        let compiled = make_compiled_with_exclude(Some(vec!["^guest".to_string()]), None);
        assert!(!compiled.should_keep(&m("tx", "1.2.3.4", "guest_01", None)));
    }

    #[test]
    fn test_exclude_username_retains_nonmatching_record() {
        let compiled = make_compiled_with_exclude(Some(vec!["^guest".to_string()]), None);
        assert!(compiled.should_keep(&m("tx", "1.2.3.4", "admin_dba", None)));
    }

    #[test]
    fn test_exclude_ip_drops_matching_record() {
        let compiled = make_compiled_with_exclude(None, Some(vec!["^10\\.0".to_string()]));
        assert!(!compiled.should_keep(&m("tx", "10.0.0.1", "user", None)));
        assert!(compiled.should_keep(&m("tx", "192.168.1.1", "user", None)));
    }

    #[test]
    fn test_exclude_or_veto_any_hit_drops() {
        let compiled = make_compiled_with_exclude(
            Some(vec!["guest".to_string()]),
            Some(vec!["^10".to_string()]),
        );
        // ip 命中 exclude → false
        assert!(!compiled.should_keep(&m("tx", "10.0.0.1", "admin", None)));
    }

    #[test]
    fn test_exclude_or_veto_no_hit_passes() {
        let compiled = make_compiled_with_exclude(
            Some(vec!["guest".to_string()]),
            Some(vec!["^10".to_string()]),
        );
        // 两者均未命中 → true
        assert!(compiled.should_keep(&m("tx", "192.168.1.1", "admin", None)));
    }

    #[test]
    fn test_exclude_with_include_veto_wins() {
        let include = IncludeFilters {
            users: Some(vec!["^admin".to_string()]),
            ..IncludeFilters::default()
        };
        let exclude = ExcludeFilters {
            ips: Some(vec!["^10".to_string()]),
            ..ExcludeFilters::default()
        };
        let compiled = CompiledMetaFilters::try_from_include_exclude(&include, &exclude).unwrap();
        // exclude ip 命中，veto 优先 → false
        assert!(!compiled.should_keep(&m("tx", "10.0.0.1", "admin", None)));
    }

    #[test]
    fn test_exclude_with_include_both_pass() {
        let include = IncludeFilters {
            users: Some(vec!["^admin".to_string()]),
            ..IncludeFilters::default()
        };
        let exclude = ExcludeFilters {
            ips: Some(vec!["^10".to_string()]),
            ..ExcludeFilters::default()
        };
        let compiled = CompiledMetaFilters::try_from_include_exclude(&include, &exclude).unwrap();
        // exclude 未命中，include 满足 → true
        assert!(compiled.should_keep(&m("tx", "192.168.1.1", "admin", None)));
    }

    #[test]
    fn test_exclude_with_include_include_fails() {
        let include = IncludeFilters {
            users: Some(vec!["^admin".to_string()]),
            ..IncludeFilters::default()
        };
        let exclude = ExcludeFilters {
            ips: Some(vec!["^10".to_string()]),
            ..ExcludeFilters::default()
        };
        let compiled = CompiledMetaFilters::try_from_include_exclude(&include, &exclude).unwrap();
        // exclude 未命中，但 include 不满足 → false
        assert!(!compiled.should_keep(&m("tx", "192.168.1.1", "sys_user", None)));
    }

    #[test]
    fn test_exclude_tags_drops_matching() {
        let exclude = ExcludeFilters {
            tags: Some(vec!["^SEL".to_string()]),
            ..ExcludeFilters::default()
        };
        let compiled =
            CompiledMetaFilters::try_from_include_exclude(&IncludeFilters::default(), &exclude)
                .unwrap();
        assert!(!compiled.should_keep(&m("tx", "ip", "user", Some("SELECT"))));
    }

    #[test]
    fn test_exclude_tags_retains_no_tag() {
        let exclude = ExcludeFilters {
            tags: Some(vec!["^SEL".to_string()]),
            ..ExcludeFilters::default()
        };
        let compiled =
            CompiledMetaFilters::try_from_include_exclude(&IncludeFilters::default(), &exclude)
                .unwrap();
        // tag=None 时不触发 exclude，保留
        assert!(compiled.should_keep(&m("tx", "ip", "user", None)));
    }

    #[test]
    fn test_exclude_tags_retains_nonmatching() {
        let exclude = ExcludeFilters {
            tags: Some(vec!["^SEL".to_string()]),
            ..ExcludeFilters::default()
        };
        let compiled =
            CompiledMetaFilters::try_from_include_exclude(&IncludeFilters::default(), &exclude)
                .unwrap();
        assert!(compiled.should_keep(&m("tx", "ip", "user", Some("INSERT"))));
    }

    #[test]
    fn test_has_any_filters_exclude_only() {
        let exclude = ExcludeFilters {
            users: Some(vec!["guest".to_string()]),
            ..ExcludeFilters::default()
        };
        let compiled =
            CompiledMetaFilters::try_from_include_exclude(&IncludeFilters::default(), &exclude)
                .unwrap();
        assert!(compiled.has_any_filters());
        assert!(!compiled.has_filters());
    }

    #[test]
    fn test_meta_has_filters_with_exclude() {
        let exclude = ExcludeFilters {
            users: Some(vec!["x".to_string()]),
            ..ExcludeFilters::default()
        };
        assert!(exclude.has_filters());
    }

    #[test]
    fn test_meta_has_filters_all_none() {
        assert!(!IncludeFilters::default().has_filters());
        assert!(!ExcludeFilters::default().has_filters());
    }

    #[test]
    fn test_exclude_invalid_regex_validate_fails() {
        let exclude = ExcludeFilters {
            users: Some(vec!["[invalid".to_string()]),
            ..ExcludeFilters::default()
        };
        let result = crate::features::filters::CompiledMetaFilters::try_from_include_exclude(
            &IncludeFilters::default(),
            &exclude,
        );
        assert!(result.is_err());
    }

    // ── CompiledSqlFilters ─────────────────────────────────────
    #[test]
    fn test_compiled_sql_include_regex() {
        let sf = SqlFilters {
            includes: Some(vec!["^SELECT".to_string()]),
            excludes: None,
        };
        let compiled = CompiledSqlFilters::try_from_sql_filters(&sf).unwrap();
        assert!(compiled.matches("SELECT * FROM t"));
        assert!(!compiled.matches("INSERT INTO t VALUES (1)"));
    }

    #[test]
    fn test_compiled_sql_exclude_regex() {
        let sf = SqlFilters {
            includes: None,
            excludes: Some(vec!["DROP".to_string()]),
        };
        let compiled = CompiledSqlFilters::try_from_sql_filters(&sf).unwrap();
        assert!(compiled.matches("SELECT 1"));
        assert!(!compiled.matches("DROP TABLE t"));
    }

    #[test]
    fn test_filters_toml_deserialization_with_trxids_and_exec_ids() {
        // Exercises vec_to_hashset (lines 22-29) and vec_to_i64_hashset (lines 32-37)
        use crate::config::Config;
        let toml = r#"
[sqllog]
path = "sqllogs"
[features.filters]
enable = true
trxids = ["123", "456"]
[features.filters.indicators]
exec_ids = [1, 2, 3]
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let filters = cfg.features.filters.unwrap();
        assert!(filters.include.trxids.is_some());
        assert_eq!(filters.include.trxids.unwrap().len(), 2);
        assert!(filters.indicators.exec_ids.is_some());
        assert_eq!(filters.indicators.exec_ids.unwrap().len(), 3);
    }

    // ── Phase 17: 新格式嵌套解析测试（RED 阶段）────────────────────

    #[test]
    fn test_new_nested_format_include() {
        use crate::config::Config;
        let toml = r#"
[sqllog]
path = "sqllogs"
[features.filters]
enable = true
[features.filters.include]
users = ["user1"]
ips = ["192.168.1.1"]
sessions = ["s001"]
threads = ["t001"]
statements = ["SELECT", "INSERT"]
apps = ["myapp"]
tags = ["audit"]
start_ts = "2024-01-01T00:00:00"
end_ts = "2024-12-31T23:59:59"
trxids = ["123456"]
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let filters = cfg.features.filters.unwrap();
        assert_eq!(filters.include.users, Some(vec!["user1".to_string()]));
        assert_eq!(filters.include.ips, Some(vec!["192.168.1.1".to_string()]));
        assert!(filters.include.trxids.is_some());
        assert_eq!(filters.include.trxids.unwrap().len(), 1);
    }

    #[test]
    fn test_new_nested_format_exclude() {
        use crate::config::Config;
        let toml = r#"
[sqllog]
path = "sqllogs"
[features.filters]
enable = true
[features.filters.exclude]
users = ["admin"]
ips = ["10.0.0.1"]
sessions = ["s999"]
threads = ["t999"]
statements = ["DROP"]
apps = ["badapp"]
tags = ["sys"]
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let filters = cfg.features.filters.unwrap();
        assert_eq!(filters.exclude.users, Some(vec!["admin".to_string()]));
        assert_eq!(filters.exclude.ips, Some(vec!["10.0.0.1".to_string()]));
    }

    #[test]
    fn test_backward_compat_flat_format() {
        use crate::config::Config;
        let toml = r#"
[sqllog]
path = "sqllogs"
[features.filters]
enable = true
usernames = ["user1"]
exclude_usernames = ["admin"]
client_ips = ["192.168.1.1"]
exclude_client_ips = ["10.0.0.1"]
sess_ids = ["s001"]
thrd_ids = ["t001"]
exclude_sess_ids = ["s999"]
exclude_thrd_ids = ["t999"]
statements = ["SELECT"]
exclude_statements = ["DROP"]
appnames = ["myapp"]
exclude_appnames = ["badapp"]
tags = ["audit"]
exclude_tags = ["sys"]
start_ts = "2024-01-01T00:00:00"
end_ts = "2024-12-31T23:59:59"
trxids = ["123"]
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let filters = cfg.features.filters.unwrap();
        // include 类字段映射到 filters.include
        assert_eq!(filters.include.users, Some(vec!["user1".to_string()]));
        assert_eq!(filters.include.ips, Some(vec!["192.168.1.1".to_string()]));
        assert!(filters.include.trxids.is_some());
        assert_eq!(filters.include.trxids.unwrap().len(), 1);
        // exclude 类字段映射到 filters.exclude
        assert_eq!(filters.exclude.users, Some(vec!["admin".to_string()]));
        assert_eq!(filters.exclude.ips, Some(vec!["10.0.0.1".to_string()]));
    }

    #[test]
    fn test_sql_filters_alias_backward_compat() {
        use crate::config::Config;
        let toml = r#"
[sqllog]
path = "sqllogs"
[features.filters]
enable = true
[features.filters.sql]
include_patterns = ["SELECT"]
exclude_patterns = ["DROP"]
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let filters = cfg.features.filters.unwrap();
        assert_eq!(filters.sql.includes, Some(vec!["SELECT".to_string()]));
        assert_eq!(filters.sql.excludes, Some(vec!["DROP".to_string()]));
    }

    #[test]
    fn test_sql_filters_new_field_names() {
        use crate::config::Config;
        let toml = r#"
[sqllog]
path = "sqllogs"
[features.filters]
enable = true
[features.filters.sql]
includes = ["FROM USER_TABLES"]
excludes = ["SELECT 1", "DUAL"]
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let filters = cfg.features.filters.unwrap();
        assert_eq!(
            filters.sql.includes,
            Some(vec!["FROM USER_TABLES".to_string()])
        );
        assert_eq!(
            filters.sql.excludes,
            Some(vec!["SELECT 1".to_string(), "DUAL".to_string()])
        );
    }

    #[test]
    fn test_mixed_format_new_format_wins_and_warns() {
        use crate::config::Config;
        let toml = r#"
[sqllog]
path = "sqllogs"
[features.filters]
enable = true
usernames = ["old_user"]
[features.filters.include]
users = ["new_user"]
[exporter.csv]
file = "out.csv"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let filters = cfg.features.filters.unwrap();
        // 新格式子表优先
        assert_eq!(
            filters.include.users,
            Some(vec!["new_user".to_string()]),
            "new nested format should win; old flat field should be dropped"
        );
        // 旧扁平字段被丢弃，不应出现在结果中
        assert!(
            !filters
                .include
                .users
                .as_ref()
                .unwrap()
                .contains(&"old_user".to_string()),
            "legacy flat field must not be merged into new format result"
        );
    }
}
