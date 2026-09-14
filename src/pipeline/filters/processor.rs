use crate::config::Config;
use crate::pipeline::{FiltersFeature, LogProcessor, Pipeline};
use dm_database_parser_sqllog::{Filter, FilterBuilder, Sqllog};
use std::collections::HashSet;

pub(crate) fn build_pipeline(cfg: &Config) -> Pipeline {
    let mut pipeline = Pipeline::new();
    if let Some(f) = cfg.filter.as_ref()
        && (f.include.has_filters() || f.exclude.has_filters() || f.has_transaction_filters())
    {
        pipeline.add(Box::new(FilterProcessor::from_feature(f)));
    }
    pipeline
}

struct FilterProcessor {
    base_filter: Filter,
    transactions: super::transaction::TransactionFilters,
    include_groups: Vec<Vec<Filter>>,
    exclude_groups: Vec<Vec<Filter>>,
    trxid_set: Option<HashSet<String>>,
    has_meta_filters: bool,
}

impl std::fmt::Debug for FilterProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilterProcessor")
            .field("has_meta_filters", &self.has_meta_filters)
            .finish_non_exhaustive()
    }
}

fn build_or_group(
    values: Option<&[String]>,
    make: impl Fn(FilterBuilder, &str) -> FilterBuilder,
) -> Vec<Filter> {
    match values {
        None | Some([]) => Vec::new(),
        Some(v) => v
            .iter()
            .map(|val| make(FilterBuilder::new(), val).build())
            .collect(),
    }
}

fn build_include_groups(f: &FiltersFeature) -> Vec<Vec<Filter>> {
    vec![
        build_or_group(f.include.users.as_deref(), |fb, v| fb.username_eq(v)),
        build_or_group(f.include.ips.as_deref(), |fb, v| fb.client_ip_eq(v)),
        build_or_group(f.include.sessions.as_deref(), |fb, v| fb.sess_id_eq(v)),
        build_or_group(f.include.threads.as_deref(), |fb, v| fb.thrd_id_eq(v)),
        build_or_group(f.include.apps.as_deref(), |fb, v| fb.appname_eq(v)),
        // 语句类型匹配 tag（不含方括号），而非 stmt 句柄。
        build_or_group(f.include.tags.as_deref(), |fb, v| fb.tag_eq(v)),
    ]
}

fn build_exclude_groups(f: &FiltersFeature) -> Vec<Vec<Filter>> {
    vec![
        build_or_group(f.exclude.users.as_deref(), |fb, v| fb.username_eq(v)),
        build_or_group(f.exclude.ips.as_deref(), |fb, v| fb.client_ip_eq(v)),
        build_or_group(f.exclude.sessions.as_deref(), |fb, v| fb.sess_id_eq(v)),
        build_or_group(f.exclude.threads.as_deref(), |fb, v| fb.thrd_id_eq(v)),
        build_or_group(f.exclude.apps.as_deref(), |fb, v| fb.appname_eq(v)),
        build_or_group(f.exclude.tags.as_deref(), |fb, v| fb.tag_eq(v)),
    ]
}

impl FilterProcessor {
    fn from_feature(f: &FiltersFeature) -> Self {
        let mut ts_builder = FilterBuilder::new();
        if let Some(start) = &f.include.start_ts {
            ts_builder = ts_builder.ts_gte(start.clone());
        }
        if let Some(end) = &f.include.end_ts {
            ts_builder = ts_builder.ts_lte(end.clone());
        }
        let base_filter = ts_builder.build();
        let include_groups = build_include_groups(f);
        let exclude_groups = build_exclude_groups(f);
        let trxid_set = f.include.trxids.clone();
        // trxid_set.is_some() 包含空集合（预扫描运行但无命中的 sentinel），
        // 空集合应拒绝所有记录，因此需要触发元过滤器路径
        let has_meta_filters = include_groups.iter().any(|g| !g.is_empty())
            || exclude_groups.iter().any(|g| !g.is_empty())
            || trxid_set.is_some();
        Self {
            base_filter,
            transactions: super::transaction::TransactionFilters::new(f),
            include_groups,
            exclude_groups,
            trxid_set,
            has_meta_filters,
        }
    }
}

impl LogProcessor for FilterProcessor {
    fn process(&self, record: &Sqllog) -> bool {
        self.process_with_meta(record)
    }

    fn process_with_meta(&self, record: &Sqllog) -> bool {
        if !self.base_filter.matches(record)
            || !self.transactions.includes(record)
            || self.transactions.excludes(record)
        {
            return false;
        }

        if !self.has_meta_filters {
            return true;
        }

        for group in &self.exclude_groups {
            if !group.is_empty() && group.iter().any(|f| f.matches(record)) {
                return false;
            }
        }

        for group in &self.include_groups {
            if !group.is_empty() && !group.iter().any(|f| f.matches(record)) {
                return false;
            }
        }

        if let Some(trxids) = &self.trxid_set {
            // 空集合是预扫描无命中的 sentinel，应拒绝所有记录
            if trxids.is_empty() || !trxids.contains(record.trxid.as_str()) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/pipeline/filters/processor.rs"]
mod tests;
