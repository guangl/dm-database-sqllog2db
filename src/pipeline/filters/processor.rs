use crate::config::Config;
use crate::pipeline::{FiltersFeature, LogProcessor, Pipeline};
use dm_database_parser_sqllog::{Filter, FilterBuilder, Sqllog};
use std::collections::HashSet;

pub(crate) fn build_pipeline(cfg: &Config) -> Pipeline {
    let mut pipeline = Pipeline::new();
    if let Some(f) = cfg.filter.as_ref() {
        if f.enable && (f.include.has_filters() || f.exclude.has_filters()) {
            pipeline.add(Box::new(FilterProcessor::from_feature(f)));
        }
    }
    pipeline
}

struct FilterProcessor {
    base_filter: Filter,
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
        // 语句类型 (INS/UPD/DEL/SEL/ORA...) 存于记录的 `tag` 字段，而非 `stmt` 句柄，
        // 因此按类型过滤须匹配 tag。
        build_or_group(f.include.statements.as_deref(), |fb, v| fb.tag_eq(v)),
        build_or_group(f.include.apps.as_deref(), |fb, v| fb.appname_eq(v)),
        build_or_group(f.include.tags.as_deref(), |fb, v| fb.tag_eq(v)),
    ]
}

fn build_exclude_groups(f: &FiltersFeature) -> Vec<Vec<Filter>> {
    vec![
        build_or_group(f.exclude.users.as_deref(), |fb, v| fb.username_eq(v)),
        build_or_group(f.exclude.ips.as_deref(), |fb, v| fb.client_ip_eq(v)),
        build_or_group(f.exclude.sessions.as_deref(), |fb, v| fb.sess_id_eq(v)),
        build_or_group(f.exclude.threads.as_deref(), |fb, v| fb.thrd_id_eq(v)),
        // 见 build_include_groups：语句类型匹配 tag 字段。
        build_or_group(f.exclude.statements.as_deref(), |fb, v| fb.tag_eq(v)),
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
        if !self.base_filter.matches(record) {
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
mod tests {
    use super::*;
    use crate::pipeline::filters::types::{ExcludeFilters, FiltersFeature, IncludeFilters};

    fn make_record(username: &str, client_ip: &str, trxid: &str, tag: Option<&str>) -> Sqllog {
        Sqllog {
            ts: "2024-01-01 00:00:00.000".to_string(),
            tag: tag.map(String::from),
            ep: 2,
            sess_id: "s".to_string(),
            thrd_id: "t".to_string(),
            username: username.to_string(),
            trxid: trxid.to_string(),
            statement: "st".to_string(),
            appname: "app".to_string(),
            client_ip: client_ip.to_string(),
            sql: "SELECT 1".to_string(),
            exectime: 100.0,
            rowcount: 1,
            exec_id: 1,
        }
    }

    fn make_feature(include: IncludeFilters, exclude: ExcludeFilters) -> FiltersFeature {
        FiltersFeature {
            enable: true,
            include,
            exclude,
            ..FiltersFeature::default()
        }
    }

    #[test]
    fn test_no_filters_passes_all() {
        let proc = FilterProcessor::from_feature(&make_feature(
            IncludeFilters::default(),
            ExcludeFilters::default(),
        ));
        assert!(proc.process_with_meta(&make_record("any", "1.2.3.4", "tx", None)));
    }

    #[test]
    fn test_include_username_passes_matching() {
        let include = IncludeFilters {
            users: Some(vec!["admin_dba".into()]),
            ..Default::default()
        };
        let proc = FilterProcessor::from_feature(&make_feature(include, ExcludeFilters::default()));
        assert!(proc.process_with_meta(&make_record("admin_dba", "1.2.3.4", "tx", None)));
        assert!(!proc.process_with_meta(&make_record("guest_01", "1.2.3.4", "tx", None)));
    }

    #[test]
    fn test_include_username_or_semantics() {
        let include = IncludeFilters {
            users: Some(vec!["admin_user".into(), "sys_dba".into()]),
            ..Default::default()
        };
        let proc = FilterProcessor::from_feature(&make_feature(include, ExcludeFilters::default()));
        assert!(proc.process_with_meta(&make_record("admin_user", "ip", "tx", None)));
        assert!(proc.process_with_meta(&make_record("sys_dba", "ip", "tx", None)));
        assert!(!proc.process_with_meta(&make_record("regular_user", "ip", "tx", None)));
    }

    #[test]
    fn test_include_and_semantics_between_fields() {
        let include = IncludeFilters {
            users: Some(vec!["admin_dba".into()]),
            ips: Some(vec!["192.168.1.1".into()]),
            ..Default::default()
        };
        let proc = FilterProcessor::from_feature(&make_feature(include, ExcludeFilters::default()));
        assert!(proc.process_with_meta(&make_record("admin_dba", "192.168.1.1", "tx", None)));
        assert!(!proc.process_with_meta(&make_record("admin_dba", "10.0.0.1", "tx", None)));
        assert!(!proc.process_with_meta(&make_record("sys_user", "192.168.1.1", "tx", None)));
    }

    #[test]
    fn test_include_tag_none_rejected() {
        let include = IncludeFilters {
            tags: Some(vec!["SEL".into()]),
            ..Default::default()
        };
        let proc = FilterProcessor::from_feature(&make_feature(include, ExcludeFilters::default()));
        assert!(!proc.process_with_meta(&make_record("u", "ip", "tx", None)));
        assert!(proc.process_with_meta(&make_record("u", "ip", "tx", Some("SEL"))));
        assert!(!proc.process_with_meta(&make_record("u", "ip", "tx", Some("INS"))));
    }

    #[test]
    fn test_include_trxid_and_semantics() {
        use crate::pipeline::filters::TrxidSet;
        let trxids: TrxidSet = ["TX123".to_string()].into_iter().collect();
        let include = IncludeFilters {
            users: Some(vec!["admin_user".into()]),
            trxids: Some(trxids),
            ..Default::default()
        };
        let proc = FilterProcessor::from_feature(&make_feature(include, ExcludeFilters::default()));
        assert!(proc.process_with_meta(&make_record("admin_user", "ip", "TX123", None)));
        assert!(!proc.process_with_meta(&make_record("admin_user", "ip", "TX999", None)));
        assert!(!proc.process_with_meta(&make_record("other_user", "ip", "TX123", None)));
    }

    #[test]
    fn test_exclude_drops_matching_record() {
        let exclude = ExcludeFilters {
            users: Some(vec!["guest_01".into()]),
            ..Default::default()
        };
        let proc = FilterProcessor::from_feature(&make_feature(IncludeFilters::default(), exclude));
        assert!(!proc.process_with_meta(&make_record("guest_01", "1.2.3.4", "tx", None)));
        assert!(proc.process_with_meta(&make_record("admin_dba", "1.2.3.4", "tx", None)));
    }

    #[test]
    fn test_exclude_or_veto_any_hit_drops() {
        let exclude = ExcludeFilters {
            users: Some(vec!["guest".into()]),
            ips: Some(vec!["10.0.0.1".into()]),
            ..Default::default()
        };
        let proc = FilterProcessor::from_feature(&make_feature(IncludeFilters::default(), exclude));
        assert!(!proc.process_with_meta(&make_record("admin", "10.0.0.1", "tx", None)));
        assert!(proc.process_with_meta(&make_record("admin", "192.168.1.1", "tx", None)));
    }

    #[test]
    fn test_exclude_tag_none_retained() {
        let exclude = ExcludeFilters {
            tags: Some(vec!["SEL".into()]),
            ..Default::default()
        };
        let proc = FilterProcessor::from_feature(&make_feature(IncludeFilters::default(), exclude));
        assert!(!proc.process_with_meta(&make_record("u", "ip", "tx", Some("SEL"))));
        assert!(proc.process_with_meta(&make_record("u", "ip", "tx", None)));
        assert!(proc.process_with_meta(&make_record("u", "ip", "tx", Some("INS"))));
    }

    #[test]
    fn test_exclude_veto_wins_over_include() {
        let include = IncludeFilters {
            users: Some(vec!["admin".into()]),
            ..Default::default()
        };
        let exclude = ExcludeFilters {
            ips: Some(vec!["10.0.0.1".into()]),
            ..Default::default()
        };
        let proc = FilterProcessor::from_feature(&make_feature(include, exclude));
        assert!(!proc.process_with_meta(&make_record("admin", "10.0.0.1", "tx", None)));
        assert!(proc.process_with_meta(&make_record("admin", "192.168.1.1", "tx", None)));
        assert!(!proc.process_with_meta(&make_record("sys_user", "192.168.1.1", "tx", None)));
    }

    #[test]
    fn test_timestamp_range_filter() {
        let include = IncludeFilters {
            start_ts: Some("2024-06-01".into()),
            end_ts: Some("2024-06-30".into()),
            ..Default::default()
        };
        let proc = FilterProcessor::from_feature(&make_feature(include, ExcludeFilters::default()));
        let mut record = make_record("u", "ip", "tx", None);
        record.ts = "2024-06-15 10:00:00.000".to_string();
        assert!(proc.process_with_meta(&record));
        record.ts = "2024-05-31 23:59:59.999".to_string();
        assert!(!proc.process_with_meta(&record));
        record.ts = "2024-07-01 00:00:00.000".to_string();
        assert!(!proc.process_with_meta(&record));
    }

    #[test]
    fn test_include_session_filter() {
        let include = IncludeFilters {
            sessions: Some(vec!["s".into()]),
            ..Default::default()
        };
        let proc = FilterProcessor::from_feature(&make_feature(include, ExcludeFilters::default()));
        assert!(proc.process_with_meta(&make_record("u", "ip", "tx", None)));
        let mut other = make_record("u", "ip", "tx", None);
        other.sess_id = "other_session".to_string();
        assert!(!proc.process_with_meta(&other));
    }

    #[test]
    fn test_include_app_filter() {
        let include = IncludeFilters {
            apps: Some(vec!["app".into()]),
            ..Default::default()
        };
        let proc = FilterProcessor::from_feature(&make_feature(include, ExcludeFilters::default()));
        assert!(proc.process_with_meta(&make_record("u", "ip", "tx", None)));
        let mut other = make_record("u", "ip", "tx", None);
        other.appname = "other_app".to_string();
        assert!(!proc.process_with_meta(&other));
    }

    #[test]
    fn test_include_statement_filter() {
        // 语句类型过滤匹配记录的 `tag` 字段（INS/UPD/DEL/SEL/ORA...），而非 `stmt` 句柄。
        let include = IncludeFilters {
            statements: Some(vec!["SEL".into()]),
            ..Default::default()
        };
        let proc = FilterProcessor::from_feature(&make_feature(include, ExcludeFilters::default()));
        assert!(proc.process_with_meta(&make_record("u", "ip", "tx", Some("SEL"))));
        // tag 不匹配则拒绝
        assert!(!proc.process_with_meta(&make_record("u", "ip", "tx", Some("INS"))));
        // 无 tag 的记录（如 TRX: START）不匹配任何语句类型
        assert!(!proc.process_with_meta(&make_record("u", "ip", "tx", None)));
    }

    #[test]
    fn test_include_thread_filter() {
        let include = IncludeFilters {
            threads: Some(vec!["t".into()]),
            ..Default::default()
        };
        let proc = FilterProcessor::from_feature(&make_feature(include, ExcludeFilters::default()));
        assert!(proc.process_with_meta(&make_record("u", "ip", "tx", None)));
        let mut other = make_record("u", "ip", "tx", None);
        other.thrd_id = "other_thread".to_string();
        assert!(!proc.process_with_meta(&other));
    }

    #[test]
    fn test_debug_format() {
        let proc = FilterProcessor::from_feature(&make_feature(
            IncludeFilters::default(),
            ExcludeFilters::default(),
        ));
        let debug_str = format!("{proc:?}");
        assert!(debug_str.contains("FilterProcessor"));
    }

    // ── build_pipeline 在 indicators/sql 过滤器下的行为测试（WR-02）──────────────

    /// 验证：trxids 为空集合（预扫描无命中的 sentinel）时，
    /// `FilterProcessor` 仍进入 pipeline 并拒绝所有记录。
    #[test]
    fn test_empty_trxid_sentinel_rejects_all_records() {
        use crate::pipeline::filters::TrxidSet;
        // 模拟 merge_found_trxids 运行后 trxids 被初始化为空集合（预扫描无命中）
        let empty_trxids: TrxidSet = TrxidSet::default();
        let include = IncludeFilters {
            trxids: Some(empty_trxids),
            ..Default::default()
        };
        let proc = FilterProcessor::from_feature(&make_feature(include, ExcludeFilters::default()));
        // 空 trxids sentinel 应拒绝所有记录，而不是放行所有记录
        assert!(
            !proc.process_with_meta(&make_record("any_user", "1.2.3.4", "TX001", None)),
            "trxids 为空集合时应拒绝所有记录"
        );
        assert!(
            !proc.process_with_meta(&make_record("other", "10.0.0.1", "TX999", None)),
            "trxids 为空集合时应拒绝任意 trxid 的记录"
        );
    }

    /// 验证：trxids 非空时，仅匹配 trxid 的记录通过过滤器。
    #[test]
    fn test_nonempty_trxid_set_filters_correctly() {
        use crate::pipeline::filters::TrxidSet;
        let mut trxids: TrxidSet = TrxidSet::default();
        trxids.insert("TX_MATCH".to_string());
        let include = IncludeFilters {
            trxids: Some(trxids),
            ..Default::default()
        };
        let proc = FilterProcessor::from_feature(&make_feature(include, ExcludeFilters::default()));
        assert!(
            proc.process_with_meta(&make_record("any_user", "1.2.3.4", "TX_MATCH", None)),
            "trxid 匹配时应通过过滤器"
        );
        assert!(
            !proc.process_with_meta(&make_record("any_user", "1.2.3.4", "TX_OTHER", None)),
            "trxid 不匹配时应被过滤"
        );
    }
}
