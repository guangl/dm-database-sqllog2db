mod serde_helpers;
pub mod types;

pub(crate) use serde_helpers::TrxidSet;

#[cfg(test)]
use types::{ExcludeFilters, IncludeFilters};

// pub: required by tests/integration.rs
pub use types::{IndicatorFilters, SqlFilters};

// FiltersFeature: pub because integration tests directly construct it and access cfg.filter
pub use types::FiltersFeature;

#[cfg(test)]
impl FiltersFeature {
    #[must_use]
    pub fn has_filters(&self) -> bool {
        if !self.enable {
            return false;
        }
        self.include.has_filters()
            || self.exclude.has_filters()
            || self.indicators.has_filters()
            || self.sql.has_filters()
    }
}

impl FiltersFeature {
    /// 检查是否提供了需要预扫描的过滤器 (Transaction-level)
    #[must_use]
    pub(crate) fn has_transaction_filters(&self) -> bool {
        // 如果未开启过滤器功能，则不执行预扫描
        if !self.enable {
            return false;
        }
        self.indicators.has_filters() || self.sql.has_filters()
    }

    /// 合并预扫描发现的事务 ID 到 `IncludeFilters` 中，以便在正式扫描时直接通过 trxid 匹配保留整笔事务。
    ///
    /// 即使 `trxids` 为空（预扫描运行但无命中），也会初始化 `include.trxids` 为空集合，
    /// 使 `has_filters()` 通过 `trxids.is_some()` 返回 `true`，
    /// 从而确保 `FilterProcessor` 进入 pipeline 并拒绝所有记录。
    pub(crate) fn merge_found_trxids(&mut self, trxids: Vec<String>) {
        if !self.enable {
            return;
        }
        // 始终初始化 trxids 集合，即使 trxids 为空，
        // 以确保 include.has_filters() 在预扫描已运行但无命中时返回 true
        self.include
            .trxids
            .get_or_insert_with(TrxidSet::default)
            .extend(trxids);
    }
}

impl IndicatorFilters {
    #[must_use]
    pub(crate) fn has_filters(&self) -> bool {
        self.exec_ids.as_ref().is_some_and(|v| !v.is_empty())
            || self.min_runtime_ms.is_some()
            || self.min_row_count.is_some()
    }

    #[cfg(test)]
    #[must_use]
    pub fn matches(&self, exec_id: i64, runtime_ms: f32, row_count: u32) -> bool {
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
            if row_count >= min_r {
                return true;
            }
        }
        false
    }
}

impl SqlFilters {
    #[must_use]
    pub(crate) fn has_filters(&self) -> bool {
        self.includes.as_ref().is_some_and(|v| !v.is_empty())
            || self.excludes.as_ref().is_some_and(|v| !v.is_empty())
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
    fn test_merge_found_trxids_empty_list_initializes_sentinel() {
        // 空列表时仍应初始化空集合（sentinel），使 has_filters() 返回 true，
        // 确保预扫描无命中时 FilterProcessor 进入 pipeline 并拒绝所有记录（CR-01 修复）
        let mut f = make_feature(true);
        f.include.users = Some(vec!["USER".into()]);
        f.merge_found_trxids(vec![]);
        let trxids = f
            .include
            .trxids
            .as_ref()
            .expect("trxids 应已初始化为 Some（空集合）");
        assert!(trxids.is_empty(), "空列表时集合应为空");
    }

    #[test]
    fn test_merge_found_trxids_adds_to_set() {
        let mut f = make_feature(true);
        f.include.users = Some(vec!["USER".into()]);
        f.merge_found_trxids(vec!["TX1".to_string(), "TX2".to_string()]);
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
        assert!(f.matches(42, 0.0_f32, 0_u32));
        assert!(!f.matches(99, 0.0_f32, 0_u32));
    }

    #[test]
    fn test_indicator_matches_min_runtime() {
        let f = IndicatorFilters {
            exec_ids: None,
            min_runtime_ms: Some(1000),
            min_row_count: None,
        };
        assert!(f.matches(0, 1000.0_f32, 0_u32));
        assert!(f.matches(0, 2000.0_f32, 0_u32));
        assert!(!f.matches(0, 999.0_f32, 0_u32));
    }

    #[test]
    fn test_indicator_matches_min_row_count() {
        let f = IndicatorFilters {
            exec_ids: None,
            min_runtime_ms: None,
            min_row_count: Some(100),
        };
        assert!(f.matches(0, 0.0_f32, 100_u32));
        assert!(!f.matches(0, 0.0_f32, 99_u32));
    }

    #[test]
    fn test_indicator_no_filters_always_false() {
        assert!(!IndicatorFilters::default().matches(1, 9999.0_f32, 9999_u32));
    }
}
