use serde::{Deserialize, Deserializer};

/// `trxid` 过滤集合类型。
pub(crate) type TrxidSet = std::collections::HashSet<String>;

pub(super) fn vec_to_hashset<'de, D>(deserializer: D) -> Result<Option<TrxidSet>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<Vec<String>> = Option::deserialize(deserializer)?;
    // Empty list normalizes to None so `trxids = []` in config means "no filter".
    // Only merge_found_trxids may produce Some(empty_set) as a prescan sentinel.
    Ok(v.and_then(|items| {
        if items.is_empty() {
            None
        } else {
            Some(items.into_iter().collect())
        }
    }))
}

/// 反序列化毫秒阈值：拒绝负数与非有限值（NaN/inf），在配置加载阶段即报错。
pub(super) fn non_negative_finite_ms<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<f32> = Option::deserialize(deserializer)?;
    if let Some(ms) = v {
        if !ms.is_finite() || ms < 0.0 {
            return Err(serde::de::Error::custom(format!(
                "min_runtime_ms must be a non-negative finite number, got {ms}"
            )));
        }
    }
    Ok(v)
}

pub(super) fn vec_to_i64_hashset<'de, D>(
    deserializer: D,
) -> Result<Option<std::collections::HashSet<i64>>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<Vec<i64>> = Option::deserialize(deserializer)?;
    Ok(v.and_then(|items| {
        if items.is_empty() {
            None
        } else {
            Some(items.into_iter().collect())
        }
    }))
}
