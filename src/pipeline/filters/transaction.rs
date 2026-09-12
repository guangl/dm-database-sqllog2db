use super::types::FiltersFeature;
use dm_database_parser_sqllog::{Filter, FilterBuilder, Sqllog};

pub(crate) struct TransactionFilters {
    include: Vec<Filter>,
    exclude: Vec<Filter>,
}

impl TransactionFilters {
    pub(crate) fn new(feature: &FiltersFeature) -> Self {
        let mut include = build_indicator_filters(
            feature.include.exec_ids.as_ref(),
            feature.include.min_runtime_ms,
            feature.include.min_row_count,
        );
        include.extend(build_sql_filters(feature.include.sql.as_deref()));
        let mut exclude = build_indicator_filters(
            feature.exclude.exec_ids.as_ref(),
            feature.exclude.min_runtime_ms,
            feature.exclude.min_row_count,
        );
        exclude.extend(build_sql_filters(feature.exclude.sql.as_deref()));
        Self { include, exclude }
    }

    pub(crate) fn includes(&self, record: &Sqllog) -> bool {
        self.include.is_empty() || self.include.iter().any(|f| f.matches(record))
    }

    pub(crate) fn excludes(&self, record: &Sqllog) -> bool {
        self.exclude.iter().any(|f| f.matches(record))
    }
}

// ===== Pre-scan: 指标/SQL 过滤器构建 =====

fn build_indicator_filters(
    exec_ids: Option<&std::collections::HashSet<i64>>,
    min_runtime_ms: Option<f32>,
    min_row_count: Option<u32>,
) -> Vec<Filter> {
    let mut filters = Vec::new();
    if let Some(min_ms) = min_runtime_ms {
        filters.push(FilterBuilder::new().exec_time_gte(min_ms).build());
    }
    if let Some(min_r) = min_row_count {
        // rowcount >= min_r: for u32, rowcount_gt(min_r - 1) works when min_r > 0
        let filter = if min_r == 0 {
            FilterBuilder::new().build()
        } else {
            FilterBuilder::new().rowcount_gt(min_r - 1).build()
        };
        filters.push(filter);
    }
    if let Some(ids) = exec_ids {
        for &id in ids {
            filters.push(FilterBuilder::new().exec_id_eq(id).build());
        }
    }
    filters
}

pub(crate) fn build_sql_filters(patterns: Option<&[String]>) -> Vec<Filter> {
    patterns
        .unwrap_or_default()
        .iter()
        .map(|p| FilterBuilder::new().sql_contains(p.clone()).build())
        .collect()
}

impl std::fmt::Debug for TransactionFilters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransactionFilters")
            .field("include_count", &self.include.len())
            .field("exclude_count", &self.exclude.len())
            .finish()
    }
}
