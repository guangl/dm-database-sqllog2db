mod compiled;
mod serde_helpers;
pub mod types;

use compact_str::CompactString;
use serde_helpers::TrxidSet;

// 仅在 crate 内部使用（validate_and_compile 返回类型），integration tests 不直接引用。
pub(crate) use compiled::{CompiledMetaFilters, CompiledSqlFilters};

#[cfg(test)]
use types::{ExcludeFilters, IncludeFilters};

// pub: required by tests/integration.rs
pub use types::{IndicatorFilters, SqlFilters};

// FiltersFeature: pub because integration tests directly construct it and access cfg.filter
pub use types::FiltersFeature;
pub(crate) use types::RecordMeta;

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
    pub(crate) fn has_transaction_filters(&self) -> bool {
        // 如果未开启过滤器功能，则不执行预扫描
        if !self.enable {
            return false;
        }
        self.indicators.has_filters() || self.sql.has_filters()
    }

    /// 合并预扫描发现的事务 ID 到 `IncludeFilters` 中，以便在正式扫描时直接通过 trxid 匹配保留整笔事务
    pub(crate) fn merge_found_trxids(&mut self, trxids: Vec<CompactString>) {
        if !self.enable || trxids.is_empty() {
            return;
        }
        self.include
            .trxids
            .get_or_insert_with(TrxidSet::default)
            .extend(trxids);
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
                patterns.iter().any(|p| sql.contains(p.as_str()))
            }
        } else {
            true
        };

        if !include_match {
            return false;
        }

        // 如果指定了排除模式，不能命中任何一个
        if let Some(patterns) = &self.excludes {
            if patterns.iter().any(|p| sql.contains(p.as_str())) {
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
        f.merge_found_trxids(vec!["TX1".into(), "TX2".into()]);
        let trxids = f.include.trxids.unwrap();
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
}
