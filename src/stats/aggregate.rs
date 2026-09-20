//! 统计聚合层：慢 SQL 最小堆 + 高频 SQL `HashMap`，支持单次扫描双侧聚合。

use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashMap};

use crate::model::LogRecord;

/// 慢 SQL 输出行（终端展示）
#[derive(Debug, Clone)]
pub struct SlowSqlRow {
    pub sql_text: String,
    pub elapsed_ms: i64,
    pub timestamp: String,
}

/// 高频 SQL 输出行（终端展示）
#[derive(Debug, Clone)]
pub struct FrequentSqlRow {
    pub normalized_sql: String,
    pub call_count: u64,
    pub avg_elapsed_ms: i64,
    pub max_elapsed_ms: i64,
}

/// 慢 SQL 堆条目（内部使用，`f32` 保留精度，展示前用 `f32_ms_to_i64` 转换）
#[derive(Debug)]
struct SlowSqlEntry {
    sql_text: String,
    elapsed_ms: f32,
    timestamp: String,
}

impl PartialEq for SlowSqlEntry {
    fn eq(&self, other: &Self) -> bool {
        self.elapsed_ms.total_cmp(&other.elapsed_ms) == Ordering::Equal
    }
}

impl Eq for SlowSqlEntry {}

impl PartialOrd for SlowSqlEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SlowSqlEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.elapsed_ms.total_cmp(&other.elapsed_ms)
    }
}

/// 高频 SQL 聚合状态（`HashMap` value）
#[derive(Debug)]
struct AggState {
    call_count: u64,
    total_elapsed: f64,
    max_elapsed: f32,
}

/// 统计聚合器：单次扫描同时聚合慢 SQL（最小堆）与高频 SQL（`HashMap`）。
#[derive(Debug)]
pub struct StatsAccumulator {
    slow_heap: BinaryHeap<Reverse<SlowSqlEntry>>,
    freq_map: HashMap<String, AggState>,
    top_n: usize,
    from: Option<String>,
    to: Option<String>,
}

impl StatsAccumulator {
    /// 创建新的聚合器，`top_n` 为输出行数上限（≥ 1），`from`/`to` 为时间段过滤范围（均 None 时不过滤）。
    ///
    /// # Panics
    ///
    /// 当 `top_n` 为 0 时 panic。
    #[must_use]
    pub fn new(top_n: u32, from: Option<String>, to: Option<String>) -> Self {
        assert!(top_n >= 1, "top_n must be >= 1");
        Self {
            slow_heap: BinaryHeap::new(),
            freq_map: HashMap::new(),
            top_n: top_n as usize,
            from,
            to,
        }
    }

    /// 处理单条日志记录，同时更新慢 SQL 堆与高频 SQL 映射。
    ///
    /// 只处理 `[ORA]` 标签的记录（Oracle 兼容模式 SQL）；其他标签跳过。
    pub fn update(&mut self, record: &LogRecord) {
        if record.tag.as_deref() != Some("ORA") {
            return;
        }
        if !self.in_range(&record.ts) {
            return;
        }
        let slow_entry = SlowSqlEntry {
            sql_text: record.sql.clone(),
            elapsed_ms: record.exectime,
            timestamp: record.ts.clone(),
        };
        self.push_slow(slow_entry);

        let normalized_key = crate::stats::normalize::normalize_sql(&record.sql);
        let freq_state = self.freq_map.entry(normalized_key).or_insert(AggState {
            call_count: 0,
            total_elapsed: 0.0,
            max_elapsed: f32::NEG_INFINITY,
        });
        freq_state.call_count += 1;
        freq_state.total_elapsed += f64::from(record.exectime);
        if record.exectime > freq_state.max_elapsed {
            freq_state.max_elapsed = record.exectime;
        }
    }

    /// 检查 `ts` 是否在 `[from, to]` 时间范围内（字符串前缀截取比较，零分配）。
    fn in_range(&self, ts: &str) -> bool {
        if let Some(from) = &self.from {
            if ts.len() < from.len() {
                return false;
            }
            if &ts[..from.len()] < from.as_str() {
                return false;
            }
        }
        if let Some(to) = &self.to {
            if ts.len() < to.len() {
                return false;
            }
            if &ts[..to.len()] > to.as_str() {
                return false;
            }
        }
        true
    }

    /// 推入慢 SQL 条目到最小堆，维护堆大小 ≤ `top_n`。
    fn push_slow(&mut self, entry: SlowSqlEntry) {
        if self.slow_heap.len() < self.top_n {
            self.slow_heap.push(Reverse(entry));
            return;
        }
        if let Some(Reverse(heap_top)) = self.slow_heap.peek()
            && entry.elapsed_ms.total_cmp(&heap_top.elapsed_ms) == Ordering::Greater
        {
            self.slow_heap.pop();
            self.slow_heap.push(Reverse(entry));
        }
    }

    /// 消费聚合器，返回慢 SQL 行（按 elapsed 降序）与高频 SQL 行（按 `call_count` 降序）。
    #[must_use]
    pub fn into_results(self) -> (Vec<SlowSqlRow>, Vec<FrequentSqlRow>) {
        let slow_rows = build_slow_rows(self.slow_heap);
        let freq_rows = build_freq_rows(self.freq_map, self.top_n);
        (slow_rows, freq_rows)
    }
}

/// 从堆中提取并排序慢 SQL 行（降序）。
fn build_slow_rows(heap: BinaryHeap<Reverse<SlowSqlEntry>>) -> Vec<SlowSqlRow> {
    let mut rows: Vec<SlowSqlRow> = heap
        .into_iter()
        .map(|Reverse(entry)| SlowSqlRow {
            sql_text: entry.sql_text,
            elapsed_ms: crate::exporter::f32_ms_to_i64(entry.elapsed_ms),
            timestamp: entry.timestamp,
        })
        .collect();
    rows.sort_by_key(|row| std::cmp::Reverse(row.elapsed_ms));
    rows
}

/// 从 `HashMap` 构建高频 SQL 行，排序并截断到 `top_n`。
fn build_freq_rows(freq_map: HashMap<String, AggState>, top_n: usize) -> Vec<FrequentSqlRow> {
    let mut rows: Vec<FrequentSqlRow> = freq_map
        .into_iter()
        .map(|(normalized_sql, state)| {
            // f64 -> f32 精度在此为有意舍入（ms 级精度已足够），抑制 clippy 截断警告
            #[expect(
                clippy::cast_precision_loss,
                reason = "call_count as f64 may lose precision for large counts; acceptable for stats use"
            )]
            #[expect(
                clippy::cast_possible_truncation,
                reason = "avg in ms fits f32 range for typical SQL elapsed times"
            )]
            let avg_f32 = (state.total_elapsed / state.call_count as f64) as f32;
            FrequentSqlRow {
                normalized_sql,
                call_count: state.call_count,
                avg_elapsed_ms: crate::exporter::f32_ms_to_i64(avg_f32),
                max_elapsed_ms: crate::exporter::f32_ms_to_i64(state.max_elapsed),
            }
        })
        .collect();
    rows.sort_by_key(|row| std::cmp::Reverse(row.call_count));
    rows.truncate(top_n);
    rows
}

#[cfg(test)]
#[path = "../../tests/unit/stats/aggregate.rs"]
mod tests;
