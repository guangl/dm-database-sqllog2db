use serde::Deserialize;

use super::field_mask::{FIELD_NAMES, FieldMask};

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
