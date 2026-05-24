use crate::config::Config;
use crate::pipeline::filters::RecordMeta;
use crate::pipeline::{CompiledMetaFilters, LogProcessor, Pipeline};

/// 构建处理器管线。
///
/// `compiled_meta` 由 `Config::validate_and_compile` 在主流程入口预编译；
/// 当输入 `None` 或 `has_filters() == false` 时返回空管线。
pub(super) fn build_pipeline(cfg: &Config, compiled_meta: Option<CompiledMetaFilters>) -> Pipeline {
    let mut pipeline = Pipeline::new();

    if let (Some(f), Some(meta)) = (cfg.filter.as_ref(), compiled_meta) {
        if f.has_filters() {
            pipeline.add(Box::new(FilterProcessor::new(meta, f)));
        }
    }

    pipeline
}

#[derive(Debug)]
struct FilterProcessor {
    /// 预编译的元数据过滤器（跨字段 AND 语义，字段内 OR 语义）
    compiled_meta: CompiledMetaFilters,
    /// 时间范围过滤（字符串比较，不用正则）
    start_ts: Option<String>,
    end_ts: Option<String>,
    /// 预计算：`compiled_meta.has_any_filters()` 的结果（include 或 exclude 任一），避免热路径重复检查
    has_meta_filters: bool,
}

impl FilterProcessor {
    /// 接受预编译的 `CompiledMetaFilters`（来自 `Config::validate_and_compile`），
    /// 避免在 `run` 路径中第二次调用 `Regex::new()`。
    ///
    /// `has_any_filters()` 包含 exclude 字段，确保纯 exclude 配置也激活 meta 检查路径（D-05）
    fn new(compiled_meta: CompiledMetaFilters, filter: &crate::pipeline::FiltersFeature) -> Self {
        let has_meta_filters = compiled_meta.has_any_filters();
        Self {
            compiled_meta,
            start_ts: filter.include.start_ts.clone(),
            end_ts: filter.include.end_ts.clone(),
            has_meta_filters,
        }
    }
}

impl LogProcessor for FilterProcessor {
    fn process(&self, record: &dm_database_parser_sqllog::Sqllog) -> bool {
        self.process_with_meta(record)
    }

    /// 热路径：使用 `Sqllog` 的直接字段（parser 库已物化所有元数据字段）。
    ///
    /// 时间过滤在前（无需构造 `RecordMeta`），之后用预计算的 `has_meta_filters`
    /// 快速判断是否需要进入元数据过滤 —— 过滤器只含时间范围时直接返回 true。
    fn process_with_meta(&self, record: &dm_database_parser_sqllog::Sqllog) -> bool {
        let ts = &record.ts;

        // 时间过滤：无需构造 RecordMeta
        if let Some(start) = &self.start_ts {
            if ts.as_str() < start.as_str() {
                return false;
            }
        }
        if let Some(end) = &self.end_ts {
            if ts.as_str() > end.as_str() {
                return false;
            }
        }

        // 快速路径：无元数据过滤 → 直接通过，跳过 RecordMeta 构造
        if !self.has_meta_filters {
            return true;
        }

        self.compiled_meta.should_keep(&RecordMeta {
            trxid: &record.trxid,
            ip: &record.client_ip,
            sess: &record.sess_id,
            thrd: &record.thrd_id,
            user: &record.username,
            stmt: &record.statement,
            app: &record.appname,
            tag: record.tag.as_deref(),
        })
    }
}
