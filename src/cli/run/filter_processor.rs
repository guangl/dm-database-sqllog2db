use crate::config::Config;
use crate::pipeline::filters::RecordMeta;
use crate::pipeline::{CompiledMetaFilters, LogProcessor, Pipeline};
use dm_database_parser_sqllog::MetaParts;

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
        let meta = record.parse_meta();
        self.process_with_meta(record, &meta)
    }

    /// 热路径重载：复用调用方已解析的 `MetaParts`，消除 `parse_meta()` 重复调用。
    ///
    /// 时间过滤在前（无需构造 `RecordMeta`），之后用预计算的 `has_meta_filters`
    /// 快速判断是否需要进入元数据过滤 —— 过滤器只含时间范围时直接返回 true。
    fn process_with_meta(
        &self,
        record: &dm_database_parser_sqllog::Sqllog,
        meta: &MetaParts<'_>,
    ) -> bool {
        let ts = record.ts.as_ref();

        // 时间过滤：无需构造 RecordMeta
        if let Some(start) = &self.start_ts {
            if ts < start.as_str() {
                return false;
            }
        }
        if let Some(end) = &self.end_ts {
            if ts > end.as_str() {
                return false;
            }
        }

        // 快速路径：无元数据过滤 → 直接通过，跳过 RecordMeta 构造
        if !self.has_meta_filters {
            return true;
        }

        self.compiled_meta.should_keep(&RecordMeta {
            trxid: meta.trxid.as_ref(),
            ip: meta.client_ip.as_ref(),
            sess: meta.sess_id.as_ref(),
            thrd: meta.thrd_id.as_ref(),
            user: meta.username.as_ref(),
            stmt: meta.statement.as_ref(),
            app: meta.appname.as_ref(),
            tag: record.tag.as_deref(),
        })
    }
}

/// 进度条控制：quiet 时返回 false（不输出进度）。
pub(super) fn make_progress_bar(quiet: bool, _interval_ms: u64) -> bool {
    !quiet
}
