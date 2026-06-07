use super::types::IndicatorFilters;

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
