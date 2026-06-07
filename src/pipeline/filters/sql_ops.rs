use super::types::SqlFilters;

impl SqlFilters {
    #[must_use]
    pub(crate) fn has_filters(&self) -> bool {
        self.includes.as_ref().is_some_and(|v| !v.is_empty())
            || self.excludes.as_ref().is_some_and(|v| !v.is_empty())
    }
}
