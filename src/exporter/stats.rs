/// 导出统计
#[derive(Debug, Default, Clone, Copy)]
pub struct ExportStats {
    pub exported: usize,
    pub skipped: usize,
    pub failed: usize,
    pub flush_operations: usize,
    pub last_flush_size: usize,
}

impl ExportStats {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_success(&mut self) {
        self.exported += 1;
    }

    #[must_use]
    pub fn total(&self) -> usize {
        self.exported + self.skipped + self.failed
    }
}
