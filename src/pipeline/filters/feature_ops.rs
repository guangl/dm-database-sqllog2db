use super::serde_helpers::TrxidSet;
use super::types::FiltersFeature;

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
