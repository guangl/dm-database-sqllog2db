use crate::engine::run::{ProcessOutcome, RunContext};
use crate::error::{Error, ErrorStats, Result};
use crate::exporter::{CsvExporter, ExporterManager};
use crate::streaming::open_log_file;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// 单个已完成任务的产出：`(原文件路径, 临时 part 路径, 记录数)`。
type PartInfo = (PathBuf, PathBuf, usize);

/// 每个并行任务的返回值：`Some((orig_path, temp_path, count, file_stats))` 或 `None`（跳过/中断）。
type TaskResult = Option<(PathBuf, PathBuf, usize, ErrorStats)>;

/// 将 N 个已处理的临时 CSV 文件按顺序拼接到最终输出路径。
/// 第一个文件保留 header；后续文件跳过第一行。
/// `append_to_existing`=true 时所有文件都跳过 header（目标文件已有 header）。
fn concat_csv_parts(
    parts: &[(PathBuf, usize)],
    output_path: &Path,
    overwrite: bool,
    append_to_existing: bool,
) -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::BufReader;
    use std::io::Write as _;

    // 无任何 part 时不触碰输出文件，避免 overwrite=true 把已有数据清空。
    if parts.is_empty() {
        return Ok(());
    }

    let file = if append_to_existing {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(output_path)?
    } else {
        OpenOptions::new()
            .create_new(!overwrite)
            .create(overwrite)
            .write(true)
            .truncate(overwrite)
            .open(output_path)?
    };
    let mut writer = std::io::BufWriter::with_capacity(2 * 1024 * 1024, file);

    let mut parts_to_remove: Vec<&Path> = Vec::with_capacity(parts.len());
    for (idx, (part_path, _)) in parts.iter().enumerate() {
        let part_file = std::fs::File::open(part_path)?;
        let mut reader = BufReader::new(part_file);

        // 第一个 part（且非追加模式）保留 header；其余情况跳过 header 行
        let skip_header = idx > 0 || append_to_existing;
        if skip_header {
            // 用 Vec<u8> + read_until 而非 String + read_line：
            // 省去 UTF-8 验证，预分配避免 header 超 capacity 时的二次分配。
            let mut discard = Vec::with_capacity(256);
            std::io::BufRead::read_until(&mut reader, b'\n', &mut discard)?;
        }

        std::io::copy(&mut reader, &mut writer)?;
        // reader 在此处离开作用域并 drop，文件句柄关闭后才删除（Windows 兼容性）
        parts_to_remove.push(part_path.as_path());
    }

    // flush 结果延后传播：无论成功与否都先清理临时文件，避免 flush 失败时留下磁盘残留。
    // 显式 drop writer 确保文件句柄在 remove 前关闭（Windows 不允许删除已打开的文件）。
    let flush_result = writer.flush();
    drop(writer);
    for p in parts_to_remove {
        if let Err(e) = std::fs::remove_file(p) {
            log::warn!("failed to remove temp part {}: {e}", p.display());
        }
    }
    flush_result?;
    Ok(())
}

/// 准备临时 CSV parts 目录，与输出文件相邻（避免跨设备 copy）。
/// 若父目录不可写，退回到系统临时目录。
fn setup_parts_dir(output_path: &Path) -> Result<PathBuf> {
    let stem = output_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let dir_name = format!(".{stem}_parts_{}", std::process::id());
    let preferred = output_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let candidate = preferred.join(&dir_name);
    if std::fs::create_dir_all(&candidate).is_ok() {
        Ok(candidate)
    } else {
        let fallback = std::env::temp_dir().join(&dir_name);
        std::fs::create_dir_all(&fallback)?;
        Ok(fallback)
    }
}

fn parse_and_write_csv(
    ctx: &RunContext<'_>,
    file: &Path,
    temp_path: &Path,
    include_performance_metrics: bool,
    interrupted: &Arc<AtomicBool>,
) -> Result<(usize, ErrorStats)> {
    let mut exporter = CsvExporter::new(temp_path);
    exporter.normalize = ctx.do_normalize;
    exporter.field_mask = ctx.field_mask;
    exporter.ordered_indices.clone_from(&ctx.ordered_indices);
    exporter.include_performance_metrics = include_performance_metrics;
    let mut em = ExporterManager::from_csv(exporter);
    em.initialize()?;
    let include_pm = em.csv_include_performance_metrics();

    let records = match open_log_file(file) {
        Ok(it) => it,
        Err(e) => {
            log::warn!("parse failed for '{}': {e}", file.display());
            let mut file_stats = ErrorStats::default();
            file_stats.add_parse_error();
            return Ok((0, file_stats));
        }
    };

    let mut file_stats = ErrorStats::default();
    let count = crate::engine::record::iterate_records(
        records,
        &ctx.pipeline,
        ctx.do_normalize,
        ctx.placeholder_override,
        interrupted,
        &mut file_stats,
        |record, normalized| em.export_one_preparsed(record, include_pm, normalized),
    )?;

    em.finalize()?;
    Ok((count, file_stats))
}

// 日志文件通过 `LogIterator`（dm-database-parser-sqllog 流式 API）逐行读取并解析，
// 内存占用与文件大小无关，不再是 jobs × 单文件大小（旧实现一次性 `.parse()` 整文件到
// `Vec<Sqllog>` 才有此问题，见 `engine::prepare` 内存预算注释）。
//
// SAFETY: `tokio::task::block_in_place` 在 current_thread runtime 下会 panic（"Cannot call
// `blocking_` unless the thread is already in a thread pool"）。当前主入口使用
// `#[tokio::main]`（默认 multi_thread），但嵌入测试或 benchmark 若改用 current_thread
// runtime，此处将以不明显的 panic 失败。block_in_place 仅用于在执行 CPU 密集的 rayon
// 任务期间让出 tokio worker 线程，解析本身是纯同步调用，不再需要驱动任何 async 运行时。
fn run_parallel_tasks(
    ctx: &RunContext<'_>,
    log_files: &[PathBuf],
    csv_include_performance_metrics: bool,
    jobs: usize,
    parts_dir: &Path,
    verbose: bool,
    interrupted: &Arc<AtomicBool>,
) -> Result<Vec<Result<TaskResult>>> {
    use rayon::prelude::*;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build()
        .map_err(|e| Error::Io(std::io::Error::other(e)))?;
    let results: Vec<Result<TaskResult>> = tokio::task::block_in_place(|| {
        pool.install(|| {
            log_files
                .par_iter()
                .enumerate()
                .map(|(idx, file)| {
                    if interrupted.load(Ordering::Acquire) {
                        return Ok(None);
                    }
                    verbose.then(|| eprintln!("Processing: {}", file.display()));
                    let temp_path = parts_dir.join(format!("{idx:08}.csv"));
                    let (count, file_stats) = parse_and_write_csv(
                        ctx,
                        file,
                        &temp_path,
                        csv_include_performance_metrics,
                        interrupted,
                    )?;
                    Ok(Some((file.clone(), temp_path, count, file_stats)))
                })
                .collect()
        })
    });
    Ok(results)
}

/// 收集并行任务结果，分离成功项与错误，合并错误统计。
///
/// 若任何任务失败，清理已生成的临时 part 文件并返回首个错误（`parts_dir` 目录本身由调用方清理）。
fn collect_parallel_results(
    results: Vec<Result<TaskResult>>,
) -> Result<(Vec<PartInfo>, ErrorStats, usize)> {
    let mut parts_info: Vec<PartInfo> = Vec::with_capacity(results.len());
    let mut parallel_stats = ErrorStats::default();
    let mut first_err: Option<Error> = None;
    let mut skipped = 0usize;
    for result in results {
        match result {
            Ok(Some((orig, temp, count, file_stats))) => {
                parallel_stats.merge(&file_stats);
                parts_info.push((orig, temp, count));
            }
            Ok(None) => skipped += 1,
            Err(e) => {
                log::warn!("parallel collect error: {e}");
                if first_err.is_none() {
                    first_err = Some(e);
                }
            }
        }
    }
    if let Some(e) = first_err {
        for (_, temp, _) in &parts_info {
            let _ = std::fs::remove_file(temp);
        }
        return Err(e);
    }
    Ok((parts_info, parallel_stats, skipped))
}

/// 拼接并行生成的 CSV parts 到最终输出文件，并清理临时目录。
///
/// 返回 `(per_file_counts, skipped, parallel_stats)` 供 `engine::run` 消费。
fn finalize_concat(
    parts_info: Vec<PartInfo>,
    output_path: &Path,
    overwrite: bool,
    append_to_existing: bool,
    parts_dir: &Path,
    skipped: usize,
    parallel_stats: ErrorStats,
) -> Result<ProcessOutcome> {
    let parts_for_concat: Vec<(PathBuf, usize)> = parts_info
        .iter()
        .map(|(_, temp, count)| (temp.clone(), *count))
        .collect();
    let concat_result = concat_csv_parts(
        &parts_for_concat,
        output_path,
        overwrite,
        append_to_existing,
    );
    // 无论拼接成功与否都清理临时目录，避免磁盘满等错误导致残留
    let _ = std::fs::remove_dir_all(parts_dir);
    // 拼接失败且非追加模式时，删除已部分写入的输出文件，避免遗留截断的 CSV
    if concat_result.is_err() && !append_to_existing {
        let _ = std::fs::remove_file(output_path);
    }
    concat_result?;
    Ok((
        parts_info
            .into_iter()
            .map(|(orig, _, count)| (orig, count))
            .collect(),
        skipped,
        parallel_stats,
    ))
}

/// 并行 CSV 处理：每个文件独立跑在 rayon 线程上，各写一个临时 CSV，
/// 最终按文件原始顺序拼接成一个完整 CSV。
///
/// 返回：`(已处理文件列表, 跳过文件数, 解析错误统计)`，已处理列表顺序与 `log_files` 一致。
/// 适用条件：CSV 导出 + 多文件 + jobs > 1 + 无 limit。
/// 注意：每个 rayon 任务开始时若 verbose=true 输出 "Processing: {path}"（D-02）。
pub(crate) fn process_csv_parallel(
    ctx: &RunContext<'_>,
    log_files: &[PathBuf],
    jobs: usize,
    verbose: bool,
    interrupted: &Arc<AtomicBool>,
) -> Result<ProcessOutcome> {
    let csv_cfg = ctx.cfg.exporter.csv.as_ref().ok_or_else(|| {
        Error::Export(crate::error::ExportError::WriteFailed {
            path: std::path::PathBuf::from("<csv>"),
            reason: "parallel CSV path requires CSV exporter to be configured".into(),
        })
    })?;
    let output_path = Path::new(&csv_cfg.file);
    let append_to_existing = csv_cfg.append && output_path.exists();
    let parts_dir = setup_parts_dir(output_path)?;
    let results = run_parallel_tasks(
        ctx,
        log_files,
        csv_cfg.include_performance_metrics,
        jobs,
        &parts_dir,
        verbose,
        interrupted,
    )?;
    let (parts_info, parallel_stats, skipped) = match collect_parallel_results(results) {
        Ok(v) => v,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&parts_dir);
            return Err(e);
        }
    };
    finalize_concat(
        parts_info,
        output_path,
        csv_cfg.overwrite,
        append_to_existing,
        &parts_dir,
        skipped,
        parallel_stats,
    )
}
