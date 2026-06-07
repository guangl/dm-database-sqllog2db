use super::types::{ExcludeFilters, FiltersFeature, IncludeFilters, IndicatorFilters, SqlFilters};

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
