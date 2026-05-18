// `super` here is the `compiled` module (compiled.rs).
use super::super::serde_helpers::TrxidSet;
use super::super::types::{ExcludeFilters, IncludeFilters, RecordMeta, SqlFilters};
use super::{CompiledMetaFilters, CompiledSqlFilters};

fn m<'a>(trxid: &'a str, ip: &'a str, user: &'a str, tag: Option<&'a str>) -> RecordMeta<'a> {
    RecordMeta {
        trxid,
        ip,
        sess: "s",
        thrd: "t",
        user,
        stmt: "st",
        app: "a",
        tag,
    }
}

fn make_compiled_meta(users: Option<Vec<String>>, ips: Option<Vec<String>>) -> CompiledMetaFilters {
    let include = IncludeFilters {
        users,
        ips,
        ..IncludeFilters::default()
    };
    CompiledMetaFilters::try_from_include_exclude(&include, &ExcludeFilters::default()).unwrap()
}

fn make_compiled_with_exclude(
    exclude_users: Option<Vec<String>>,
    exclude_ips: Option<Vec<String>>,
) -> CompiledMetaFilters {
    let exclude = ExcludeFilters {
        users: exclude_users,
        ips: exclude_ips,
        ..ExcludeFilters::default()
    };
    CompiledMetaFilters::try_from_include_exclude(&IncludeFilters::default(), &exclude).unwrap()
}

#[test]
fn test_compiled_meta_unconfigured_passes() {
    let compiled = make_compiled_meta(None, None);
    assert!(compiled.should_keep(&m("tx", "1.2.3.4", "any_user", None)));
}

#[test]
fn test_compiled_meta_and_semantics() {
    let compiled = make_compiled_meta(
        Some(vec!["^admin".to_string()]),
        Some(vec!["^192\\.168".to_string()]),
    );
    assert!(compiled.should_keep(&m("tx", "192.168.1.1", "admin_dba", None)));
    assert!(!compiled.should_keep(&m("tx", "10.0.0.1", "admin_dba", None)));
    assert!(!compiled.should_keep(&m("tx", "192.168.1.1", "sys_user", None)));
}

#[test]
fn test_compiled_meta_single_field_or() {
    let include = IncludeFilters {
        users: Some(vec!["^admin".to_string(), ".*_dba$".to_string()]),
        ..IncludeFilters::default()
    };
    let compiled =
        CompiledMetaFilters::try_from_include_exclude(&include, &ExcludeFilters::default())
            .unwrap();
    assert!(compiled.should_keep(&m("tx", "ip", "admin_user", None)));
    assert!(compiled.should_keep(&m("tx", "ip", "sys_dba", None)));
    assert!(!compiled.should_keep(&m("tx", "ip", "regular_user", None)));
}

#[test]
fn test_compiled_meta_tags_none_rejected() {
    let include = IncludeFilters {
        tags: Some(vec!["^SEL".to_string()]),
        ..IncludeFilters::default()
    };
    let compiled =
        CompiledMetaFilters::try_from_include_exclude(&include, &ExcludeFilters::default())
            .unwrap();
    assert!(!compiled.should_keep(&m("tx", "ip", "user", None)));
    assert!(compiled.should_keep(&m("tx", "ip", "user", Some("SELECT"))));
    assert!(!compiled.should_keep(&m("tx", "ip", "user", Some("INSERT"))));
}

#[test]
fn test_compiled_meta_trxids_and() {
    use compact_str::CompactString;
    let mut trxid_set = TrxidSet::default();
    trxid_set.insert(CompactString::from("TX123"));
    let include = IncludeFilters {
        users: Some(vec!["^admin".to_string()]),
        trxids: Some(trxid_set),
        ..IncludeFilters::default()
    };
    let compiled =
        CompiledMetaFilters::try_from_include_exclude(&include, &ExcludeFilters::default())
            .unwrap();
    assert!(compiled.should_keep(&m("TX123", "ip", "admin_user", None)));
    assert!(!compiled.should_keep(&m("TX999", "ip", "admin_user", None)));
    assert!(!compiled.should_keep(&m("TX123", "ip", "other_user", None)));
}

// ── IncludeFilters / ExcludeFilters has_filters ────────────
#[test]
fn test_t1_meta_has_filters_with_exclude_usernames() {
    let exclude = ExcludeFilters {
        users: Some(vec!["guest".to_string()]),
        ..ExcludeFilters::default()
    };
    assert!(exclude.has_filters());
}

#[test]
fn test_t1_meta_has_filters_all_none_is_false() {
    assert!(!IncludeFilters::default().has_filters());
    assert!(!ExcludeFilters::default().has_filters());
}

#[test]
fn test_t1_compiled_from_meta_exclude_usernames() {
    let exclude = ExcludeFilters {
        users: Some(vec!["^guest".to_string()]),
        ..ExcludeFilters::default()
    };
    let compiled =
        CompiledMetaFilters::try_from_include_exclude(&IncludeFilters::default(), &exclude)
            .unwrap();
    assert!(compiled.exclude_usernames.is_some());
}

#[test]
fn test_t1_compiled_from_meta_exclude_none() {
    let compiled = CompiledMetaFilters::try_from_include_exclude(
        &IncludeFilters::default(),
        &ExcludeFilters::default(),
    )
    .unwrap();
    assert!(compiled.exclude_usernames.is_none());
}

#[test]
fn test_t1_has_any_filters_include_only() {
    let include = IncludeFilters {
        users: Some(vec!["admin".to_string()]),
        ..IncludeFilters::default()
    };
    let compiled =
        CompiledMetaFilters::try_from_include_exclude(&include, &ExcludeFilters::default())
            .unwrap();
    assert_eq!(compiled.has_any_filters(), compiled.has_filters());
}

#[test]
fn test_t1_has_any_filters_exclude_only() {
    let exclude = ExcludeFilters {
        users: Some(vec!["guest".to_string()]),
        ..ExcludeFilters::default()
    };
    let compiled =
        CompiledMetaFilters::try_from_include_exclude(&IncludeFilters::default(), &exclude)
            .unwrap();
    assert!(compiled.has_any_filters());
    assert!(!compiled.has_filters());
}

// ── exclude filters ────────────────────────────────────────
#[test]
fn test_exclude_username_drops_matching_record() {
    let compiled = make_compiled_with_exclude(Some(vec!["^guest".to_string()]), None);
    assert!(!compiled.should_keep(&m("tx", "1.2.3.4", "guest_01", None)));
}

#[test]
fn test_exclude_username_retains_nonmatching_record() {
    let compiled = make_compiled_with_exclude(Some(vec!["^guest".to_string()]), None);
    assert!(compiled.should_keep(&m("tx", "1.2.3.4", "admin_dba", None)));
}

#[test]
fn test_exclude_ip_drops_matching_record() {
    let compiled = make_compiled_with_exclude(None, Some(vec!["^10\\.0".to_string()]));
    assert!(!compiled.should_keep(&m("tx", "10.0.0.1", "user", None)));
    assert!(compiled.should_keep(&m("tx", "192.168.1.1", "user", None)));
}

#[test]
fn test_exclude_or_veto_semantics() {
    // Any hit drops the record; no hit passes it.
    let compiled = make_compiled_with_exclude(
        Some(vec!["guest".to_string()]),
        Some(vec!["^10".to_string()]),
    );
    assert!(!compiled.should_keep(&m("tx", "10.0.0.1", "admin", None))); // ip hit
    assert!(compiled.should_keep(&m("tx", "192.168.1.1", "admin", None))); // no hit
}

#[test]
fn test_exclude_with_include_interaction() {
    // exclude veto wins even when include would pass; include miss also drops; both pass is ok.
    let include = IncludeFilters {
        users: Some(vec!["^admin".to_string()]),
        ..IncludeFilters::default()
    };
    let exclude = ExcludeFilters {
        ips: Some(vec!["^10".to_string()]),
        ..ExcludeFilters::default()
    };
    let compiled = CompiledMetaFilters::try_from_include_exclude(&include, &exclude).unwrap();
    assert!(!compiled.should_keep(&m("tx", "10.0.0.1", "admin", None))); // exclude veto wins
    assert!(compiled.should_keep(&m("tx", "192.168.1.1", "admin", None))); // both pass
    assert!(!compiled.should_keep(&m("tx", "192.168.1.1", "sys_user", None))); // include fails
}

#[test]
fn test_exclude_tags_behavior() {
    // Matching tag is dropped; no-tag is retained; non-matching tag is retained.
    let exclude = ExcludeFilters {
        tags: Some(vec!["^SEL".to_string()]),
        ..ExcludeFilters::default()
    };
    let compiled =
        CompiledMetaFilters::try_from_include_exclude(&IncludeFilters::default(), &exclude)
            .unwrap();
    assert!(!compiled.should_keep(&m("tx", "ip", "user", Some("SELECT"))));
    assert!(compiled.should_keep(&m("tx", "ip", "user", None)));
    assert!(compiled.should_keep(&m("tx", "ip", "user", Some("INSERT"))));
}

#[test]
fn test_exclude_invalid_regex_validate_fails() {
    let exclude = ExcludeFilters {
        users: Some(vec!["[invalid".to_string()]),
        ..ExcludeFilters::default()
    };
    let result =
        CompiledMetaFilters::try_from_include_exclude(&IncludeFilters::default(), &exclude);
    assert!(result.is_err());
}

// ── CompiledSqlFilters ─────────────────────────────────────
#[test]
fn test_compiled_sql_include_regex() {
    let sf = SqlFilters {
        includes: Some(vec!["^SELECT".to_string()]),
        excludes: None,
    };
    let compiled = CompiledSqlFilters::try_from_sql_filters(&sf).unwrap();
    assert!(compiled.matches("SELECT * FROM t"));
    assert!(!compiled.matches("INSERT INTO t VALUES (1)"));
}

#[test]
fn test_compiled_sql_exclude_regex() {
    let sf = SqlFilters {
        includes: None,
        excludes: Some(vec!["DROP".to_string()]),
    };
    let compiled = CompiledSqlFilters::try_from_sql_filters(&sf).unwrap();
    assert!(compiled.matches("SELECT 1"));
    assert!(!compiled.matches("DROP TABLE t"));
}
