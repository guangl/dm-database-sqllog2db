//! 多文件并行场景下的内存预算控制（保守兜底）。
//!
//! 历史背景：旧实现每个 rayon 任务通过 `AsyncLogParser` 把整份文件一次性读入内存解析成
//! `Vec<Sqllog>`，峰值内存约为 `jobs × 单文件大小`，`jobs` 越大、文件越大越容易 OOM，本模块
//! 因此存在。自切换到 [`crate::streaming::open_log_file`] 流式迭代后，单文件解析峰值内存已与
//! 文件大小基本无关（逐行读取），本模块的限流大多数情况下不再必要，仅作为防御性上限保留。
//!
//! 策略：把并行解析阶段的峰值内存控制在 [`DEFAULT_MEMORY_BUDGET_BYTES`]（2GB）以内：取参与
//! 并行的文件中最大的一个作为单任务内存占用的保守估计（乘以 [`PARSE_MEMORY_AMPLIFICATION`]
//! 放大系数，覆盖解析后 `Sqllog` 结构体中每个字段额外的 `String` 分配开销），用
//! `budget / 单任务估计内存` 反推允许的最大并发数，并与用户请求的 `jobs` 取较小值。
//!
//! 注意：该估算沿用旧的"整文件驻留"内存模型，对流式解析而言偏保守——超大文件可能被降并发到
//! 远低于实际所需的水平。若发现大文件并行度被过度压低，应调整放大系数/预算或改用与流式一致的
//! 固定 per-file 估算。
//!
//! 不处理的场景：单个文件本身的预估内存就超过预算（即使并发数降到 1 也无法满足）；此时退回到
//! `jobs = 1`，仅做警告，不阻止运行（流式解析下这类情况实际内存占用远低于估算）。

use std::path::Path;

/// 并行解析阶段的默认内存预算：2GB。
pub(super) const DEFAULT_MEMORY_BUDGET_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// 解析后内存相对原始文本的放大系数（保守估计）：
/// `Sqllog` 的每个字段都是独立的 `String`/`Vec` 分配，加上 `Vec<Sqllog>` 本身的容量开销，
/// 实测文本到结构体的膨胀通常在 2~3 倍之间；取 3 作为安全上界。
const PARSE_MEMORY_AMPLIFICATION: u64 = 3;

/// 根据参与并行的文件大小，把用户请求的 `jobs` 降低到内存预算允许的范围内。
///
/// 返回值始终 `>= 1`（即使单文件预估内存已超过预算，也保留至少 1 个并发，仅靠调用方记录警告）。
pub(super) fn effective_jobs_for_memory_budget(
    files: &[std::path::PathBuf],
    requested_jobs: usize,
    budget_bytes: u64,
) -> usize {
    if requested_jobs <= 1 {
        return requested_jobs.max(1);
    }
    let max_size = files.iter().filter_map(|f| file_size(f)).max().unwrap_or(0);
    if max_size == 0 {
        return requested_jobs;
    }
    let per_task_bytes = max_size.saturating_mul(PARSE_MEMORY_AMPLIFICATION).max(1);
    let budget_jobs = usize::try_from((budget_bytes / per_task_bytes).max(1)).unwrap_or(usize::MAX);
    requested_jobs.min(budget_jobs).max(1)
}

fn file_size(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|m| m.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn write_temp_file(dir: &std::path::Path, name: &str, size: usize) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, vec![b'a'; size]).unwrap();
        path
    }

    #[test]
    fn test_requested_jobs_of_one_is_unaffected() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_temp_file(dir.path(), "a.log", 10 * 1024 * 1024);
        assert_eq!(
            effective_jobs_for_memory_budget(&[f], 1, 2 * 1024 * 1024 * 1024),
            1
        );
    }

    #[test]
    fn test_small_files_keep_requested_jobs() {
        let dir = tempfile::tempdir().unwrap();
        let files: Vec<_> = (0..8)
            .map(|i| write_temp_file(dir.path(), &format!("f{i}.log"), 1024 * 1024))
            .collect();
        // 1MB files * 3x amplification = 3MB per task; well within a 2GB budget at 8 jobs.
        assert_eq!(
            effective_jobs_for_memory_budget(&files, 8, 2 * 1024 * 1024 * 1024),
            8
        );
    }

    #[test]
    fn test_large_files_cap_jobs_below_requested() {
        let dir = tempfile::tempdir().unwrap();
        // Each file is 500MB; amplified to 1.5GB per task. A 2GB budget allows only 1 concurrent task.
        let files: Vec<_> = (0..8)
            .map(|i| write_temp_file(dir.path(), &format!("big{i}.log"), 500 * 1024 * 1024))
            .collect();
        let jobs = effective_jobs_for_memory_budget(&files, 8, 2 * 1024 * 1024 * 1024);
        assert_eq!(jobs, 1);
    }

    #[test]
    fn test_single_huge_file_falls_back_to_one_job_not_zero() {
        let dir = tempfile::tempdir().unwrap();
        // 4GB nominal size (amplified to 12GB) vastly exceeds a 2GB budget; must not return 0.
        let f = write_temp_file(dir.path(), "huge.log", 4096); // small actual file, fake huge via metadata is not possible portably
        let jobs = effective_jobs_for_memory_budget(&[f], 8, 1); // budget of 1 byte forces the floor
        assert_eq!(jobs, 1, "must always return at least 1 job");
    }

    #[test]
    fn test_missing_files_are_ignored_and_default_to_requested_jobs() {
        let missing = PathBuf::from("/nonexistent/path/should/not/exist.log");
        assert_eq!(
            effective_jobs_for_memory_budget(&[missing], 8, 2 * 1024 * 1024 * 1024),
            8
        );
    }
}
