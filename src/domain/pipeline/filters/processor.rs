use super::matcher::Predicate;
use crate::config::Config;
use crate::model::LogRecord;
use crate::pipeline::{FiltersFeature, LogProcessor, Pipeline};
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
    base_filters: Vec<Predicate>,
    transactions: super::transaction::TransactionFilters,
    include_groups: Vec<Vec<Predicate>>,
    exclude_groups: Vec<Vec<Predicate>>,
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

fn build_or_group(values: Option<&[String]>, make: impl Fn(&str) -> Predicate) -> Vec<Predicate> {
    values
        .unwrap_or_default()
        .iter()
        .map(String::as_str)
        .map(make)
        .collect()
}

fn build_include_groups(f: &FiltersFeature) -> Vec<Vec<Predicate>> {
    vec![
        build_or_group(f.include.users.as_deref(), |v| {
            Predicate::Username(v.to_owned())
        }),
        build_or_group(f.include.ips.as_deref(), |v| {
            Predicate::ClientIp(v.to_owned())
        }),
        build_or_group(f.include.sessions.as_deref(), |v| {
            Predicate::Session(v.to_owned())
        }),
        build_or_group(f.include.threads.as_deref(), |v| {
            Predicate::Thread(v.to_owned())
        }),
        build_or_group(f.include.apps.as_deref(), |v| Predicate::App(v.to_owned())),
        // 语句类型匹配 tag（不含方括号），而非 stmt 句柄。
        build_or_group(f.include.tags.as_deref(), |v| Predicate::Tag(v.to_owned())),
    ]
}

fn build_exclude_groups(f: &FiltersFeature) -> Vec<Vec<Predicate>> {
    vec![
        build_or_group(f.exclude.users.as_deref(), |v| {
            Predicate::Username(v.to_owned())
        }),
        build_or_group(f.exclude.ips.as_deref(), |v| {
            Predicate::ClientIp(v.to_owned())
        }),
        build_or_group(f.exclude.sessions.as_deref(), |v| {
            Predicate::Session(v.to_owned())
        }),
        build_or_group(f.exclude.threads.as_deref(), |v| {
            Predicate::Thread(v.to_owned())
        }),
        build_or_group(f.exclude.apps.as_deref(), |v| Predicate::App(v.to_owned())),
        build_or_group(f.exclude.tags.as_deref(), |v| Predicate::Tag(v.to_owned())),
    ]
}

impl FilterProcessor {
    fn from_feature(f: &FiltersFeature) -> Self {
        let mut base_filters = Vec::new();
        if let Some(start) = &f.include.start_ts {
            base_filters.push(Predicate::TimestampAtLeast(start.clone()));
        }
        if let Some(end) = &f.include.end_ts {
            base_filters.push(Predicate::TimestampAtMost(end.clone()));
        }
        let include_groups = build_include_groups(f);
        let exclude_groups = build_exclude_groups(f);
        let trxid_set = f.include.trxids.clone();
        // trxid_set.is_some() 包含空集合（预扫描运行但无命中的 sentinel），
        // 空集合应拒绝所有记录，因此需要触发元过滤器路径
        let has_meta_filters = include_groups.iter().any(|g| !g.is_empty())
            || exclude_groups.iter().any(|g| !g.is_empty())
            || trxid_set.is_some();
        Self {
            base_filters,
            transactions: super::transaction::TransactionFilters::new(f),
            include_groups,
            exclude_groups,
            trxid_set,
            has_meta_filters,
        }
    }
}

impl LogProcessor for FilterProcessor {
    fn process(&self, record: &LogRecord) -> bool {
        self.process_with_meta(record)
    }

    fn process_with_meta(&self, record: &LogRecord) -> bool {
        if !self
            .base_filters
            .iter()
            .all(|filter| filter.matches(record))
            || !self.transactions.includes(record)
            || self.transactions.excludes(record)
        {
            return false;
        }

        if !self.has_meta_filters {
            return true;
        }

        for group in &self.exclude_groups {
            if !group.is_empty() && group.iter().any(|filter| filter.matches(record)) {
                return false;
            }
        }

        for group in &self.include_groups {
            if !group.is_empty() && !group.iter().any(|filter| filter.matches(record)) {
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
#[path = "../../../../tests/unit/domain/pipeline/filters/processor.rs"]
mod tests;
