/// 导出字段名列表（顺序与 CSV/SQLite 列顺序一致，共 15 个字段）
pub const FIELD_NAMES: &[&str] = &[
    "ts",             // 0
    "ep",             // 1
    "sess_id",        // 2
    "thrd_id",        // 3
    "username",       // 4
    "trx_id",         // 5
    "statement",      // 6
    "appname",        // 7
    "client_ip",      // 8
    "tag",            // 9
    "sql",            // 10
    "exec_time_ms",   // 11
    "row_count",      // 12
    "exec_id",        // 13
    "normalized_sql", // 14
];

/// 字段投影掩码：u16 位图，bit i=1 表示导出第 i 个字段（共 15 个）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldMask(pub u16);

impl FieldMask {
    /// 全部 15 个字段都导出（默认值）
    pub const ALL: Self = Self(0x7FFF);

    /// 从字段名列表构建掩码，未知字段名返回错误消息
    pub fn from_names(names: &[String]) -> std::result::Result<Self, String> {
        let mut mask = 0u16;
        for name in names {
            match FIELD_NAMES.iter().position(|&n| n == name.as_str()) {
                Some(idx) => mask |= 1u16 << idx,
                None => return Err(format!("unknown field: '{name}'")),
            }
        }
        Ok(Self(mask))
    }

    /// 第 `idx` 个字段是否启用
    #[inline]
    #[must_use]
    pub(crate) fn is_active(self, idx: usize) -> bool {
        idx < 15 && (self.0 >> idx) & 1 == 1
    }

    /// `normalized_sql` 字段（索引 14）是否启用
    #[inline]
    #[must_use]
    pub fn includes_normalized_sql(self) -> bool {
        self.is_active(14)
    }
}

impl Default for FieldMask {
    fn default() -> Self {
        Self::ALL
    }
}
