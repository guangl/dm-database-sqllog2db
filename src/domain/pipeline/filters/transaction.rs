use super::matcher::Predicate;
use super::types::FiltersFeature;
use crate::model::LogRecord;

pub(crate) struct TransactionFilters {
    include: Vec<Predicate>,
    exclude: Vec<Predicate>,
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

    pub(crate) fn includes(&self, record: &LogRecord) -> bool {
        self.include.is_empty() || self.include.iter().any(|f| f.matches(record))
    }

    pub(crate) fn excludes(&self, record: &LogRecord) -> bool {
        self.exclude.iter().any(|f| f.matches(record))
    }
}

// ===== Pre-scan: 指标/SQL 过滤器构建 =====

fn build_indicator_filters(
    exec_ids: Option<&std::collections::HashSet<i64>>,
    min_runtime_ms: Option<f32>,
    min_row_count: Option<u32>,
) -> Vec<Predicate> {
    let mut filters = Vec::new();
    if let Some(min_ms) = min_runtime_ms {
        filters.push(Predicate::RuntimeAtLeast(min_ms));
    }
    if let Some(min_r) = min_row_count {
        // Keep zero as an active predicate: it is a deliberate "match every
        // record" transaction filter, including when used on the exclude side.
        filters.push(Predicate::RowCountAtLeast(min_r));
    }
    if let Some(ids) = exec_ids {
        for &id in ids {
            filters.push(Predicate::ExecId(id));
        }
    }
    filters
}

fn build_sql_filters(patterns: Option<&[String]>) -> Vec<Predicate> {
    patterns
        .unwrap_or_default()
        .iter()
        .map(|p| Predicate::SqlContains(p.clone()))
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
