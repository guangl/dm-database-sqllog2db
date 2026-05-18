use super::serde_helpers::TrxidSet;
use super::serde_helpers::compile_patterns;
use super::serde_helpers::match_any_regex;
use super::types::{ExcludeFilters, IncludeFilters, RecordMeta, SqlFilters};
use regex::Regex;

/// 预编译后的元数据过滤器，在热路径中使用。由 `MetaFilters` 在启动时构造。
// 结构体 `pub`（因 handle_run 等 pub 函数签名中使用），但字段为 `pub(crate)`，
// 且 filters/mod.rs 通过 `pub(crate) use` 限制 crate 外部访问。
#[derive(Debug)]
pub struct CompiledMetaFilters {
    pub(crate) usernames: Option<Vec<Regex>>,
    pub(crate) client_ips: Option<Vec<Regex>>,
    pub(crate) sess_ids: Option<Vec<Regex>>,
    pub(crate) thrd_ids: Option<Vec<Regex>>,
    pub(crate) statements: Option<Vec<Regex>>,
    pub(crate) appnames: Option<Vec<Regex>>,
    pub(crate) tags: Option<Vec<Regex>>,
    pub(crate) trxids: Option<TrxidSet>,
    pub(crate) exclude_usernames: Option<Vec<Regex>>,
    pub(crate) exclude_client_ips: Option<Vec<Regex>>,
    pub(crate) exclude_sess_ids: Option<Vec<Regex>>,
    pub(crate) exclude_thrd_ids: Option<Vec<Regex>>,
    pub(crate) exclude_statements: Option<Vec<Regex>>,
    pub(crate) exclude_appnames: Option<Vec<Regex>>,
    pub(crate) exclude_tags: Option<Vec<Regex>>,
}

impl CompiledMetaFilters {
    /// 从 `IncludeFilters` 和 `ExcludeFilters` 编译所有正则，
    /// 遇到非法 pattern 返回 `ConfigError::InvalidValue`。
    pub(crate) fn try_from_include_exclude(
        include: &IncludeFilters,
        exclude: &ExcludeFilters,
    ) -> crate::error::Result<Self> {
        Ok(Self {
            usernames: compile_patterns("filter.include.users", include.users.as_deref())?,
            client_ips: compile_patterns("filter.include.ips", include.ips.as_deref())?,
            sess_ids: compile_patterns("filter.include.sessions", include.sessions.as_deref())?,
            thrd_ids: compile_patterns("filter.include.threads", include.threads.as_deref())?,
            statements: compile_patterns(
                "filter.include.statements",
                include.statements.as_deref(),
            )?,
            appnames: compile_patterns("filter.include.apps", include.apps.as_deref())?,
            tags: compile_patterns("filter.include.tags", include.tags.as_deref())?,
            trxids: include.trxids.clone(),
            exclude_usernames: compile_patterns("filter.exclude.users", exclude.users.as_deref())?,
            exclude_client_ips: compile_patterns("filter.exclude.ips", exclude.ips.as_deref())?,
            exclude_sess_ids: compile_patterns(
                "filter.exclude.sessions",
                exclude.sessions.as_deref(),
            )?,
            exclude_thrd_ids: compile_patterns(
                "filter.exclude.threads",
                exclude.threads.as_deref(),
            )?,
            exclude_statements: compile_patterns(
                "filter.exclude.statements",
                exclude.statements.as_deref(),
            )?,
            exclude_appnames: compile_patterns("filter.exclude.apps", exclude.apps.as_deref())?,
            exclude_tags: compile_patterns("filter.exclude.tags", exclude.tags.as_deref())?,
        })
    }

    /// 是否有任何已编译的过滤条件（用于快路径跳过）。
    /// 只检查 include 字段，不含 exclude 字段。
    #[must_use]
    pub(crate) fn has_filters(&self) -> bool {
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
    pub(crate) fn has_any_filters(&self) -> bool {
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
    pub(crate) fn should_keep(&self, meta: &RecordMeta) -> bool {
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
// 结构体 `pub`（因 handle_run 等 pub 函数签名中使用），但字段为 `pub(crate)`，
// 且 filters/mod.rs 通过 `pub(crate) use` 限制 crate 外部访问。
#[derive(Debug)]
pub struct CompiledSqlFilters {
    pub(crate) include_patterns: Option<Vec<Regex>>,
    pub(crate) exclude_patterns: Option<Vec<Regex>>,
}

impl CompiledSqlFilters {
    /// 从 `SqlFilters` 编译正则，遇到非法 pattern 返回 `ConfigError::InvalidValue`。
    pub(crate) fn try_from_sql_filters(sf: &SqlFilters) -> crate::error::Result<Self> {
        Ok(Self {
            include_patterns: compile_patterns(
                "filter.record_sql.includes",
                sf.includes.as_deref(),
            )?,
            exclude_patterns: compile_patterns(
                "filter.record_sql.excludes",
                sf.excludes.as_deref(),
            )?,
        })
    }

    /// 是否有任何已编译的过滤条件。
    #[must_use]
    #[allow(dead_code)]
    pub(crate) fn has_filters(&self) -> bool {
        self.include_patterns.is_some() || self.exclude_patterns.is_some()
    }

    /// 判断 SQL 是否通过过滤：
    /// - include：必须命中其中之一（未配置 = 通过）
    /// - exclude：不能命中任何一个
    #[inline]
    #[must_use]
    pub(crate) fn matches(&self, sql: &str) -> bool {
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

#[cfg(test)]
#[path = "compiled_tests.rs"]
mod compiled_tests;
