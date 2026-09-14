use super::*;
use dm_database_parser_sqllog::Sqllog;

fn make_record(sql: &str, exectime: f32, ts: &str) -> Sqllog {
    Sqllog {
        sql: sql.to_string(),
        exectime,
        ts: ts.to_string(),
        tag: Some("ORA".to_string()),
        ..Sqllog::default()
    }
}

#[test]
fn test_slow_sql_top_n_limit() {
    let mut acc = StatsAccumulator::new(3, None, None);
    acc.update(&make_record("SELECT 1", 10.0, "2025-01-01"));
    acc.update(&make_record("SELECT 2", 50.0, "2025-01-02"));
    acc.update(&make_record("SELECT 3", 30.0, "2025-01-03"));
    acc.update(&make_record("SELECT 4", 20.0, "2025-01-04"));
    acc.update(&make_record("SELECT 5", 40.0, "2025-01-05"));

    let (slow, _) = acc.into_results();
    assert_eq!(slow.len(), 3);
    // 降序：50, 40, 30
    assert_eq!(slow[0].elapsed_ms, 50);
    assert_eq!(slow[1].elapsed_ms, 40);
    assert_eq!(slow[2].elapsed_ms, 30);
}

#[test]
fn test_slow_sql_includes_zero_and_negative_elapsed() {
    let mut acc = StatsAccumulator::new(5, None, None);
    acc.update(&make_record("SELECT A", 0.0, "2025-01-01"));
    acc.update(&make_record("SELECT B", -1.0, "2025-01-02"));
    acc.update(&make_record("SELECT C", 5.0, "2025-01-03"));

    let (slow, _) = acc.into_results();
    assert_eq!(slow.len(), 3, "all 3 records should be included (D-12)");
}

#[test]
fn test_frequent_sql_aggregation() {
    let mut acc = StatsAccumulator::new(10, None, None);
    // 同一模板 3 条
    acc.update(&make_record(
        "SELECT id FROM t WHERE id = 1",
        1.0,
        "2025-01-01",
    ));
    acc.update(&make_record(
        "SELECT id FROM t WHERE id = 2",
        2.0,
        "2025-01-02",
    ));
    acc.update(&make_record(
        "SELECT id FROM t WHERE id = 3",
        3.0,
        "2025-01-03",
    ));
    // 不同模板 1 条
    acc.update(&make_record("INSERT INTO t VALUES (1)", 5.0, "2025-01-04"));

    let (_, frequent) = acc.into_results();
    // 找到 call_count == 3 的那条
    let target = frequent.iter().find(|r| r.call_count == 3);
    assert!(target.is_some(), "should find 3-call entry");
    let target = target.unwrap();
    assert_eq!(target.avg_elapsed_ms, 2, "avg of 1+2+3 = 2ms");
    assert_eq!(target.max_elapsed_ms, 3, "max of 1,2,3 = 3ms");
}

#[test]
fn test_frequent_sql_top_n_limit_and_sort() {
    let mut acc = StatsAccumulator::new(3, None, None);
    for count in 1..=5u64 {
        let sql = format!("SELECT * FROM t{count}");
        for _ in 0..count {
            acc.update(&make_record(&sql, 1.0, "2025-01-01"));
        }
    }

    let (_, frequent) = acc.into_results();
    assert_eq!(frequent.len(), 3);
    assert_eq!(frequent[0].call_count, 5);
    assert_eq!(frequent[1].call_count, 4);
    assert_eq!(frequent[2].call_count, 3);
}

#[test]
fn test_slow_entry_total_cmp_handles_equal_elapsed() {
    let mut acc = StatsAccumulator::new(1, None, None);
    acc.update(&make_record("SELECT X", 5.0, "2025-01-01"));
    acc.update(&make_record("SELECT Y", 5.0, "2025-01-02"));
    // 不应 panic，结果稳定
    let (slow, _) = acc.into_results();
    assert_eq!(slow.len(), 1);
}

#[test]
fn test_into_results_when_records_fewer_than_top_n() {
    let mut acc = StatsAccumulator::new(5, None, None);
    // 使用结构不同的 SQL，normalize_sql 后 key 不同，保证 frequent 有 2 条
    acc.update(&make_record("SELECT id FROM users", 10.0, "2025-01-01"));
    acc.update(&make_record(
        "INSERT INTO orders VALUES (1)",
        20.0,
        "2025-01-02",
    ));

    let (slow, frequent) = acc.into_results();
    assert_eq!(slow.len(), 2, "D-11: output only actual count");
    assert_eq!(frequent.len(), 2, "D-11: output only actual count");
}

#[test]
fn test_filter_both_from_and_to_excludes_outside_records() {
    let mut acc = StatsAccumulator::new(
        10,
        Some("2024-01-15".to_string()),
        Some("2024-01-15".to_string()),
    );
    acc.update(&make_record("SELECT 1", 1.0, "2024-01-14 10:00:00"));
    acc.update(&make_record("SELECT 2", 2.0, "2024-01-15 00:00:00"));
    acc.update(&make_record("SELECT 3", 3.0, "2024-01-15 23:59:59"));
    acc.update(&make_record("SELECT 4", 4.0, "2024-01-16 10:00:00"));
    let (slow, _) = acc.into_results();
    assert_eq!(
        slow.len(),
        2,
        "only records on 2024-01-15 should be included"
    );
}

#[test]
fn test_filter_from_only_excludes_earlier_records() {
    let mut acc = StatsAccumulator::new(10, Some("2024-01-15".to_string()), None);
    acc.update(&make_record("SELECT A", 1.0, "2024-01-14"));
    acc.update(&make_record("SELECT B", 2.0, "2024-01-15"));
    acc.update(&make_record("SELECT C", 3.0, "2024-01-20"));
    let (slow, _) = acc.into_results();
    assert_eq!(
        slow.len(),
        2,
        "records on/after 2024-01-15 should be included"
    );
}

#[test]
fn test_filter_to_only_excludes_later_records() {
    let mut acc = StatsAccumulator::new(10, None, Some("2024-01-15".to_string()));
    acc.update(&make_record("SELECT A", 1.0, "2024-01-10"));
    acc.update(&make_record("SELECT B", 2.0, "2024-01-15"));
    acc.update(&make_record("SELECT C", 3.0, "2024-01-16"));
    let (slow, _) = acc.into_results();
    assert_eq!(
        slow.len(),
        2,
        "records on/before 2024-01-15 should be included"
    );
}

#[test]
fn test_filter_none_behavior_unchanged() {
    let mut acc = StatsAccumulator::new(3, None, None);
    acc.update(&make_record("SELECT 1", 10.0, "2025-01-01"));
    acc.update(&make_record("SELECT 2", 50.0, "2025-01-02"));
    acc.update(&make_record("SELECT 3", 30.0, "2025-01-03"));
    acc.update(&make_record("SELECT 4", 20.0, "2025-01-04"));
    acc.update(&make_record("SELECT 5", 40.0, "2025-01-05"));
    let (slow, _) = acc.into_results();
    assert_eq!(slow.len(), 3);
    assert_eq!(slow[0].elapsed_ms, 50);
    assert_eq!(slow[1].elapsed_ms, 40);
    assert_eq!(slow[2].elapsed_ms, 30);
}

#[test]
fn test_non_ora_records_are_skipped() {
    let mut acc = StatsAccumulator::new(10, None, None);
    // 无 tag 的记录应被跳过
    acc.update(&Sqllog {
        sql: "SELECT 1".to_string(),
        exectime: 10.0,
        ts: "2025-01-01".to_string(),
        tag: None,
        ..Sqllog::default()
    });
    // SEL tag 的记录也应被跳过
    acc.update(&Sqllog {
        sql: "SELECT 2".to_string(),
        exectime: 20.0,
        ts: "2025-01-02".to_string(),
        tag: Some("SEL".to_string()),
        ..Sqllog::default()
    });
    let (slow, freq) = acc.into_results();
    assert_eq!(slow.len(), 0, "non-ORA records should be skipped");
    assert_eq!(freq.len(), 0, "non-ORA records should be skipped");
}

#[test]
fn test_filter_ts_too_short_treated_as_out_of_range() {
    let mut acc = StatsAccumulator::new(10, Some("2024-01-15 10:00:00".to_string()), None);
    // ts is only 10 bytes, from is 19 bytes — length guard must fire
    acc.update(&make_record("SELECT X", 1.0, "2024-01-15"));
    let (slow, _) = acc.into_results();
    assert_eq!(
        slow.len(),
        0,
        "ts too short should be treated as out of range"
    );
}
