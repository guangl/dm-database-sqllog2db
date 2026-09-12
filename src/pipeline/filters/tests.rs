use super::types::FiltersFeature;

// ── has_filters ────────────────────────────────────────────

#[test]
fn test_has_filters_empty() {
    assert!(!FiltersFeature::default().has_filters());
}

#[test]
fn test_has_filters_with_username() {
    let mut f = FiltersFeature::default();
    f.include.users = Some(vec!["USER".into()]);
    assert!(f.has_filters());
}

#[test]
fn test_has_filters_with_start_ts() {
    let mut f = FiltersFeature::default();
    f.include.start_ts = Some("2025-01-01".into());
    assert!(f.has_filters());
}

#[test]
fn test_has_filters_with_indicator() {
    let mut f = FiltersFeature::default();
    f.include.min_runtime_ms = Some(1000.0);
    assert!(f.has_filters());
}

// ── has_transaction_filters ────────────────────────────────

#[test]
fn test_has_transaction_filters_no_indicators() {
    let mut f = FiltersFeature::default();
    f.include.users = Some(vec!["USER".into()]);
    assert!(!f.has_transaction_filters());
}

#[test]
fn test_has_transaction_filters_with_min_runtime() {
    let mut f = FiltersFeature::default();
    f.include.min_runtime_ms = Some(500.0);
    assert!(f.has_transaction_filters());
}

#[test]
fn test_has_transaction_filters_with_exec_ids() {
    let mut f = FiltersFeature::default();
    f.include.exec_ids = Some([1_i64, 2, 3].into_iter().collect());
    assert!(f.has_transaction_filters());
}

// ── merge_found_trxids ─────────────────────────────────────
#[test]
fn test_merge_found_trxids_empty_list_initializes_sentinel() {
    // 空列表时仍应初始化空集合（sentinel），使 has_filters() 返回 true，
    // 确保预扫描无命中时 FilterProcessor 进入 pipeline 并拒绝所有记录（CR-01 修复）
    let mut f = FiltersFeature::default();
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
    let mut f = FiltersFeature::default();
    f.include.users = Some(vec!["USER".into()]);
    f.merge_found_trxids(vec!["TX1".to_string(), "TX2".to_string()]);
    let trxids = f.include.trxids.unwrap();
    assert!(trxids.contains("TX1"));
    assert!(trxids.contains("TX2"));
}
