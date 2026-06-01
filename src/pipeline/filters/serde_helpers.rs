use serde::{Deserialize, Deserializer};

/// `trxid` 过滤集合类型。
pub(crate) type TrxidSet = std::collections::HashSet<String>;

pub(super) fn vec_to_hashset<'de, D>(deserializer: D) -> Result<Option<TrxidSet>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<Vec<String>> = Option::deserialize(deserializer)?;
    Ok(v.map(|items| items.into_iter().collect()))
}

pub(super) fn vec_to_i64_hashset<'de, D>(
    deserializer: D,
) -> Result<Option<std::collections::HashSet<i64>>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<Vec<i64>> = Option::deserialize(deserializer)?;
    Ok(v.map(|items| items.into_iter().collect()))
}
