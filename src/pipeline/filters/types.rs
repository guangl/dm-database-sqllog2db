use serde::Deserialize;

use super::serde_helpers::{TrxidSet, non_negative_finite_ms, vec_to_hashset, vec_to_i64_hashset};

/// 过滤只分保留和排除两组；配置条件自动生效。
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct FiltersFeature {
    #[serde(default)]
    pub include: IncludeFilters,
    #[serde(default)]
    pub exclude: ExcludeFilters,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct IncludeFilters {
    #[serde(default)]
    pub sql: Option<Vec<String>>,
    #[serde(default, deserialize_with = "vec_to_i64_hashset")]
    pub exec_ids: Option<std::collections::HashSet<i64>>,
    #[serde(default, deserialize_with = "non_negative_finite_ms")]
    pub min_runtime_ms: Option<f32>,
    pub min_row_count: Option<u32>,
    pub users: Option<Vec<String>>,
    pub ips: Option<Vec<String>>,
    pub sessions: Option<Vec<String>>,
    pub threads: Option<Vec<String>>,
    pub apps: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub start_ts: Option<String>,
    pub end_ts: Option<String>,
    #[serde(default, deserialize_with = "vec_to_hashset")]
    pub trxids: Option<TrxidSet>,
}

impl IncludeFilters {
    pub(crate) fn has_transaction_filters(&self) -> bool {
        self.exec_ids.as_ref().is_some_and(|v| !v.is_empty())
            || self.min_runtime_ms.is_some()
            || self.min_row_count.is_some()
            || self.sql.as_ref().is_some_and(|v| !v.is_empty())
    }

    pub(crate) fn clear_transaction_filters(&mut self) {
        self.exec_ids = None;
        self.min_runtime_ms = None;
        self.min_row_count = None;
        self.sql = None;
    }

    #[must_use]
    pub fn has_filters(&self) -> bool {
        self.users.as_ref().is_some_and(|v| !v.is_empty())
            || self.ips.as_ref().is_some_and(|v| !v.is_empty())
            || self.sessions.as_ref().is_some_and(|v| !v.is_empty())
            || self.threads.as_ref().is_some_and(|v| !v.is_empty())
            || self.apps.as_ref().is_some_and(|v| !v.is_empty())
            || self.tags.as_ref().is_some_and(|v| !v.is_empty())
            || self.start_ts.is_some()
            || self.end_ts.is_some()
            || self.trxids.is_some()
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct ExcludeFilters {
    #[serde(default)]
    pub sql: Option<Vec<String>>,
    #[serde(default, deserialize_with = "vec_to_i64_hashset")]
    pub exec_ids: Option<std::collections::HashSet<i64>>,
    #[serde(default, deserialize_with = "non_negative_finite_ms")]
    pub min_runtime_ms: Option<f32>,
    pub min_row_count: Option<u32>,
    pub users: Option<Vec<String>>,
    pub ips: Option<Vec<String>>,
    pub sessions: Option<Vec<String>>,
    pub threads: Option<Vec<String>>,
    pub apps: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

impl ExcludeFilters {
    pub(crate) fn has_transaction_filters(&self) -> bool {
        self.exec_ids.as_ref().is_some_and(|v| !v.is_empty())
            || self.min_runtime_ms.is_some()
            || self.min_row_count.is_some()
            || self.sql.as_ref().is_some_and(|v| !v.is_empty())
    }

    pub(crate) fn clear_transaction_filters(&mut self) {
        self.exec_ids = None;
        self.min_runtime_ms = None;
        self.min_row_count = None;
        self.sql = None;
    }

    #[must_use]
    pub fn has_filters(&self) -> bool {
        self.users.as_ref().is_some_and(|v| !v.is_empty())
            || self.ips.as_ref().is_some_and(|v| !v.is_empty())
            || self.sessions.as_ref().is_some_and(|v| !v.is_empty())
            || self.threads.as_ref().is_some_and(|v| !v.is_empty())
            || self.apps.as_ref().is_some_and(|v| !v.is_empty())
            || self.tags.as_ref().is_some_and(|v| !v.is_empty())
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/pipeline/filters/types.rs"]
mod tests;

impl FiltersFeature {
    /// 检查是否提供了需要预扫描的过滤器 (Transaction-level)
    #[must_use]
    pub(crate) fn has_transaction_filters(&self) -> bool {
        self.include.has_transaction_filters() || self.exclude.has_transaction_filters()
    }

    /// 合并预扫描发现的事务 ID 到 `IncludeFilters` 中，以便在正式扫描时直接通过 trxid 匹配保留整笔事务。
    ///
    /// 即使 `trxids` 为空（预扫描运行但无命中），也会初始化 `include.trxids` 为空集合，
    /// 使 `has_filters()` 通过 `trxids.is_some()` 返回 `true`，
    /// 从而确保 `FilterProcessor` 进入 pipeline 并拒绝所有记录。
    pub(crate) fn merge_found_trxids(&mut self, trxids: Vec<String>) {
        // 始终初始化 trxids 集合，即使 trxids 为空，
        // 以确保 include.has_filters() 在预扫描已运行但无命中时返回 true
        self.include
            .trxids
            .get_or_insert_with(TrxidSet::default)
            .extend(trxids);
    }
}
