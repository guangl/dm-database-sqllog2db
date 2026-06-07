use serde::Deserialize;

/// `[replace_parameters]` 配置段
#[derive(Debug, Deserialize, Clone)]
pub struct NormalizeConfig {
    /// 是否在导出结果中写入 `normalized_sql` 列（默认 true）
    #[serde(default = "default_true")]
    pub enable: bool,
    /// 显式声明 SQL 中使用的占位符列表，例如 `["?"]` 或 `[":1"]`。
    /// - 只含 `"?"` → 仅匹配 `?` 顺序占位符
    /// - 含任意 `:N` 形式（如 `":1"`）→ 仅匹配 `:N` 序号占位符
    /// - 空数组（默认）→ 自动检测
    #[serde(default)]
    pub placeholders: Vec<String>,
}

impl Default for NormalizeConfig {
    fn default() -> Self {
        Self {
            enable: true,
            placeholders: Vec::new(),
        }
    }
}

impl NormalizeConfig {
    /// 将 `placeholders` 列表转换为 `compute_normalized` 所需的 `placeholder_override`：
    /// - `None`        → 自动检测
    /// - `Some(false)` → 强制 `?` 风格
    /// - `Some(true)`  → 强制 `:N` 风格
    #[must_use]
    pub fn placeholder_override(&self) -> Option<bool> {
        let has_question = self.placeholders.iter().any(|p| p == "?");
        let has_colon = self.placeholders.iter().any(|p| {
            p.starts_with(':') && p[1..].chars().next().is_some_and(|c| c.is_ascii_digit())
        });
        match (has_question, has_colon) {
            (true, false) => Some(false),
            (false, true) => Some(true),
            _ => None,
        }
    }
}

fn default_true() -> bool {
    true
}
