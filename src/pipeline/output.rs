/// 导出字段名列表（顺序与 CSV/Parquet 列顺序一致，共 15 个字段）
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
    ///
    /// # Errors
    ///
    /// 列表中包含未知字段名时返回含该字段名的错误消息。
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

use serde::Deserialize;

/// `[output]` 配置段：字段投影
#[derive(Debug, Deserialize, Clone, Default)]
pub struct OutputConfig {
    /// 字段投影：仅导出指定字段，默认为全部 15 个字段
    #[serde(default)]
    pub fields: Option<Vec<String>>,
}

impl OutputConfig {
    /// 计算字段投影掩码。字段名在 `validate()` 阶段已验证，无效名称静默退化为全量掩码。
    #[must_use]
    pub fn field_mask(&self) -> FieldMask {
        match &self.fields {
            None => FieldMask::ALL,
            Some(names) if names.is_empty() => FieldMask::ALL,
            Some(names) => FieldMask::from_names(names).unwrap_or(FieldMask::ALL),
        }
    }

    /// 按用户配置顺序返回字段索引列表，供 exporter 写入时按序遍历。
    #[must_use]
    pub fn ordered_field_indices(&self) -> Vec<usize> {
        match &self.fields {
            None => (0..FIELD_NAMES.len()).collect(),
            Some(names) if names.is_empty() => (0..FIELD_NAMES.len()).collect(),
            Some(names) => names
                .iter()
                .filter_map(|name| FIELD_NAMES.iter().position(|&n| n == name.as_str()))
                .collect(),
        }
    }
}
