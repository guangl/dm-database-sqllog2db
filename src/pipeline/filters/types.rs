use serde::{Deserialize, Deserializer};

use super::serde_helpers::{TrxidSet, vec_to_hashset, vec_to_i64_hashset};

/// 包含过滤器 (include 子表字段)
#[derive(Debug, Deserialize, Clone, Default)]
pub struct IncludeFilters {
    #[serde(default)]
    pub users: Option<Vec<String>>,
    #[serde(default)]
    pub ips: Option<Vec<String>>,
    #[serde(default)]
    pub sessions: Option<Vec<String>>,
    #[serde(default)]
    pub threads: Option<Vec<String>>,
    #[serde(default)]
    pub statements: Option<Vec<String>>,
    #[serde(default)]
    pub apps: Option<Vec<String>>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub start_ts: Option<String>,
    #[serde(default)]
    pub end_ts: Option<String>,
    #[serde(default, deserialize_with = "vec_to_hashset")]
    pub trxids: Option<TrxidSet>,
}

impl IncludeFilters {
    #[must_use]
    pub fn has_filters(&self) -> bool {
        self.users.as_ref().is_some_and(|v| !v.is_empty())
            || self.ips.as_ref().is_some_and(|v| !v.is_empty())
            || self.sessions.as_ref().is_some_and(|v| !v.is_empty())
            || self.threads.as_ref().is_some_and(|v| !v.is_empty())
            || self.statements.as_ref().is_some_and(|v| !v.is_empty())
            || self.apps.as_ref().is_some_and(|v| !v.is_empty())
            || self.tags.as_ref().is_some_and(|v| !v.is_empty())
            || self.start_ts.is_some()
            || self.end_ts.is_some()
            || self.trxids.is_some()
    }
}

/// 排除过滤器 (exclude 子表字段)
#[derive(Debug, Deserialize, Clone, Default)]
pub struct ExcludeFilters {
    #[serde(default)]
    pub users: Option<Vec<String>>,
    #[serde(default)]
    pub ips: Option<Vec<String>>,
    #[serde(default)]
    pub sessions: Option<Vec<String>>,
    #[serde(default)]
    pub threads: Option<Vec<String>>,
    #[serde(default)]
    pub statements: Option<Vec<String>>,
    #[serde(default)]
    pub apps: Option<Vec<String>>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

impl ExcludeFilters {
    #[must_use]
    pub fn has_filters(&self) -> bool {
        self.users.as_ref().is_some_and(|v| !v.is_empty())
            || self.ips.as_ref().is_some_and(|v| !v.is_empty())
            || self.sessions.as_ref().is_some_and(|v| !v.is_empty())
            || self.threads.as_ref().is_some_and(|v| !v.is_empty())
            || self.statements.as_ref().is_some_and(|v| !v.is_empty())
            || self.apps.as_ref().is_some_and(|v| !v.is_empty())
            || self.tags.as_ref().is_some_and(|v| !v.is_empty())
    }
}

/// 过滤器配置（手写 Deserialize，支持新格式嵌套子表和旧格式扁平字段向后兼容）
// pub: required by integration tests (directly constructs FiltersFeature and accesses cfg.filter)
#[derive(Debug, Clone, Default)]
pub struct FiltersFeature {
    /// 是否启用过滤器
    pub enable: bool,
    /// 包含过滤条件子表
    pub include: IncludeFilters,
    /// 排除过滤条件子表
    pub exclude: ExcludeFilters,
    /// 指标过滤器 (事务级: 命中即保留整笔事务 - 需要预扫描)
    pub indicators: IndicatorFilters,
    /// SQL 内容过滤器 (事务级: 预扫描阶段匹配 SQL，保留整笔事务)
    pub sql: SqlFilters,
}

/// 中间反序列化结构体（私有），同时接受新格式子表和旧格式扁平字段
#[derive(Debug, Deserialize)]
struct RawFiltersFeature {
    #[serde(default)]
    enable: bool,
    // 新格式子表（优先）
    #[serde(default)]
    include: Option<IncludeFilters>,
    #[serde(default)]
    exclude: Option<ExcludeFilters>,
    #[serde(default)]
    indicators: IndicatorFilters,
    #[serde(default)]
    sql: SqlFilters,
    // 旧格式扁平字段（向后兼容）— include 类
    #[serde(default)]
    usernames: Option<Vec<String>>,
    #[serde(default)]
    client_ips: Option<Vec<String>>,
    #[serde(default)]
    sess_ids: Option<Vec<String>>,
    #[serde(default)]
    thrd_ids: Option<Vec<String>>,
    #[serde(default)]
    statements: Option<Vec<String>>,
    #[serde(default)]
    appnames: Option<Vec<String>>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    start_ts: Option<String>,
    #[serde(default)]
    end_ts: Option<String>,
    #[serde(default, deserialize_with = "vec_to_hashset")]
    trxids: Option<TrxidSet>,
    // 旧格式扁平字段（向后兼容）— exclude 类
    #[serde(default)]
    exclude_usernames: Option<Vec<String>>,
    #[serde(default)]
    exclude_client_ips: Option<Vec<String>>,
    #[serde(default)]
    exclude_sess_ids: Option<Vec<String>>,
    #[serde(default)]
    exclude_thrd_ids: Option<Vec<String>>,
    #[serde(default)]
    exclude_statements: Option<Vec<String>>,
    #[serde(default)]
    exclude_appnames: Option<Vec<String>>,
    #[serde(default)]
    exclude_tags: Option<Vec<String>>,
}

impl<'de> Deserialize<'de> for FiltersFeature {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = RawFiltersFeature::deserialize(d)?;
        Ok(FiltersFeature::from(raw))
    }
}

impl From<RawFiltersFeature> for FiltersFeature {
    fn from(raw: RawFiltersFeature) -> Self {
        // 检测混合格式：新子表与旧扁平字段同时存在时发出警告
        let flat_include_present = raw.usernames.is_some()
            || raw.client_ips.is_some()
            || raw.sess_ids.is_some()
            || raw.thrd_ids.is_some()
            || raw.statements.is_some()
            || raw.appnames.is_some()
            || raw.tags.is_some()
            || raw.start_ts.is_some()
            || raw.end_ts.is_some()
            || raw.trxids.is_some();

        if raw.include.is_some() && flat_include_present {
            log::warn!(
                "[filter] contains both a `[filter.include]` sub-table \
                 and legacy flat fields (e.g. `usernames`). The sub-table takes priority; \
                 flat fields are ignored. Remove the legacy keys to suppress this warning."
            );
        }

        let flat_exclude_present = raw.exclude_usernames.is_some()
            || raw.exclude_client_ips.is_some()
            || raw.exclude_sess_ids.is_some()
            || raw.exclude_thrd_ids.is_some()
            || raw.exclude_statements.is_some()
            || raw.exclude_appnames.is_some()
            || raw.exclude_tags.is_some();

        if raw.exclude.is_some() && flat_exclude_present {
            log::warn!(
                "[filter] contains both a `[filter.exclude]` sub-table \
                 and legacy flat exclude fields (e.g. `exclude_usernames`). The sub-table takes priority; \
                 flat fields are ignored. Remove the legacy keys to suppress this warning."
            );
        }

        // 新格式优先：有 include 子表则用新格式，否则从旧扁平字段构造
        let include = raw.include.unwrap_or(IncludeFilters {
            users: raw.usernames,
            ips: raw.client_ips,
            sessions: raw.sess_ids,
            threads: raw.thrd_ids,
            statements: raw.statements,
            apps: raw.appnames,
            tags: raw.tags,
            start_ts: raw.start_ts,
            end_ts: raw.end_ts,
            trxids: raw.trxids,
        });
        let exclude = raw.exclude.unwrap_or(ExcludeFilters {
            users: raw.exclude_usernames,
            ips: raw.exclude_client_ips,
            sessions: raw.exclude_sess_ids,
            threads: raw.exclude_thrd_ids,
            statements: raw.exclude_statements,
            apps: raw.exclude_appnames,
            tags: raw.exclude_tags,
        });
        FiltersFeature {
            enable: raw.enable,
            include,
            exclude,
            indicators: raw.indicators,
            sql: raw.sql,
        }
    }
}

/// 指标过滤器 (Transaction-level)
#[derive(Debug, Deserialize, Clone, Default)]
pub struct IndicatorFilters {
    /// 使用 `HashSet<i64>` 代替 `Vec<i64>`，将 `matches()` 热路径中的
    /// `.contains()` 从 O(n) 降为 O(1)。
    #[serde(default, deserialize_with = "vec_to_i64_hashset")]
    pub exec_ids: Option<std::collections::HashSet<i64>>,
    pub min_runtime_ms: Option<u32>,
    pub min_row_count: Option<u32>,
}

/// SQL 过滤器（事务级预扫描阶段）。
///
/// `includes` / `excludes` 使用字面量子串匹配（`str::contains`），不支持正则表达式。
#[derive(Debug, Deserialize, Clone, Default)]
pub struct SqlFilters {
    /// 旧字段名 `include_patterns` 通过 alias 向后兼容。
    #[serde(default, alias = "include_patterns")]
    pub includes: Option<Vec<String>>,
    /// 旧字段名 `exclude_patterns` 通过 alias 向后兼容。
    #[serde(default, alias = "exclude_patterns")]
    pub excludes: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 用于触发 `FiltersFeature` 自定义 Deserialize 的包装结构，
    /// 将 `[filter]` 节反序列化为 `FiltersFeature`（绕过顶层 TOML 包装要求）。
    #[derive(serde::Deserialize)]
    struct FilterWrapper {
        filter: FiltersFeature,
    }

    // ── serde_helpers::vec_to_hashset ──────────────────────────

    #[test]
    fn test_trxids_deserialized_to_hashset() {
        let toml = r#"
            [filter]
            enable = true
            trxids = ["TX001", "TX002"]
        "#;
        let w: FilterWrapper = toml::from_str(toml).unwrap();
        let trxids = w.filter.include.trxids.unwrap();
        assert!(trxids.contains("TX001"), "trxids should contain TX001");
        assert!(trxids.contains("TX002"), "trxids should contain TX002");
        assert_eq!(trxids.len(), 2, "trxids len should be 2");
    }

    #[test]
    fn test_trxids_absent_returns_none() {
        let toml = "[filter]\nenable = true\n";
        let w: FilterWrapper = toml::from_str(toml).unwrap();
        assert!(
            w.filter.include.trxids.is_none(),
            "trxids should be None when absent"
        );
    }

    // ── serde_helpers::vec_to_i64_hashset ─────────────────────

    #[test]
    fn test_exec_ids_deserialized_to_hashset() {
        let toml = "[filter]\nenable = true\n[filter.indicators]\nexec_ids = [1, 2, 42]\n";
        let w: FilterWrapper = toml::from_str(toml).unwrap();
        let ids = w.filter.indicators.exec_ids.unwrap();
        assert!(ids.contains(&1_i64), "exec_ids should contain 1");
        assert!(ids.contains(&2_i64), "exec_ids should contain 2");
        assert!(ids.contains(&42_i64), "exec_ids should contain 42");
        assert_eq!(ids.len(), 3, "exec_ids len should be 3");
    }

    #[test]
    fn test_exec_ids_absent_returns_none() {
        let toml = "[filter]\nenable = true\n";
        let w: FilterWrapper = toml::from_str(toml).unwrap();
        assert!(
            w.filter.indicators.exec_ids.is_none(),
            "exec_ids should be None when absent"
        );
    }

    // ── FiltersFeature::from — 旧格式扁平字段映射 ───────────────

    #[test]
    fn test_legacy_flat_usernames_mapped_to_include_users() {
        let toml = r#"
            [filter]
            enable = true
            usernames = ["ALICE", "BOB"]
        "#;
        let w: FilterWrapper = toml::from_str(toml).unwrap();
        assert_eq!(
            w.filter.include.users,
            Some(vec!["ALICE".to_string(), "BOB".to_string()]),
            "usernames should map to include.users"
        );
    }

    #[test]
    fn test_legacy_flat_exclude_usernames_mapped() {
        let toml = r#"
            [filter]
            enable = true
            exclude_usernames = ["X"]
        "#;
        let w: FilterWrapper = toml::from_str(toml).unwrap();
        assert_eq!(
            w.filter.exclude.users,
            Some(vec!["X".to_string()]),
            "exclude_usernames should map to exclude.users"
        );
    }

    #[test]
    fn test_legacy_flat_all_include_fields_mapped() {
        let toml = r#"
            [filter]
            enable = true
            client_ips = ["10.0.0.1"]
            sess_ids = ["S1"]
            thrd_ids = ["T1"]
            statements = ["ST1"]
            appnames = ["A1"]
            tags = ["TG1"]
            start_ts = "2024-01-01"
            end_ts = "2024-12-31"
        "#;
        let w: FilterWrapper = toml::from_str(toml).unwrap();
        let inc = &w.filter.include;
        assert_eq!(
            inc.ips,
            Some(vec!["10.0.0.1".to_string()]),
            "client_ips → ips"
        );
        assert_eq!(
            inc.sessions,
            Some(vec!["S1".to_string()]),
            "sess_ids → sessions"
        );
        assert_eq!(
            inc.threads,
            Some(vec!["T1".to_string()]),
            "thrd_ids → threads"
        );
        assert_eq!(
            inc.statements,
            Some(vec!["ST1".to_string()]),
            "statements → statements"
        );
        assert_eq!(inc.apps, Some(vec!["A1".to_string()]), "appnames → apps");
        assert_eq!(inc.tags, Some(vec!["TG1".to_string()]), "tags → tags");
        assert_eq!(
            inc.start_ts,
            Some("2024-01-01".to_string()),
            "start_ts → start_ts"
        );
        assert_eq!(
            inc.end_ts,
            Some("2024-12-31".to_string()),
            "end_ts → end_ts"
        );
    }

    // ── FiltersFeature::from — 混合格式优先级 ───────────────────

    #[test]
    fn test_mixed_format_new_table_takes_priority() {
        let toml = r#"
            [filter]
            enable = true
            usernames = ["IGNORED"]
            [filter.include]
            users = ["WINNER"]
        "#;
        let w: FilterWrapper = toml::from_str(toml).unwrap();
        assert_eq!(
            w.filter.include.users,
            Some(vec!["WINNER".to_string()]),
            "new [filter.include] sub-table should take priority over flat usernames"
        );
    }

    #[test]
    fn test_mixed_format_exclude_new_table_priority() {
        let toml = r#"
            [filter]
            enable = true
            exclude_usernames = ["IGNORED"]
            [filter.exclude]
            users = ["WINNER"]
        "#;
        let w: FilterWrapper = toml::from_str(toml).unwrap();
        assert_eq!(
            w.filter.exclude.users,
            Some(vec!["WINNER".to_string()]),
            "new [filter.exclude] sub-table should take priority over flat exclude_usernames"
        );
    }

    // ── IncludeFilters::has_filters — ips/sessions/threads/apps/tags ──

    #[test]
    fn test_include_filters_has_filters_with_ips() {
        let filters = IncludeFilters {
            ips: Some(vec!["10.0.0.1".to_string()]),
            ..IncludeFilters::default()
        };
        assert!(
            filters.has_filters(),
            "has_filters should be true when ips is set"
        );
    }

    #[test]
    fn test_include_filters_has_filters_with_sessions() {
        let filters = IncludeFilters {
            sessions: Some(vec!["SESS1".to_string()]),
            ..IncludeFilters::default()
        };
        assert!(
            filters.has_filters(),
            "has_filters should be true when sessions is set"
        );
    }

    #[test]
    fn test_include_filters_has_filters_with_threads() {
        let filters = IncludeFilters {
            threads: Some(vec!["T1".to_string()]),
            ..IncludeFilters::default()
        };
        assert!(
            filters.has_filters(),
            "has_filters should be true when threads is set"
        );
    }

    #[test]
    fn test_include_filters_has_filters_with_apps() {
        let filters = IncludeFilters {
            apps: Some(vec!["MyApp".to_string()]),
            ..IncludeFilters::default()
        };
        assert!(
            filters.has_filters(),
            "has_filters should be true when apps is set"
        );
    }

    #[test]
    fn test_include_filters_has_filters_with_tags() {
        let filters = IncludeFilters {
            tags: Some(vec!["tag1".to_string()]),
            ..IncludeFilters::default()
        };
        assert!(
            filters.has_filters(),
            "has_filters should be true when tags is set"
        );
    }

    // ── ExcludeFilters::has_filters — ips/sessions/threads/apps/tags ──

    #[test]
    fn test_exclude_filters_has_filters_with_ips() {
        let filters = ExcludeFilters {
            ips: Some(vec!["192.168.1.1".to_string()]),
            ..ExcludeFilters::default()
        };
        assert!(
            filters.has_filters(),
            "exclude has_filters should be true when ips is set"
        );
    }

    #[test]
    fn test_exclude_filters_has_filters_with_sessions() {
        let filters = ExcludeFilters {
            sessions: Some(vec!["SESS2".to_string()]),
            ..ExcludeFilters::default()
        };
        assert!(
            filters.has_filters(),
            "exclude has_filters should be true when sessions is set"
        );
    }

    #[test]
    fn test_exclude_filters_has_filters_with_threads() {
        let filters = ExcludeFilters {
            threads: Some(vec!["T2".to_string()]),
            ..ExcludeFilters::default()
        };
        assert!(
            filters.has_filters(),
            "exclude has_filters should be true when threads is set"
        );
    }

    #[test]
    fn test_exclude_filters_has_filters_with_apps() {
        let filters = ExcludeFilters {
            apps: Some(vec!["OtherApp".to_string()]),
            ..ExcludeFilters::default()
        };
        assert!(
            filters.has_filters(),
            "exclude has_filters should be true when apps is set"
        );
    }

    #[test]
    fn test_exclude_filters_has_filters_with_tags() {
        let filters = ExcludeFilters {
            tags: Some(vec!["exclude_tag".to_string()]),
            ..ExcludeFilters::default()
        };
        assert!(
            filters.has_filters(),
            "exclude has_filters should be true when tags is set"
        );
    }

    // ── SqlFilters::has_filters 边缘情况 ──────────────────────────

    #[test]
    fn test_sql_filters_has_filters_excludes_only_returns_true() {
        // excludes-only（无 includes）应返回 true
        let sf = SqlFilters {
            includes: None,
            excludes: Some(vec!["DROP".to_string()]),
        };
        assert!(
            sf.has_filters(),
            "SqlFilters with excludes-only should return has_filters=true"
        );
    }

    #[test]
    fn test_sql_filters_has_filters_some_empty_vec_returns_false() {
        // Some(vec![]) 是空集合，不应算作有过滤器
        let sf = SqlFilters {
            includes: Some(vec![]),
            excludes: Some(vec![]),
        };
        assert!(
            !sf.has_filters(),
            "SqlFilters with Some(vec![]) for both fields should return has_filters=false"
        );
    }

    #[test]
    fn test_sql_filters_has_filters_none_both_returns_false() {
        let sf = SqlFilters::default();
        assert!(
            !sf.has_filters(),
            "SqlFilters with both None should return has_filters=false"
        );
    }

    #[test]
    fn test_sql_filters_has_filters_includes_only_returns_true() {
        let sf = SqlFilters {
            includes: Some(vec!["SELECT".to_string()]),
            excludes: None,
        };
        assert!(
            sf.has_filters(),
            "SqlFilters with includes-only should return has_filters=true"
        );
    }
}
