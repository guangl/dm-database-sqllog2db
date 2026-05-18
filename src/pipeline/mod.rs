pub mod filters;
pub use filters::FiltersFeature;
pub(crate) use filters::{CompiledMetaFilters, CompiledSqlFilters};

pub mod normalizer;
pub(crate) use normalizer::compute_normalized;

pub mod fingerprint;
pub(crate) use fingerprint::fingerprint;
pub(crate) use fingerprint::normalize_template;

pub mod aggregator;
pub(crate) use aggregator::ChartEntry;
pub(crate) use aggregator::TemplateAggregator;
pub(crate) use aggregator::TemplateStats;

use dm_database_parser_sqllog::{MetaParts, Sqllog};
use serde::Deserialize;

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

fn default_top_n() -> usize {
    10
}

/// `[template]` 配置段
#[derive(Debug, Deserialize, Clone, Default)]
pub struct TemplateConfig {
    /// 是否启用 SQL 模板归一化（默认 false）
    #[serde(default)]
    pub enable: bool,
    /// 模板统计 CSV 输出路径；空字符串 = 不生成
    #[serde(default)]
    pub output_csv_path: String,
    /// 模板统计 `SQLite` 表名；空字符串 = 不生成
    #[serde(default)]
    pub output_sqlite_table: String,
}

/// `[charts]` 配置段
#[derive(Debug, Deserialize, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct ChartsConfig {
    /// 图表输出目录（必填，无默认值）
    pub output_dir: String,
    /// 频率 Top-N 数量（默认 10）
    #[serde(default = "default_top_n")]
    pub top_n: usize,
    /// 是否生成频率柱状图（默认 true）
    #[serde(default = "default_true")]
    pub frequency_bar: bool,
    /// 是否生成延迟直方图（默认 true）
    #[serde(default = "default_true")]
    pub latency_hist: bool,
    /// 是否生成时间趋势折线图（默认 true）
    #[serde(default = "default_true")]
    pub trend_line: bool,
    /// 是否生成用户占比饼图（默认 true）
    #[serde(default = "default_true")]
    pub user_pie: bool,
}

impl Default for ChartsConfig {
    fn default() -> Self {
        Self {
            output_dir: "charts/".to_string(),
            top_n: 10,
            frequency_bar: true,
            latency_hist: true,
            trend_line: true,
            user_pie: true,
        }
    }
}

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

/// 记录处理器接口：实现此接口即可加入处理管线
/// 返回 true 表示保留该记录，false 表示丢弃
pub trait LogProcessor: Send + Sync + std::fmt::Debug {
    fn process(&self, record: &Sqllog) -> bool;

    /// 使用调用方已预解析的 `MetaParts` 运行过滤逻辑，
    /// 消除 `parse_meta()` 的重复调用。
    /// 默认实现退化为 `process()`。
    fn process_with_meta(&self, record: &Sqllog, _meta: &MetaParts<'_>) -> bool {
        self.process(record)
    }
}

/// 处理管线：按顺序执行处理器，任一返回 false 则丢弃记录
#[derive(Debug, Default)]
pub struct Pipeline {
    processors: Vec<Box<dyn LogProcessor>>,
}

impl Pipeline {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, processor: Box<dyn LogProcessor>) {
        self.processors.push(processor);
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.processors.is_empty()
    }

    #[inline]
    #[must_use]
    pub fn run_with_meta(&self, record: &Sqllog, meta: &MetaParts<'_>) -> bool {
        self.processors
            .iter()
            .all(|p| p.process_with_meta(record, meta))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_empty() {
        let p = Pipeline::new();
        assert!(p.is_empty());
    }

    #[test]
    fn test_pipeline_add() {
        let mut p = Pipeline::new();

        #[derive(Debug)]
        struct AlwaysPass;
        impl LogProcessor for AlwaysPass {
            fn process(&self, _: &dm_database_parser_sqllog::Sqllog) -> bool {
                true
            }
        }

        p.add(Box::new(AlwaysPass));
        assert!(!p.is_empty());
    }

    #[test]
    fn test_placeholder_override_question() {
        let cfg = NormalizeConfig {
            enable: true,
            placeholders: vec!["?".into()],
        };
        assert_eq!(cfg.placeholder_override(), Some(false));
    }

    #[test]
    fn test_placeholder_override_colon() {
        let cfg = NormalizeConfig {
            enable: true,
            placeholders: vec![":1".into()],
        };
        assert_eq!(cfg.placeholder_override(), Some(true));
    }

    #[test]
    fn test_placeholder_override_auto() {
        let cfg = NormalizeConfig {
            enable: true,
            placeholders: vec![],
        };
        assert_eq!(cfg.placeholder_override(), None);
    }

    #[test]
    fn test_placeholder_override_both_is_auto() {
        let cfg = NormalizeConfig {
            enable: true,
            placeholders: vec!["?".into(), ":1".into()],
        };
        assert_eq!(cfg.placeholder_override(), None);
    }

    #[test]
    fn test_charts_config_default_values() {
        let cfg = ChartsConfig::default();
        assert_eq!(cfg.output_dir, "charts/");
        assert_eq!(cfg.top_n, 10);
        assert!(cfg.frequency_bar);
        assert!(cfg.latency_hist);
        assert!(cfg.trend_line);
        assert!(cfg.user_pie);
    }

    #[test]
    fn test_charts_config_deserialize_only_output_dir() {
        let cfg: ChartsConfig = toml::from_str(r#"output_dir = "out/""#).unwrap();
        assert_eq!(cfg.output_dir, "out/");
        assert_eq!(cfg.top_n, 10);
        assert!(cfg.frequency_bar);
        assert!(cfg.latency_hist);
    }

    #[test]
    fn test_charts_config_deserialize_full() {
        let toml = r#"
output_dir = "custom/"
top_n = 5
frequency_bar = false
latency_hist = false
"#;
        let cfg: ChartsConfig = toml::from_str(toml).unwrap();
        assert_eq!(cfg.output_dir, "custom/");
        assert_eq!(cfg.top_n, 5);
        assert!(!cfg.frequency_bar);
        assert!(!cfg.latency_hist);
        assert!(cfg.trend_line);
        assert!(cfg.user_pie);
    }

    #[test]
    fn test_template_config_default() {
        let cfg = TemplateConfig::default();
        assert!(!cfg.enable);
        assert_eq!(cfg.output_csv_path, "");
        assert_eq!(cfg.output_sqlite_table, "");
    }

    #[test]
    fn test_template_config_deserialize_full() {
        let toml = "enable = true\noutput_csv_path = \"out.csv\"\noutput_sqlite_table = \"tpl\"";
        let cfg: TemplateConfig = toml::from_str(toml).unwrap();
        assert!(cfg.enable);
        assert_eq!(cfg.output_csv_path, "out.csv");
        assert_eq!(cfg.output_sqlite_table, "tpl");
    }

    #[test]
    fn test_template_config_deserialize_empty_is_default() {
        let cfg: TemplateConfig = toml::from_str("").unwrap();
        assert!(!cfg.enable);
        assert_eq!(cfg.output_csv_path, "");
        assert_eq!(cfg.output_sqlite_table, "");
    }

    #[test]
    fn test_normalize_config_default() {
        let cfg = NormalizeConfig::default();
        assert!(cfg.enable);
        assert!(cfg.placeholders.is_empty());
    }

    #[test]
    fn test_output_config_field_mask_default() {
        let cfg = OutputConfig::default();
        assert_eq!(cfg.field_mask(), FieldMask::ALL);
    }

    #[test]
    fn test_output_config_field_mask_with_names() {
        let cfg = OutputConfig {
            fields: Some(vec!["sql".into(), "username".into()]),
        };
        let mask = cfg.field_mask();
        // sql=10, username=4
        assert!(mask.is_active(10));
        assert!(mask.is_active(4));
        assert!(!mask.is_active(0)); // ts not included
    }

    #[test]
    fn test_output_config_ordered_indices_preserves_user_order() {
        let cfg = OutputConfig {
            fields: Some(vec!["sql".into(), "username".into(), "ts".into()]),
        };
        let indices = cfg.ordered_field_indices();
        assert_eq!(indices, vec![10_usize, 4, 0]);
    }

    #[test]
    fn test_output_config_ordered_indices_none_returns_all() {
        let cfg = OutputConfig::default();
        let indices = cfg.ordered_field_indices();
        assert_eq!(indices, (0..15_usize).collect::<Vec<_>>());
    }

    #[test]
    fn test_output_config_ordered_indices_empty_equals_all() {
        let cfg = OutputConfig {
            fields: Some(vec![]),
        };
        let indices = cfg.ordered_field_indices();
        assert_eq!(indices.len(), 15);
        assert_eq!(indices, (0..15_usize).collect::<Vec<_>>());
    }

    #[test]
    fn test_charts_config_deserialize_trend_user_flags() {
        let toml_str = r#"
output_dir = "out/"
trend_line = false
user_pie = false
"#;
        let cfg: ChartsConfig = toml::from_str(toml_str).unwrap();
        assert!(!cfg.trend_line);
        assert!(!cfg.user_pie);
        assert!(cfg.frequency_bar);
        assert!(cfg.latency_hist);
    }

    #[test]
    fn test_charts_config_new_fields_default_true() {
        let cfg: ChartsConfig = toml::from_str(r#"output_dir = "out/""#).unwrap();
        assert!(cfg.trend_line);
        assert!(cfg.user_pie);
    }

    #[test]
    fn test_default_true_via_serde() {
        let cfg: NormalizeConfig = toml::from_str("").unwrap();
        assert!(cfg.enable);
    }

    #[test]
    fn test_process_with_meta_default_delegates_to_process() {
        use dm_database_parser_sqllog::LogParser;

        #[derive(Debug)]
        struct AlwaysPass;
        impl LogProcessor for AlwaysPass {
            fn process(&self, _: &dm_database_parser_sqllog::Sqllog) -> bool {
                true
            }
        }

        #[derive(Debug)]
        struct AlwaysFail;
        impl LogProcessor for AlwaysFail {
            fn process(&self, _: &dm_database_parser_sqllog::Sqllog) -> bool {
                false
            }
        }

        let dir = tempfile::TempDir::new().unwrap();
        let log = dir.path().join("t.log");
        std::fs::write(&log, "2025-01-15 10:30:28.001 (EP[0] sess:0x0001 user:U trxid:1 stmt:0x1 appname:A ip:10.0.0.1) [SEL] SELECT 1. EXECTIME: 1(ms) ROWCOUNT: 1(rows) EXEC_ID: 1.\n").unwrap();
        let parser = LogParser::from_path(log.to_str().unwrap()).unwrap();
        let records: Vec<_> = parser.iter().flatten().collect();
        assert!(!records.is_empty());

        let record = &records[0];
        let meta = record.parse_meta();

        let mut p = Pipeline::new();
        p.add(Box::new(AlwaysPass));
        assert!(p.run_with_meta(record, &meta));

        let mut p2 = Pipeline::new();
        p2.add(Box::new(AlwaysFail));
        assert!(!p2.run_with_meta(record, &meta));
    }
}
