use serde::Deserialize;

use super::serde_helpers::{TrxidSet, non_negative_finite_ms, vec_to_hashset, vec_to_i64_hashset};

/// 过滤只分保留和排除两组；配置条件自动生效。
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct FiltersFeature {
    #[serde(default)]
    pub include: IncludeFilters,
    #[serde(default)]
    pub exclude: ExcludeFilters,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct IncludeFilters {
    #[serde(default)]
    pub sql: Option<Vec<String>>,
    #[serde(default, deserialize_with = "vec_to_i64_hashset")]
    pub exec_ids: Option<std::collections::HashSet<i64>>,
    #[serde(default, deserialize_with = "non_negative_finite_ms")]
    pub min_runtime_ms: Option<f32>,
    pub min_row_count: Option<u32>,
    pub users: Option<Vec<String>>,
    pub ips: Option<Vec<String>>,
    pub sessions: Option<Vec<String>>,
    pub threads: Option<Vec<String>>,
    pub apps: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub start_ts: Option<String>,
    pub end_ts: Option<String>,
    #[serde(default, deserialize_with = "vec_to_hashset")]
    pub trxids: Option<TrxidSet>,
}

impl IncludeFilters {
    pub(crate) fn has_transaction_filters(&self) -> bool {
        self.exec_ids.as_ref().is_some_and(|v| !v.is_empty())
            || self.min_runtime_ms.is_some()
            || self.min_row_count.is_some()
            || self.sql.as_ref().is_some_and(|v| !v.is_empty())
    }

    pub(crate) fn clear_transaction_filters(&mut self) {
        self.exec_ids = None;
        self.min_runtime_ms = None;
        self.min_row_count = None;
        self.sql = None;
    }

    #[must_use]
    pub fn has_filters(&self) -> bool {
        self.users.as_ref().is_some_and(|v| !v.is_empty())
            || self.ips.as_ref().is_some_and(|v| !v.is_empty())
            || self.sessions.as_ref().is_some_and(|v| !v.is_empty())
            || self.threads.as_ref().is_some_and(|v| !v.is_empty())
            || self.apps.as_ref().is_some_and(|v| !v.is_empty())
            || self.tags.as_ref().is_some_and(|v| !v.is_empty())
            || self.start_ts.is_some()
            || self.end_ts.is_some()
            || self.trxids.is_some()
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct ExcludeFilters {
    #[serde(default)]
    pub sql: Option<Vec<String>>,
    #[serde(default, deserialize_with = "vec_to_i64_hashset")]
    pub exec_ids: Option<std::collections::HashSet<i64>>,
    #[serde(default, deserialize_with = "non_negative_finite_ms")]
    pub min_runtime_ms: Option<f32>,
    pub min_row_count: Option<u32>,
    pub users: Option<Vec<String>>,
    pub ips: Option<Vec<String>>,
    pub sessions: Option<Vec<String>>,
    pub threads: Option<Vec<String>>,
    pub apps: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

impl ExcludeFilters {
    pub(crate) fn has_transaction_filters(&self) -> bool {
        self.exec_ids.as_ref().is_some_and(|v| !v.is_empty())
            || self.min_runtime_ms.is_some()
            || self.min_row_count.is_some()
            || self.sql.as_ref().is_some_and(|v| !v.is_empty())
    }

    pub(crate) fn clear_transaction_filters(&mut self) {
        self.exec_ids = None;
        self.min_runtime_ms = None;
        self.min_row_count = None;
        self.sql = None;
    }

    #[must_use]
    pub fn has_filters(&self) -> bool {
        self.users.as_ref().is_some_and(|v| !v.is_empty())
            || self.ips.as_ref().is_some_and(|v| !v.is_empty())
            || self.sessions.as_ref().is_some_and(|v| !v.is_empty())
            || self.threads.as_ref().is_some_and(|v| !v.is_empty())
            || self.apps.as_ref().is_some_and(|v| !v.is_empty())
            || self.tags.as_ref().is_some_and(|v| !v.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Deserialize)]
    struct Wrapper {
        filter: FiltersFeature,
    }

    #[test]
    fn groups_read_all_conditions() {
        for group in ["include", "exclude"] {
            let cfg: Wrapper = toml::from_str(&format!(
                r#"
                [filter.{group}]
                users = ["SYSDBA"]
                ips = ["127.0.0.1"]
                sessions = ["0x1"]
                threads = ["1"]
                apps = ["A"]
                tags = ["SEL"]
                sql = ["SELECT"]
                exec_ids = [42, 42]
                min_runtime_ms = 1.5
                min_row_count = 0
            "#
            ))
            .unwrap();
            assert!(cfg.filter.has_transaction_filters());
            if group == "include" {
                assert!(cfg.filter.include.has_filters());
                assert_eq!(cfg.filter.include.min_runtime_ms, Some(1.5));
                assert_eq!(cfg.filter.include.exec_ids.unwrap().len(), 1);
            } else {
                assert!(cfg.filter.exclude.has_filters());
                assert_eq!(cfg.filter.exclude.min_row_count, Some(0));
                assert_eq!(cfg.filter.exclude.sql.unwrap(), ["SELECT"]);
            }
        }
    }

    #[test]
    fn empty_lists_are_inactive() {
        let cfg: Wrapper = toml::from_str("[filter.include]\ntrxids = []\nexec_ids = []\nsql = []\n[filter.exclude]\nexec_ids = []\nsql = []").unwrap();
        assert!(cfg.filter.include.trxids.is_none());
        assert!(!cfg.filter.has_filters());
    }

    #[test]
    fn trxids_are_deduplicated() {
        let cfg: Wrapper = toml::from_str("[filter.include]\ntrxids = ['1', '1', '2']").unwrap();
        assert_eq!(cfg.filter.include.trxids.unwrap().len(), 2);
    }

    #[test]
    fn legacy_and_unknown_fields_are_rejected() {
        for source in [
            "[filter]\nenable = true",
            "[filter]\nusernames = ['U']",
            "[filter]\nexclude_usernames = ['U']",
            "[filter.indicators]\nmin_runtime_ms = 100",
            "[filter.sql]\nincludes = ['SELECT']",
            "[filter.include]\nstatements = ['SEL']",
            "[filter.exclude]\nstatements = ['SEL']",
            "[filter.include]\nmin_runtme_ms = 10",
            "[filter.exclude]\nsqll = ['DROP']",
        ] {
            let error = toml::from_str::<Wrapper>(source).err().expect(source);
            assert!(error.to_string().contains("unknown field"), "{error}");
        }
    }

    #[test]
    fn transaction_thresholds_are_validated() {
        for group in ["include", "exclude"] {
            for value in ["-1", "nan", "inf"] {
                let source = format!("[filter.{group}]\nmin_runtime_ms = {value}");
                assert!(toml::from_str::<Wrapper>(&source).is_err());
            }
        }
    }
}
