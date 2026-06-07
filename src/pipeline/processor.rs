use dm_database_parser_sqllog::Sqllog;

/// 记录处理器接口：实现此接口即可加入处理管线
/// 返回 true 表示保留该记录，false 表示丢弃
pub trait LogProcessor: Send + Sync + std::fmt::Debug {
    fn process(&self, record: &Sqllog) -> bool;

    /// 使用已解析的 `Sqllog` 字段运行过滤逻辑（parser 库提供物化后的字段（栈上数据））。
    /// 默认实现退化为 `process()`。
    fn process_with_meta(&self, record: &Sqllog) -> bool {
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
    pub fn run_with_meta(&self, record: &Sqllog) -> bool {
        self.processors.iter().all(|p| p.process_with_meta(record))
    }
}
